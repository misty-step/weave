use std::process::Command;

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;

use crate::window::RetroWindow;

/// One Bitterblossom plane run whose `created_at` falls in the window.
/// Mirrors the fields of bitterblossom's own `RunRow` (see
/// `bitterblossom/src/ledger.rs`) that are useful in a retro: which task,
/// which agent, terminal state, and cost. Field access is defensive
/// (`Option`-based) because this crate does not depend on bitterblossom's
/// types directly -- it consumes `bb runs list --json` as an external
/// contract, not a shared struct, so a bb-side field rename degrades this
/// collector instead of breaking the build.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct BbRun {
    pub id: String,
    pub task: String,
    pub agent: String,
    pub state: String,
    pub created_at: String,
    pub source: String,
}

/// Shell out to `bb --config <plane> runs list --json` and return the runs
/// whose `created_at` falls in `window`. `plane` is a directory containing
/// `plane.toml`; when `None`, this returns an empty list with a stderr note
/// rather than guessing at a default -- there is no single fleet-wide plane
/// today, and a wrong guess (silently reading an unrelated test fixture
/// plane) would be worse than an honest "not configured."
pub fn collect_bb_runs(plane: Option<&str>, window: &RetroWindow) -> Vec<BbRun> {
    let Some(plane) = plane else {
        eprintln!("fleet-retro: no --bb-plane configured; skipping bb plane runs");
        return Vec::new();
    };
    let output = match Command::new("bb")
        .args(["--config", plane, "runs", "list", "--json"])
        .output()
    {
        Ok(output) => output,
        Err(err) => {
            eprintln!("fleet-retro: bb runs list failed to execute: {err}");
            return Vec::new();
        }
    };
    if !output.status.success() {
        eprintln!(
            "fleet-retro: bb runs list exited nonzero: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return Vec::new();
    }
    let Ok(value) = serde_json::from_slice::<Value>(&output.stdout) else {
        eprintln!("fleet-retro: bb runs list did not return valid JSON");
        return Vec::new();
    };
    extract_runs_in_window(&value, plane, window)
}

fn parse_flexible_ts(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .ok()
        .or_else(|| {
            chrono::NaiveDateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S")
                .ok()
                .map(|naive| naive.and_utc())
        })
}

/// Pure extraction over an already-parsed `bb runs list --json` payload.
/// Accepts either a bare array or `{"runs": [...]}` since bb's CLI surfaces
/// have used both shapes across commands.
pub fn extract_runs_in_window(value: &Value, plane: &str, window: &RetroWindow) -> Vec<BbRun> {
    let items: Vec<&Value> = match value {
        Value::Array(items) => items.iter().collect(),
        Value::Object(_) => value
            .get("runs")
            .and_then(Value::as_array)
            .map(|items| items.iter().collect())
            .unwrap_or_default(),
        _ => Vec::new(),
    };

    let source = format!("bb:{plane}");
    let mut runs = Vec::new();
    for item in items {
        let Some(created_raw) = item.get("created_at").and_then(Value::as_str) else {
            continue;
        };
        let Some(created_at) = parse_flexible_ts(created_raw) else {
            continue;
        };
        if !window.contains(created_at) {
            continue;
        }
        runs.push(BbRun {
            id: item
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            task: item
                .get("task")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            agent: item
                .get("agent_name")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            state: item
                .get("state")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            created_at: created_at.to_rfc3339(),
            source: source.clone(),
        });
    }
    runs.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    runs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::git::ts;
    use serde_json::json;

    #[test]
    fn extracts_runs_in_window_from_array_shape() {
        let payload = json!([
            {"id": "run-1", "task": "build", "agent_name": "vulcan", "state": "done", "created_at": "2026-07-05T01:00:00Z"},
            {"id": "run-2", "task": "review", "agent_name": "cerberus", "state": "done", "created_at": "2020-01-01T00:00:00Z"},
        ]);
        let window = RetroWindow::custom(ts(2026, 7, 4, 0, 0, 0), ts(2026, 7, 6, 0, 0, 0)).unwrap();

        let runs = extract_runs_in_window(&payload, "test-plane", &window);

        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].id, "run-1");
        assert_eq!(runs[0].source, "bb:test-plane");
    }

    #[test]
    fn extracts_runs_from_wrapped_object_shape() {
        let payload = json!({"runs": [
            {"id": "run-3", "task": "gardener", "agent_name": "gardener", "state": "running", "created_at": "2026-07-05T12:00:00Z"},
        ]});
        let window = RetroWindow::custom(ts(2026, 7, 4, 0, 0, 0), ts(2026, 7, 6, 0, 0, 0)).unwrap();

        let runs = extract_runs_in_window(&payload, "test-plane", &window);

        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].id, "run-3");
    }

    #[test]
    fn no_plane_configured_returns_empty_without_panicking() {
        let window = RetroWindow::custom(ts(2026, 7, 4, 0, 0, 0), ts(2026, 7, 6, 0, 0, 0)).unwrap();
        assert!(collect_bb_runs(None, &window).is_empty());
    }
}
