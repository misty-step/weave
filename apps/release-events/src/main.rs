use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use clap::Parser;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Sha256;
use subtle::ConstantTimeEq;
use tokio::sync::Mutex;
use tower_http::limit::RequestBodyLimitLayer;
use tracing::{error, info};

const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;
const SIGNATURE_HEADER: &str = "x-signature-256";

#[derive(Debug, Parser, Clone)]
struct Config {
    #[arg(long, env = "RELEASE_EVENTS_ADDR", default_value = "0.0.0.0:8080")]
    addr: String,
    #[arg(long, env = "RELEASE_EVENTS_ROOT", default_value = "/data/events")]
    root: PathBuf,
    #[arg(long, env = "LANDMARK_WEBHOOK_SECRET", default_value = "")]
    webhook_secret: String,
    #[arg(long, env = "RELEASE_EVENTS_READER_TOKEN", default_value = "")]
    reader_token: String,
}

#[derive(Clone)]
struct AppState {
    log_path: Arc<PathBuf>,
    webhook_secret: Arc<String>,
    reader_token: Arc<String>,
    write_lock: Arc<Mutex<()>>,
}

#[derive(Debug, Deserialize)]
struct EventsQuery {
    since: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct StoredEvent {
    received_at: DateTime<Utc>,
    kind: EventKind,
    repository: String,
    version: String,
    release_url: String,
    payload: Value,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum EventKind {
    LandmarkWebhook,
    LandmarkReleaseKit,
}

#[derive(Debug, Serialize)]
struct EventsResponse {
    events: Vec<StoredEvent>,
}

#[derive(Debug)]
struct AppError {
    status: StatusCode,
    body: &'static str,
}

impl AppError {
    fn bad_request(body: &'static str) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            body,
        }
    }

    fn unauthorized(body: &'static str) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            body,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (self.status, self.body).into_response()
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        error!(error = %err, "release event receiver internal error");
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            body: "internal error\n",
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "weave_release_events=info,tower_http=warn".into()),
        )
        .init();

    let cfg = Config::parse();
    if cfg.webhook_secret.is_empty() {
        bail!("LANDMARK_WEBHOOK_SECRET must be set");
    }
    if cfg.reader_token.is_empty() {
        bail!("RELEASE_EVENTS_READER_TOKEN must be set");
    }
    tokio::fs::create_dir_all(&cfg.root)
        .await
        .with_context(|| format!("creating {}", cfg.root.display()))?;
    let app = router(&cfg);
    let listener = tokio::net::TcpListener::bind(&cfg.addr)
        .await
        .with_context(|| format!("binding {}", cfg.addr))?;
    info!(addr = %cfg.addr, root = %cfg.root.display(), "release event receiver listening");
    axum::serve(listener, app).await?;
    Ok(())
}

fn router(cfg: &Config) -> Router {
    let state = AppState {
        log_path: Arc::new(cfg.root.join("events.jsonl")),
        webhook_secret: Arc::new(cfg.webhook_secret.clone()),
        reader_token: Arc::new(cfg.reader_token.clone()),
        write_lock: Arc::new(Mutex::new(())),
    };
    Router::new()
        .route("/healthz", get(|| async { "ok\n" }))
        .route("/v1/events", post(post_event).get(get_events))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(RequestBodyLimitLayer::new(MAX_BODY_BYTES))
        .with_state(state)
}

async fn post_event(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, AppError> {
    if !signature_ok(&headers, &state.webhook_secret, &body) {
        return Err(AppError::unauthorized("bad or missing signature\n"));
    }
    let payload: Value =
        serde_json::from_slice(&body).map_err(|_| AppError::bad_request("invalid json\n"))?;
    let event = classify_event(payload).map_err(AppError::bad_request)?;
    append_event(&state, &event).await?;
    Ok((StatusCode::CREATED, Json(event)).into_response())
}

async fn get_events(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<EventsQuery>,
) -> Result<Response, AppError> {
    if !bearer_ok(&headers, &state.reader_token) {
        return Err(AppError::unauthorized("bad or missing bearer token\n"));
    }
    let since = match query.since.as_deref() {
        Some(raw) => Some(
            DateTime::parse_from_rfc3339(raw)
                .map_err(|_| AppError::bad_request("invalid since timestamp\n"))?
                .with_timezone(&Utc),
        ),
        None => None,
    };
    let events = read_events(&state.log_path, since).await?;
    Ok(Json(EventsResponse { events }).into_response())
}

fn classify_event(payload: Value) -> Result<StoredEvent, &'static str> {
    if let Some(summary) = plain_landmark_summary(&payload) {
        return Ok(StoredEvent {
            received_at: Utc::now(),
            kind: EventKind::LandmarkWebhook,
            repository: summary.repository,
            version: summary.version,
            release_url: summary.release_url,
            payload,
        });
    }
    if let Some(summary) = release_kit_summary(&payload) {
        return Ok(StoredEvent {
            received_at: Utc::now(),
            kind: EventKind::LandmarkReleaseKit,
            repository: summary.repository,
            version: summary.version,
            release_url: summary.release_url,
            payload,
        });
    }
    Err("unsupported event shape\n")
}

struct EventSummary {
    repository: String,
    version: String,
    release_url: String,
}

fn plain_landmark_summary(payload: &Value) -> Option<EventSummary> {
    let version = nonempty(payload.get("version")?)?;
    let release_url = nonempty(payload.get("release_url")?)?;
    nonempty(payload.get("notes")?)?;
    let repository = nonempty(payload.get("repository")?)?;
    Some(EventSummary {
        repository,
        version,
        release_url,
    })
}

fn release_kit_summary(payload: &Value) -> Option<EventSummary> {
    let schema_or_kind_matches = payload.get("schema_version").and_then(Value::as_str)
        == Some("landmark.release-kit.v1")
        || payload.get("kind").and_then(Value::as_str) == Some("landmark.release-kit");
    let looks_like_kit = payload.get("release").is_some_and(Value::is_object)
        && payload
            .get("producer_contracts")
            .is_some_and(Value::is_array);
    if !schema_or_kind_matches && !looks_like_kit {
        return None;
    }
    let release = payload.get("release")?;
    let version = nonempty(release.get("tag").or_else(|| release.get("version"))?)?;
    let release_url = nonempty(release.get("release_url")?)?;
    let repository = release
        .get("repository")
        .and_then(nonempty)
        .or_else(|| payload.pointer("/product/repository").and_then(nonempty))?;
    Some(EventSummary {
        repository,
        version,
        release_url,
    })
}

fn nonempty(value: &Value) -> Option<String> {
    let trimmed = value.as_str()?.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

async fn append_event(state: &AppState, event: &StoredEvent) -> Result<()> {
    if let Some(parent) = state.log_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let line = serde_json::to_string(event)? + "\n";
    let _guard = state.write_lock.lock().await;
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(state.log_path.as_ref())
        .await
        .with_context(|| format!("opening {}", state.log_path.display()))?;
    use tokio::io::AsyncWriteExt;
    file.write_all(line.as_bytes()).await?;
    file.flush().await?;
    Ok(())
}

async fn read_events(path: &Path, since: Option<DateTime<Utc>>) -> Result<Vec<StoredEvent>> {
    let contents = match tokio::fs::read_to_string(path).await {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err).with_context(|| format!("reading {}", path.display())),
    };
    let mut events = Vec::new();
    for (idx, line) in contents.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let event: StoredEvent = serde_json::from_str(line)
            .with_context(|| format!("parsing {} line {}", path.display(), idx + 1))?;
        if since.is_none_or(|ts| event.received_at > ts) {
            events.push(event);
        }
    }
    Ok(events)
}

fn signature_ok(headers: &HeaderMap, secret: &str, body: &[u8]) -> bool {
    let Some(presented) = headers.get(SIGNATURE_HEADER).and_then(|v| v.to_str().ok()) else {
        return false;
    };
    let Ok(expected) = compute_signature(secret, body) else {
        return false;
    };
    presented.as_bytes().ct_eq(expected.as_bytes()).into()
}

fn compute_signature(secret: &str, body: &[u8]) -> Result<String> {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())?;
    mac.update(body);
    Ok(format!(
        "sha256={}",
        hex::encode(mac.finalize().into_bytes())
    ))
}

fn bearer_ok(headers: &HeaderMap, token: &str) -> bool {
    let Some(value) = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    else {
        return false;
    };
    let Some(presented) = value.strip_prefix("Bearer ") else {
        return false;
    };
    presented.as_bytes().ct_eq(token.as_bytes()).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use serde_json::json;
    use tower::ServiceExt;

    fn test_cfg(root: &Path) -> Config {
        Config {
            addr: "127.0.0.1:0".into(),
            root: root.to_path_buf(),
            webhook_secret: "hook-secret".into(),
            reader_token: "reader-secret".into(),
        }
    }

    fn signed_post(path: &str, body: &'static str, secret: &str) -> Request<Body> {
        Request::post(path)
            .header("content-type", "application/json")
            .header(
                SIGNATURE_HEADER,
                compute_signature(secret, body.as_bytes()).unwrap(),
            )
            .body(Body::from(body))
            .unwrap()
    }

    async fn response_body(resp: Response) -> String {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        String::from_utf8_lossy(&bytes).into_owned()
    }

    #[tokio::test]
    async fn valid_signature_creates_and_persists_plain_landmark_payload() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = test_cfg(dir.path());
        let body = r###"{"version":"v1.2.3","release_url":"https://github.com/misty-step/landmark/releases/tag/v1.2.3","notes":"## What's New\n- shipped","repository":"misty-step/landmark"}"###;

        let resp = router(&cfg)
            .oneshot(signed_post("/v1/events", body, &cfg.webhook_secret))
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::CREATED);
        let stored = std::fs::read_to_string(dir.path().join("events.jsonl")).unwrap();
        let lines: Vec<_> = stored.lines().collect();
        assert_eq!(lines.len(), 1);
        let event: StoredEvent = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(event.kind, EventKind::LandmarkWebhook);
        assert_eq!(event.version, "v1.2.3");
        assert_eq!(event.repository, "misty-step/landmark");
    }

    #[tokio::test]
    async fn bad_or_missing_signature_is_unauthorized_and_not_persisted() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = test_cfg(dir.path());
        let body = r#"{"version":"v1.2.3","release_url":"https://example.test/release","notes":"notes","repository":"misty-step/landmark"}"#;

        let missing = router(&cfg)
            .oneshot(
                Request::post("/v1/events")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(missing.status(), StatusCode::UNAUTHORIZED);

        let bad = router(&cfg)
            .oneshot(
                Request::post("/v1/events")
                    .header("content-type", "application/json")
                    .header(SIGNATURE_HEADER, "sha256=bad")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(bad.status(), StatusCode::UNAUTHORIZED);
        assert!(!dir.path().join("events.jsonl").exists());
    }

    #[tokio::test]
    async fn get_requires_reader_bearer_token() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = test_cfg(dir.path());
        let body = r#"{"version":"v1.2.3","release_url":"https://example.test/release","notes":"notes","repository":"misty-step/landmark"}"#;
        let post = router(&cfg)
            .oneshot(signed_post("/v1/events", body, &cfg.webhook_secret))
            .await
            .unwrap();
        assert_eq!(post.status(), StatusCode::CREATED);

        for auth in [None, Some("Bearer wrong")] {
            let mut req = Request::get("/v1/events");
            if let Some(auth) = auth {
                req = req.header(header::AUTHORIZATION, auth);
            }
            let resp = router(&cfg)
                .oneshot(req.body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        }

        let resp = router(&cfg)
            .oneshot(
                Request::get("/v1/events")
                    .header(header::AUTHORIZATION, "Bearer reader-secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body: Value = serde_json::from_str(&response_body(resp).await).unwrap();
        assert_eq!(body["events"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn since_filter_returns_events_after_the_timestamp() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = test_cfg(dir.path());
        std::fs::create_dir_all(dir.path()).unwrap();
        let old = json!({
            "received_at": "2026-07-03T10:00:00Z",
            "kind": "landmark_webhook",
            "repository": "misty-step/old",
            "version": "v0.1.0",
            "release_url": "https://example.test/old",
            "payload": {"version": "v0.1.0"}
        });
        let new = json!({
            "received_at": "2026-07-03T11:00:00Z",
            "kind": "landmark_webhook",
            "repository": "misty-step/new",
            "version": "v0.2.0",
            "release_url": "https://example.test/new",
            "payload": {"version": "v0.2.0"}
        });
        std::fs::write(
            dir.path().join("events.jsonl"),
            format!("{}\n{}\n", old, new),
        )
        .unwrap();

        let resp = router(&cfg)
            .oneshot(
                Request::get("/v1/events?since=2026-07-03T10:30:00Z")
                    .header(header::AUTHORIZATION, "Bearer reader-secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body: Value = serde_json::from_str(&response_body(resp).await).unwrap();
        let events = body["events"].as_array().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["repository"], "misty-step/new");
    }

    #[tokio::test]
    async fn valid_signature_accepts_release_kit_shape() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = test_cfg(dir.path());
        let body = r#"{"schema_version":"landmark.release-kit.v1","generated_at":"2026-07-03T11:00:00Z","product":{"name":"Landmark","repository":"misty-step/landmark"},"release":{"tag":"v1.2.3","version":"1.2.3","repository":"misty-step/landmark","release_url":"https://github.com/misty-step/landmark/releases/tag/v1.2.3"},"classification":{"importance":"minor","audiences":["developer"],"why_it_matters":"test"},"artifacts":[{"id":"release-notes","kind":"release_notes","audience":"developer","owner":"landmark","status":"written","acceptance":["ok"]}],"producer_contracts":[],"provenance":[{"artifact_id":"release-notes","sources":["test"]}],"approvals":[{"artifact_id":"release-notes","state":"approved"}],"status":{"complete":true,"blocked":false,"summary":"ok"}}"#;

        let resp = router(&cfg)
            .oneshot(signed_post("/v1/events", body, &cfg.webhook_secret))
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::CREATED);
        let body: Value = serde_json::from_str(&response_body(resp).await).unwrap();
        assert_eq!(body["kind"], "landmark_release_kit");
        assert_eq!(body["repository"], "misty-step/landmark");
    }
}
