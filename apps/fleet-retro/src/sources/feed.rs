use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::window::RetroWindow;

/// Known `feed-post` event kinds (mirrors `~/.factory-lanes/scripts/feed-post`'s
/// `KINDS` tuple). A line whose `kind` is not one of these is not a
/// feed-post entry -- most likely a different producer appending its own
/// schema into the same daily file (observed live: counterspell's
/// `weave.remote_event.v1` session-routing telemetry shares
/// `~/.factory-lanes/feed/*.jsonl` with the digest poster). Filtering on
/// this closed set, rather than "did it parse as JSON," is what keeps that
/// noise out of the retro.
const KNOWN_KINDS: &[&str] = &[
    "shipped", "report", "blocked", "question", "note", "digest", "release", "receipt",
];

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FeedEvent {
    pub ts: String,
    pub agent: String,
    pub kind: String,
    pub title: String,
    pub body: Option<String>,
    pub links: Vec<FeedLink>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeedLink {
    pub label: String,
    pub url: String,
}

impl FeedEvent {
    fn parsed_ts(&self) -> Option<DateTime<Utc>> {
        DateTime::parse_from_rfc3339(&self.ts)
            .ok()
            .map(|dt| dt.with_timezone(&Utc))
    }
}

/// Parse one JSONL line as a feed-post event. Returns `None` (never an
/// error) for anything that isn't a recognizable feed-post entry: malformed
/// JSON, missing required fields, or a `kind` outside the known set. A
/// retro run should never fail because some other tool shares the feed
/// directory; it should just not count that tool's lines as fleet activity.
fn parse_feed_line(line: &str, source: &str) -> Option<FeedEvent> {
    let value: Value = serde_json::from_str(line).ok()?;
    let kind = value.get("kind")?.as_str()?;
    if !KNOWN_KINDS.contains(&kind) {
        return None;
    }
    let ts = value.get("ts")?.as_str()?.to_string();
    let title = value.get("title")?.as_str()?.to_string();
    let agent = value
        .get("agent")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let body = value
        .get("body")
        .and_then(Value::as_str)
        .map(str::to_string);
    let links = value
        .get("links")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| serde_json::from_value(item.clone()).ok())
                .collect()
        })
        .unwrap_or_default();
    Some(FeedEvent {
        ts,
        agent,
        kind: kind.to_string(),
        title,
        body,
        links,
        source: source.to_string(),
    })
}

/// Read every `*.jsonl` file directly under `feed_dir` and return the
/// recognizable feed-post events whose timestamp falls in `window`. Files
/// are read in name order (the poster names files `YYYY-MM-DD.jsonl`, so
/// this is also chronological); a file that fails to open is skipped, not
/// fatal, since the window naturally narrows which day-files matter.
pub fn collect_feed_events(feed_dir: &Path, window: &RetroWindow) -> Vec<FeedEvent> {
    let Ok(entries) = std::fs::read_dir(feed_dir) else {
        return Vec::new();
    };
    let mut files: Vec<_> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "jsonl"))
        .collect();
    files.sort();

    let mut events = Vec::new();
    for path in files {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let source = format!("feed:{}", path.display());
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let Some(event) = parse_feed_line(line, &source) else {
                continue;
            };
            let Some(ts) = event.parsed_ts() else {
                continue;
            };
            if window.contains(ts) {
                events.push(event);
            }
        }
    }
    events.sort_by(|a, b| a.ts.cmp(&b.ts));
    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::git::ts;

    fn write_day(dir: &Path, name: &str, lines: &[&str]) {
        std::fs::write(dir.join(name), lines.join("\n") + "\n").unwrap();
    }

    #[test]
    fn recognizes_a_wellformed_digest_line() {
        let dir = tempfile::tempdir().unwrap();
        write_day(
            dir.path(),
            "2026-07-05.jsonl",
            &[
                r#"{"ts": "2026-07-05T01:04:48Z", "agent": "linejam-overhaul", "kind": "shipped", "title": "linejam overhaul complete", "links": [{"label": "pr", "url": "https://example.invalid/pr/300"}]}"#,
            ],
        );
        let window = RetroWindow::custom(ts(2026, 7, 4, 0, 0, 0), ts(2026, 7, 6, 0, 0, 0)).unwrap();

        let events = collect_feed_events(dir.path(), &window);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].title, "linejam overhaul complete");
        assert_eq!(events[0].links[0].url, "https://example.invalid/pr/300");
    }

    #[test]
    fn skips_foreign_schema_lines_sharing_the_same_file() {
        // This is the live counterspell "weave.remote_event.v1" shape found
        // in ~/.factory-lanes/feed/2026-07-05.jsonl: valid JSON, no `kind`
        // in the feed-post KINDS set, must never be counted as fleet
        // activity or crash the parser.
        let dir = tempfile::tempdir().unwrap();
        write_day(
            dir.path(),
            "2026-07-05.jsonl",
            &[
                r#"{"action":"session_ignored","actor":{"id":"counterspell","kind":"system"},"occurred_at":"2026-07-05T04:26:00Z","schema_version":"weave.remote_event.v1"}"#,
            ],
        );
        let window = RetroWindow::custom(ts(2026, 7, 4, 0, 0, 0), ts(2026, 7, 6, 0, 0, 0)).unwrap();

        let events = collect_feed_events(dir.path(), &window);

        assert!(events.is_empty());
    }

    #[test]
    fn skips_malformed_json_without_failing() {
        let dir = tempfile::tempdir().unwrap();
        write_day(dir.path(), "2026-07-05.jsonl", &["not json at all {{{"]);
        let window = RetroWindow::custom(ts(2026, 7, 4, 0, 0, 0), ts(2026, 7, 6, 0, 0, 0)).unwrap();

        let events = collect_feed_events(dir.path(), &window);

        assert!(events.is_empty());
    }

    #[test]
    fn excludes_events_outside_the_window() {
        let dir = tempfile::tempdir().unwrap();
        write_day(
            dir.path(),
            "2026-07-01.jsonl",
            &[
                r#"{"ts": "2026-07-01T12:00:00Z", "agent": "x", "kind": "shipped", "title": "too early"}"#,
            ],
        );
        let window = RetroWindow::custom(ts(2026, 7, 4, 0, 0, 0), ts(2026, 7, 6, 0, 0, 0)).unwrap();

        let events = collect_feed_events(dir.path(), &window);

        assert!(events.is_empty());
    }
}
