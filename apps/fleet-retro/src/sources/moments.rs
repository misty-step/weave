use std::path::Path;
use std::process::Command;

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;

use crate::window::RetroWindow;

/// One published moment card from Bitterblossom's flight-recorder anomaly
/// scorer (`scripts/moment-scorer.py`, bitterblossom-914): a curated,
/// deterministically-scored (no model judgment) surprise/failure/recovery/
/// cost_anomaly signal against that plane's own run ledger, capped at 3
/// published cards/day fleet-wide. This collector reads the scorer's own
/// `list --json` output as an external contract (a Python CLI, not a shared
/// Rust type), matching `bb.rs`'s existing convention for `bb runs list
/// --json` -- a scorer-side field rename degrades this collector instead of
/// breaking the build.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MomentCard {
    pub run_id: String,
    pub task: String,
    pub class: String,
    pub excerpt: String,
    pub run_link: String,
    pub created_at: String,
    pub source: String,
}

fn parse_flexible_ts(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.with_timezone(&Utc))
        .ok()
}

/// Pure extraction over an already-parsed `moment-scorer.py list --json`
/// payload (a bare JSON array of card objects). Kept separate from the
/// process invocation so it is unit-testable against a hand-built fixture
/// without shelling out to python3.
pub fn extract_cards_in_window(
    value: &Value,
    moments_db: &str,
    window: &RetroWindow,
) -> Vec<MomentCard> {
    let Value::Array(items) = value else {
        return Vec::new();
    };
    let source = format!("moment:{moments_db}");
    let mut cards = Vec::new();
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
        cards.push(MomentCard {
            run_id: item
                .get("run_id")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            task: item
                .get("task")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            class: item
                .get("class")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            excerpt: item
                .get("excerpt")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            run_link: item
                .get("run_link")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            created_at: created_at.to_rfc3339(),
            source: source.clone(),
        });
    }
    cards.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    cards
}

/// Shell out to `python3 <script> list --moments-db <db> --json` and return
/// the published cards (the review queue's curated, capped set -- an
/// unpublished over-cap card is deliberately excluded, matching the
/// scorer's own daily-cap intent) whose `created_at` falls in `window`.
/// Returns an empty list, not an error, when `script` or `moments_db` is
/// `None`: there is no single fleet-wide moments store today, and a wrong
/// guess (reading an unrelated example plane's store) would be worse than
/// an honest "not configured".
pub fn collect_moments(
    script: Option<&Path>,
    moments_db: Option<&Path>,
    window: &RetroWindow,
) -> Vec<MomentCard> {
    let (Some(script), Some(moments_db)) = (script, moments_db) else {
        eprintln!("fleet-retro: no moment-scorer script/db configured; skipping moments");
        return Vec::new();
    };
    let output = match Command::new("python3")
        .arg(script)
        .arg("list")
        .arg("--moments-db")
        .arg(moments_db)
        .arg("--json")
        .output()
    {
        Ok(output) => output,
        Err(err) => {
            eprintln!("fleet-retro: moment-scorer list failed to execute: {err}");
            return Vec::new();
        }
    };
    if !output.status.success() {
        eprintln!(
            "fleet-retro: moment-scorer list exited nonzero: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return Vec::new();
    }
    let Ok(value) = serde_json::from_slice::<Value>(&output.stdout) else {
        eprintln!("fleet-retro: moment-scorer list did not return valid JSON");
        return Vec::new();
    };
    extract_cards_in_window(&value, &moments_db.display().to_string(), window)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::git::ts;
    use serde_json::json;

    #[test]
    fn extracts_published_cards_in_window() {
        let payload = json!([
            {"run_id": "run-1", "task": "build", "class": "failure", "excerpt": "attempt 1 failed: timeout", "run_link": "bb runs show run-1 --json", "published": true, "created_at": "2026-07-05T04:00:00Z"},
            {"run_id": "run-2", "task": "review", "class": "surprise", "excerpt": "guard event fired", "run_link": "bb runs show run-2 --json", "published": true, "created_at": "2020-01-01T00:00:00Z"},
        ]);
        let window = RetroWindow::custom(ts(2026, 7, 4, 0, 0, 0), ts(2026, 7, 6, 0, 0, 0)).unwrap();

        let cards = extract_cards_in_window(&payload, "test-plane/.bb/moments.db", &window);

        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].run_id, "run-1");
        assert_eq!(cards[0].class, "failure");
        assert_eq!(cards[0].source, "moment:test-plane/.bb/moments.db");
    }

    #[test]
    fn no_script_or_db_configured_returns_empty_without_panicking() {
        let window = RetroWindow::custom(ts(2026, 7, 4, 0, 0, 0), ts(2026, 7, 6, 0, 0, 0)).unwrap();
        assert!(collect_moments(None, None, &window).is_empty());
    }

    #[test]
    fn malformed_or_missing_fields_are_skipped_not_fatal() {
        let payload = json!([
            {"task": "build", "class": "failure"},
        ]);
        let window = RetroWindow::custom(ts(2026, 7, 4, 0, 0, 0), ts(2026, 7, 6, 0, 0, 0)).unwrap();
        let cards = extract_cards_in_window(&payload, "db", &window);
        assert!(cards.is_empty());
    }
}
