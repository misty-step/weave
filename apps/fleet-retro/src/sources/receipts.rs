use std::path::Path;

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::window::RetroWindow;

const EXCERPT_WORD_LIMIT: usize = 40;

/// One narrative receipt from `~/.factory-lanes/campaign/` (see
/// `docs/posting-contract.md`'s "Receipt frontmatter" section in the
/// factory-ops repo for the field contract this reads). Receipts are the
/// fleet's richest narrative source and, unlike git/Powder/bb/feed, carry no
/// structured API of their own -- this collector reads the minimal
/// frontmatter block new receipts write and backfilled receipts were given,
/// falling back to the file's mtime when a receipt predates the convention
/// or omits `ts`.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ReceiptItem {
    pub path: String,
    pub title: String,
    pub excerpt: String,
    pub cards: Vec<String>,
    pub ts: String,
    pub source: String,
}

/// The handful of frontmatter fields this collector reads. Parsed with a
/// small hand-rolled scanner rather than a YAML crate: the block is a
/// controlled subset (scalar `key: value` lines plus `key: [a, b]` flow
/// lists) that this same convention doc defines, so a full YAML parser would
/// buy correctness this format doesn't need at the cost of a new dependency.
#[derive(Debug, Default, PartialEq, Eq)]
struct Frontmatter {
    ts: Option<String>,
    cards: Vec<String>,
}

fn parse_flow_list(value: &str) -> Vec<String> {
    let trimmed = value.trim();
    let Some(inner) = trimmed.strip_prefix('[').and_then(|s| s.strip_suffix(']')) else {
        return Vec::new();
    };
    inner
        .split(',')
        .map(|item| {
            item.trim()
                .trim_matches(|c| c == '"' || c == '\'')
                .to_string()
        })
        .filter(|item| !item.is_empty())
        .collect()
}

/// Split a receipt's text into (frontmatter, body). Returns `None`
/// frontmatter when the file doesn't open with a `---` delimiter -- most
/// receipts predate the convention and simply have no block, which is a
/// normal case for this collector (it falls back to mtime), not an error.
fn parse_frontmatter(text: &str) -> (Option<Frontmatter>, &str) {
    let Some(rest) = text.strip_prefix("---\n") else {
        return (None, text);
    };
    let Some(end) = rest.find("\n---\n") else {
        return (None, text);
    };
    let block = &rest[..end];
    let body = &rest[end + "\n---\n".len()..];

    let mut fm = Frontmatter::default();
    for line in block.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        match key.trim() {
            "ts" => fm.ts = Some(value.trim().to_string()),
            "cards" => fm.cards = parse_flow_list(value),
            _ => {}
        }
    }
    (Some(fm), body)
}

/// The receipt's title: the first Markdown heading (`# ...`) in the body, or
/// the file's stem when no heading is present.
fn extract_title(body: &str, path: &Path) -> String {
    body.lines()
        .find_map(|line| line.trim().strip_prefix("# ").map(str::trim))
        .filter(|heading| !heading.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            path.file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default()
        })
}

/// A short, plain-text excerpt: every word in the body after the title
/// heading (if any), collapsed onto one line, truncated to
/// `EXCERPT_WORD_LIMIT` words with an ellipsis when longer.
fn extract_excerpt(body: &str, title: &str) -> String {
    let title_line = format!("# {title}");
    let after_title = body
        .lines()
        .skip_while(|line| line.trim().is_empty())
        .skip_while(|line| line.trim() == title_line)
        .collect::<Vec<_>>()
        .join(" ");
    // Strip Markdown syntax before truncating -- doing this after would let
    // a `**bold**` marker survive as a literal asterisk, or worse, split it
    // in half (found live: a receipt excerpt rendered `**Delivery = two-phas…`).
    let after_title = crate::text::plain_text(&after_title);
    let words: Vec<&str> = after_title.split_whitespace().collect();
    if words.len() <= EXCERPT_WORD_LIMIT {
        words.join(" ")
    } else {
        format!("{}…", words[..EXCERPT_WORD_LIMIT].join(" "))
    }
}

fn mtime_rfc3339(path: &Path) -> Option<String> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    let dt: DateTime<Utc> = modified.into();
    Some(dt.to_rfc3339())
}

fn parse_ts(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

/// Read every `*.md` file directly under `dir` (the campaign receipts
/// directory) and return the ones whose effective timestamp -- frontmatter
/// `ts`, falling back to file mtime -- falls in `window`. A file that can't
/// be read or has no resolvable timestamp is skipped, not fatal, matching
/// every other collector's "one bad input doesn't blank the source" rule.
pub fn collect_receipts(dir: &Path, window: &RetroWindow) -> Vec<ReceiptItem> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<_> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "md"))
        .collect();
    files.sort();

    let mut items = Vec::new();
    for path in files {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let (frontmatter, body) = parse_frontmatter(&text);
        let ts_raw = frontmatter
            .as_ref()
            .and_then(|fm| fm.ts.clone())
            .or_else(|| mtime_rfc3339(&path));
        let Some(ts_raw) = ts_raw else { continue };
        let Some(ts) = parse_ts(&ts_raw) else {
            continue;
        };
        if !window.contains(ts) {
            continue;
        }
        let title = extract_title(body, &path);
        let excerpt = extract_excerpt(body, &title);
        let cards = frontmatter.map(|fm| fm.cards).unwrap_or_default();
        items.push(ReceiptItem {
            path: path.display().to_string(),
            title,
            excerpt,
            cards,
            ts: ts.to_rfc3339(),
            source: format!("receipt:{}", path.display()),
        });
    }
    items.sort_by(|a, b| a.ts.cmp(&b.ts));
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::git::ts;

    fn write(dir: &Path, name: &str, contents: &str) {
        std::fs::write(dir.join(name), contents).unwrap();
    }

    #[test]
    fn reads_frontmatter_ts_and_cards_within_window() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "bitterblossom-118-report.md",
            "---\nts: 2026-07-05T04:40:57+00:00\nkind: report\ncards: [bitterblossom-118]\nrepos: [bitterblossom]\n---\n# bitterblossom-118 — Dashboard data completeness\n\nShipped the read APIs end to end and verified against a live plane.\n",
        );
        let window = RetroWindow::custom(ts(2026, 7, 4, 0, 0, 0), ts(2026, 7, 6, 0, 0, 0)).unwrap();

        let items = collect_receipts(dir.path(), &window);

        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].title,
            "bitterblossom-118 — Dashboard data completeness"
        );
        assert_eq!(items[0].cards, vec!["bitterblossom-118"]);
        assert_eq!(items[0].ts, "2026-07-05T04:40:57+00:00");
        assert!(items[0].excerpt.contains("Shipped the read APIs"));
    }

    #[test]
    fn falls_back_to_mtime_when_no_frontmatter_present() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "lane-contract.md",
            "# Campaign lane contract\n\nSome body text.\n",
        );
        // A window generously wide around "now" -- the file's real mtime
        // will land in it since we just wrote it, without needing to fake
        // mtimes via a platform-specific syscall.
        let now = Utc::now();
        let window = RetroWindow::custom(
            now - chrono::Duration::hours(1),
            now + chrono::Duration::hours(1),
        )
        .unwrap();

        let items = collect_receipts(dir.path(), &window);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Campaign lane contract");
        assert!(items[0].cards.is_empty());
    }

    #[test]
    fn excludes_receipts_outside_the_window() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "old.md",
            "---\nts: 2020-01-01T00:00:00+00:00\nkind: receipt\n---\n# Old receipt\nBody.\n",
        );
        let window = RetroWindow::custom(ts(2026, 7, 4, 0, 0, 0), ts(2026, 7, 6, 0, 0, 0)).unwrap();

        let items = collect_receipts(dir.path(), &window);

        assert!(items.is_empty());
    }

    #[test]
    fn ignores_non_markdown_files() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "notes.txt", "not a receipt");
        let now = Utc::now();
        let window = RetroWindow::custom(
            now - chrono::Duration::hours(1),
            now + chrono::Duration::hours(1),
        )
        .unwrap();

        let items = collect_receipts(dir.path(), &window);

        assert!(items.is_empty());
    }

    #[test]
    fn excerpt_is_truncated_to_word_limit_with_ellipsis() {
        let long_body = format!("# Long receipt\n\n{}", "word ".repeat(80));
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "long.md",
            &format!("---\nts: 2026-07-05T00:00:00+00:00\nkind: receipt\n---\n{long_body}"),
        );
        let window = RetroWindow::custom(ts(2026, 7, 4, 0, 0, 0), ts(2026, 7, 6, 0, 0, 0)).unwrap();

        let items = collect_receipts(dir.path(), &window);

        assert_eq!(items.len(), 1);
        assert!(items[0].excerpt.ends_with('…'));
        assert_eq!(
            items[0].excerpt.split_whitespace().count(),
            EXCERPT_WORD_LIMIT
        );
    }

    #[test]
    fn excerpt_strips_markdown_bold_markers_before_truncating() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "shaped.md",
            "---\nts: 2026-07-05T00:00:00+00:00\nkind: receipt\n---\n# Shaped\n\n**Delivery = two-phased**, ship then harden.\n",
        );
        let window = RetroWindow::custom(ts(2026, 7, 4, 0, 0, 0), ts(2026, 7, 6, 0, 0, 0)).unwrap();

        let items = collect_receipts(dir.path(), &window);

        assert_eq!(items.len(), 1);
        assert!(
            !items[0].excerpt.contains('*'),
            "excerpt must not leak literal markdown markers: {}",
            items[0].excerpt
        );
        assert!(items[0].excerpt.contains("Delivery = two-phased"));
    }

    #[test]
    fn missing_directory_returns_empty_without_panicking() {
        let window = RetroWindow::custom(ts(2026, 7, 4, 0, 0, 0), ts(2026, 7, 6, 0, 0, 0)).unwrap();
        assert!(collect_receipts(Path::new("/nonexistent/path/xyz"), &window).is_empty());
    }
}
