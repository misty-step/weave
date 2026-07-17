use chrono::{DateTime, TimeZone, Utc};
use serde::Serialize;
use serde_json::Value;

use crate::window::RetroWindow;

/// One dated thing that happened to a Powder card: created, claimed,
/// completed, status-changed, or commented. Sourced from `card.events` in
/// the `get_card` response (unix-second `created_at`), which is Powder's
/// own audit trail -- not re-derived from card status alone, so a card that
/// moved through several states in the window shows each move, not just
/// its current status.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CardMovement {
    pub card_id: String,
    pub repo: String,
    pub event_type: String,
    pub actor: String,
    pub at: String,
    pub summary: String,
    pub source: String,
}

/// Read-only Powder HTTP client. `POWDER_API_BASE_URL`/`POWDER_API_KEY` are
/// read via env only -- never logged, never embedded in generated output.
/// Returns `None` from `from_env` (not an error) when unconfigured, so a
/// retro run without Powder wiring reports "Powder: not configured" as a
/// named source gap instead of failing the whole generator.
pub struct PowderClient {
    base_url: String,
    api_key: String,
}

impl PowderClient {
    /// Reads explicit, value-free LaunchAgent configuration only. The API key
    /// is a Mint placeholder, never the real Powder credential.
    pub fn from_env() -> Option<Self> {
        let base_url = std::env::var("POWDER_API_BASE_URL").ok()?;
        let api_key = std::env::var("POWDER_API_KEY").ok()?;
        if base_url.trim().is_empty() || api_key.trim().is_empty() {
            return None;
        }
        Some(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
        })
    }

    fn get(&self, path: &str) -> anyhow::Result<Value> {
        let url = format!("{}{}", self.base_url, path);
        let response = ureq::get(&url)
            .set("Authorization", &format!("Bearer {}", self.api_key))
            .call()?;
        Ok(response.into_json()?)
    }

    pub fn list_card_ids(&self, limit: u32) -> anyhow::Result<Vec<String>> {
        let value = self.get(&format!("/api/v1/cards?limit={limit}"))?;
        Ok(value
            .get("cards")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|card| card.get("id").and_then(Value::as_str))
            .map(str::to_string)
            .collect())
    }

    pub fn get_card(&self, id: &str) -> anyhow::Result<Value> {
        self.get(&format!("/api/v1/cards/{id}"))
    }
}

fn unix_seconds_to_utc(seconds: i64) -> Option<DateTime<Utc>> {
    Utc.timestamp_opt(seconds, 0).single()
}

/// Derive a repo name from Powder's `<repo>-<number>` card-id convention
/// (`landmark-907`, `canary-911`) when the card's own `repo` field is empty.
/// Falls back to `unknown` only when the id itself doesn't follow that
/// shape -- this is strictly an accuracy improvement over always reporting
/// `unknown`, never a guess dressed up as certainty (the card row simply
/// carries whatever the id implies).
fn repo_from_card_id(card_id: &str) -> String {
    match card_id.rsplit_once('-') {
        Some((prefix, suffix))
            if !prefix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()) =>
        {
            prefix.to_string()
        }
        _ => "unknown".to_string(),
    }
}

/// Fixed character budget for a rendered comment excerpt -- one number, used
/// consistently for every comment, so the timeline and repo-activity rows
/// this feeds never show two different cut lengths side by side.
const COMMENT_EXCERPT_BUDGET: usize = 160;

fn summarize(event_type: &str, payload: &str) -> String {
    match event_type {
        "create" => "created".to_string(),
        "claim" => format!("claimed ({payload})"),
        "release_claim" => "claim released".to_string(),
        "status_change" => format!("status -> {payload}"),
        "complete" => "completed".to_string(),
        "comment" => {
            // Strip Markdown and collapse paragraph breaks *before*
            // truncating on a word boundary -- truncating first (the prior
            // behavior: a raw char slice) could cut inside a `**bold**`
            // marker or land mid-word, which is exactly the bug a live
            // designer critique caught in a rendered report.
            let cleaned = crate::text::plain_text(payload.trim());
            format!(
                "commented: {}",
                crate::text::truncate_words(&cleaned, COMMENT_EXCERPT_BUDGET)
            )
        }
        other => format!("{other}: {payload}"),
    }
}

/// Pure extraction over an already-fetched `get_card` response: pull every
/// event and comment whose timestamp falls in `window`. Kept separate from
/// the HTTP fetch so it can be unit-tested against a hand-built fixture
/// without a live Powder instance.
pub fn extract_movements_from_card_detail(
    detail: &Value,
    window: &RetroWindow,
) -> Vec<CardMovement> {
    let card_id = detail
        .get("card")
        .and_then(|c| c.get("id"))
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let repo = detail
        .get("card")
        .and_then(|c| c.get("repo"))
        .and_then(Value::as_str)
        .filter(|r| !r.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| repo_from_card_id(&card_id));
    let source = format!("powder:card:{card_id}");

    let mut movements = Vec::new();

    for event in detail
        .get("events")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(created_at) = event.get("created_at").and_then(Value::as_i64) else {
            continue;
        };
        let Some(at) = unix_seconds_to_utc(created_at) else {
            continue;
        };
        if !window.contains(at) {
            continue;
        }
        let event_type = event
            .get("event_type")
            .and_then(Value::as_str)
            .unwrap_or("event");
        let payload = event.get("payload").and_then(Value::as_str).unwrap_or("");
        let actor = event
            .get("actor")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        movements.push(CardMovement {
            card_id: card_id.clone(),
            repo: repo.clone(),
            event_type: event_type.to_string(),
            actor: actor.to_string(),
            at: at.to_rfc3339(),
            summary: summarize(event_type, payload),
            source: source.clone(),
        });
    }

    for comment in detail
        .get("comments")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(created_at) = comment.get("created_at").and_then(Value::as_i64) else {
            continue;
        };
        let Some(at) = unix_seconds_to_utc(created_at) else {
            continue;
        };
        if !window.contains(at) {
            continue;
        }
        let author = comment
            .get("author")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let body = comment.get("body").and_then(Value::as_str).unwrap_or("");
        movements.push(CardMovement {
            card_id: card_id.clone(),
            repo: repo.clone(),
            event_type: "comment".to_string(),
            actor: author.to_string(),
            at: at.to_rfc3339(),
            summary: summarize("comment", body),
            source: source.clone(),
        });
    }

    movements.sort_by(|a, b| a.at.cmp(&b.at));
    movements
}

/// Live orchestration: list cards, fetch each one's detail, extract
/// in-window movements. A single card's fetch failure is logged to stderr
/// and skipped rather than failing the whole collection -- one dead card
/// should not blank out every other repo's Powder activity.
pub fn collect_card_movements(
    client: &PowderClient,
    window: &RetroWindow,
    card_limit: u32,
) -> Vec<CardMovement> {
    let mut movements = Vec::new();
    let ids = match client.list_card_ids(card_limit) {
        Ok(ids) => ids,
        Err(err) => {
            eprintln!("fleet-retro: powder list_cards failed: {err}");
            return movements;
        }
    };
    for id in ids {
        match client.get_card(&id) {
            Ok(detail) => movements.extend(extract_movements_from_card_detail(&detail, window)),
            Err(err) => eprintln!("fleet-retro: powder get_card({id}) failed: {err}"),
        }
    }
    movements
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::git::ts;
    use serde_json::json;

    #[test]
    fn falls_back_to_card_id_prefix_when_repo_field_is_missing() {
        let detail = json!({
            "card": {"id": "canary-911"},
            "events": [
                {"actor": "bridge", "card_id": "canary-911", "created_at": 1783225265, "event_type": "status_change", "payload": "done"},
            ],
            "comments": [],
        });
        let window = RetroWindow::custom(ts(2026, 7, 4, 0, 0, 0), ts(2026, 7, 6, 0, 0, 0)).unwrap();

        let movements = extract_movements_from_card_detail(&detail, &window);

        assert_eq!(movements[0].repo, "canary");
    }

    #[test]
    fn extracts_events_and_comments_within_window() {
        let detail = json!({
            "card": {"id": "landmark-907", "repo": "landmark"},
            "events": [
                {"actor": "operator-admin", "card_id": "landmark-907", "created_at": 1783208032, "event_type": "create", "payload": "created card"},
            ],
            "comments": [
                {"author": "lane-landmark-907", "created_at": 1783225265, "body": "Fixed with a structural grounding gate."},
            ],
        });
        // 1783208032 -> 2026-07-04T21:33:52Z; 1783225265 -> 2026-07-05T02:21:05Z
        let window = RetroWindow::custom(ts(2026, 7, 4, 0, 0, 0), ts(2026, 7, 6, 0, 0, 0)).unwrap();

        let movements = extract_movements_from_card_detail(&detail, &window);

        assert_eq!(movements.len(), 2);
        assert_eq!(movements[0].event_type, "create");
        assert_eq!(movements[1].event_type, "comment");
        assert_eq!(movements[1].repo, "landmark");
    }

    #[test]
    fn excludes_events_outside_window() {
        let detail = json!({
            "card": {"id": "old-card", "repo": "landmark"},
            "events": [
                {"actor": "x", "card_id": "old-card", "created_at": 1_600_000_000, "event_type": "create", "payload": "created card"},
            ],
            "comments": [],
        });
        let window = RetroWindow::custom(ts(2026, 7, 4, 0, 0, 0), ts(2026, 7, 6, 0, 0, 0)).unwrap();

        let movements = extract_movements_from_card_detail(&detail, &window);

        assert!(movements.is_empty());
    }

    #[test]
    fn truncates_long_comments_containing_multibyte_characters_without_panicking() {
        // Regression: a naive byte-index slice (&s[..120]) panics whenever
        // the cut point lands inside a multi-byte UTF-8 character. Real
        // fleet comments routinely contain em dashes ('—', 3 bytes) right
        // around that length -- this reproduced live against tonight's
        // actual Powder data on the first real run.
        let long_comment = format!(
            "{}— structural grounding gate — retried fallback models on fabrication",
            "x".repeat(115)
        );
        let detail = json!({
            "card": {"id": "landmark-907", "repo": "landmark"},
            "events": [],
            "comments": [
                {"author": "lane-landmark-907", "created_at": 1783225265, "body": long_comment},
            ],
        });
        let window = RetroWindow::custom(ts(2026, 7, 4, 0, 0, 0), ts(2026, 7, 6, 0, 0, 0)).unwrap();

        let movements = extract_movements_from_card_detail(&detail, &window);

        assert_eq!(movements.len(), 1);
        assert!(movements[0].summary.starts_with("commented: "));
        assert!(movements[0].summary.ends_with('…'));
    }

    #[test]
    fn comment_strips_markdown_bold_and_paragraph_breaks_before_truncating() {
        // Regression: the literal bug a live designer critique caught --
        // "SHAPED WITH OPERATOR ... **Delivery = two-phas…" -- a raw `\n\n`
        // and an unrendered `**` marker landed in the rendered summary, with
        // the truncation cut falling mid-word inside the marker.
        let comment = "SHAPED WITH OPERATOR 2026-07-07 morning — ratified design.\n\n**Delivery = two-phased**: ship first, then harden.";
        let summary = summarize("comment", comment);

        assert!(
            !summary.contains("**"),
            "markdown bold markers must not leak into the rendered summary: {summary}"
        );
        assert!(
            !summary.contains('\n'),
            "paragraph breaks must not render as literal newlines: {summary}"
        );
        assert!(summary.starts_with("commented: "));
    }

    // Configuration is supplied explicitly by the scheduled job. Live
    // cutover proof covers the value-free Mint endpoint and placeholder.
}
