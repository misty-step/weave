use std::collections::{BTreeMap, BTreeSet};

use glance_catalog::inline::InlineNode;
use glance_catalog::leaf::Metric;
use glance_catalog::structural::{
    Cell, CellValue, ColumnSpec, Hero, Narrative, Row, Table, Timeline, TimelineEntry,
};

use crate::pack::EvidencePack;
use crate::sources::SourceNote;
use crate::spec::*;
use crate::window::RetroWindow;

const TIMELINE_LIMIT: usize = 300;
const HIGHLIGHTS_PER_REPO: usize = 4;

#[derive(Default)]
struct RepoRollup {
    commits: usize,
    prs: BTreeSet<String>,
    cards: BTreeSet<String>,
    highlights: Vec<(String, String)>, // (at, text) for sort-then-truncate
    /// Set only when this repo row has ever seen a git-sourced item
    /// (`repo-swept`/`commit`/`pr-ref`) -- the per-repo "N commits, N PR
    /// reference(s)" provenance note is a git-specific claim, and a repo
    /// name that only ever showed up via a Powder card or a bb task must not
    /// get one.
    git_source: Option<String>,
}

/// The repo key a feed-post event rolls up under: the posting agent's name,
/// unless it's one of fleet-retro's own generator identities (in which case
/// there is no more specific repo to attribute it to than "fleet" itself).
fn derive_feed_repo(agent: &str) -> String {
    if agent != "fleet-digest" && agent != "fleet-retro" && !agent.is_empty() {
        return agent.to_string();
    }
    "fleet".to_string()
}

const REPO_TABLE_COLUMNS: [(&str, &str, bool); 4] = [
    ("commits", "commits", true),
    ("prs", "PRs", true),
    ("cards_touched", "cards", true),
    ("highlights", "highlights", false),
];

fn repo_table_columns() -> Vec<ColumnSpec> {
    let mut columns = vec![ColumnSpec {
        key: "repo".to_string(),
        label: "repo".to_string(),
        numeric: false,
        // The "item that matters" column -- aesthetic.css's .ae-item,
        // matching glance-gen's FileTable name column and this table's own
        // pre-aesthetic-926 repo cell.
        emphasize: true,
    }];
    columns.extend(
        REPO_TABLE_COLUMNS
            .iter()
            .map(|(key, label, numeric)| ColumnSpec {
                key: key.to_string(),
                label: label.to_string(),
                numeric: *numeric,
                emphasize: false,
            }),
    );
    columns
}

struct RepoActivityRow {
    repo: String,
    commits: usize,
    prs: usize,
    cards_touched: usize,
    highlights: Vec<String>,
}

fn repo_activity_row_to_table_row(row: RepoActivityRow) -> Row {
    Row {
        cells: vec![
            Cell {
                column_key: "repo".to_string(),
                value: CellValue::Text { text: row.repo },
            },
            Cell {
                column_key: "commits".to_string(),
                value: CellValue::Text {
                    text: row.commits.to_string(),
                },
            },
            Cell {
                column_key: "prs".to_string(),
                value: CellValue::Text {
                    text: row.prs.to_string(),
                },
            },
            Cell {
                column_key: "cards_touched".to_string(),
                value: CellValue::Text {
                    text: row.cards_touched.to_string(),
                },
            },
            Cell {
                column_key: "highlights".to_string(),
                value: CellValue::List {
                    items: row.highlights,
                },
            },
        ],
    }
}

/// Pure assembly: turn an already-collected, already-projected
/// `EvidencePack` into a validated `RetroSpec`. Nothing in here does I/O --
/// `pack::build_pack` already turned every collector's native output into
/// the pack's generic `{id, ts, source, kind, title, refs, excerpt}` items,
/// so this function (and therefore the report's shape) is fully
/// unit-testable against a hand-built pack, independent of live
/// git/Powder/bb state.
///
/// Dispatch is keyed on each item's `source` prefix (`git:`, `powder:`,
/// `bb:`, `feed:`, `receipt:`) rather than `kind` alone: feed-post's own
/// `KNOWN_KINDS` enum reserves `"receipt"` as a valid feed-post kind (for
/// receipt mirrors), which collides with the campaign-receipts collector's
/// `"receipt"` kind -- the source prefix disambiguates them unambiguously
/// since every collector already stamps a distinct one.
#[allow(clippy::too_many_arguments)]
pub fn build_spec(
    window: &RetroWindow,
    generated_at: &str,
    pack: &EvidencePack,
    narrative: Narrative,
    narrative_citations: Vec<Citation>,
    footer: Footer,
    mut notes: Vec<SourceNote>,
) -> anyhow::Result<RetroSpec> {
    let mut repos: BTreeMap<String, RepoRollup> = BTreeMap::new();
    let mut timeline: Vec<TimelineEntry> = Vec::new();
    let mut receipt_rows: Vec<ReceiptRow> = Vec::new();
    let mut total_commits = 0usize;
    let mut all_prs: BTreeSet<String> = BTreeSet::new();
    let mut all_cards: BTreeSet<String> = BTreeSet::new();
    let mut total_bb_runs = 0usize;
    let mut total_feed_events = 0usize;
    let mut total_card_movements = 0usize;
    let mut total_receipts = 0usize;
    let mut total_moments = 0usize;

    for item in &pack.items {
        // Dispatch on the *source* prefix first, not `kind` alone:
        // feed-post's own `KNOWN_KINDS` enum reserves `"receipt"` as a valid
        // feed-post kind (for receipt mirrors), which collides with the
        // campaign-receipts collector's `"receipt"` kind. Every collector
        // already stamps a distinct source prefix (`git:`, `powder:`, `bb:`,
        // `feed:`, `receipt:`), so checking that first resolves the
        // collision unambiguously; only within the `git:` family does `kind`
        // then distinguish `repo-swept`/`commit`/`pr-ref`.
        if item.source.starts_with("git:") {
            match item.kind.as_str() {
                "repo-swept" => {
                    let repo = item.ref_value("repo:").unwrap_or("unknown").to_string();
                    let rollup = repos.entry(repo).or_default();
                    rollup.git_source.get_or_insert_with(|| item.source.clone());
                }
                "commit" => {
                    let repo = item.ref_value("repo:").unwrap_or("unknown").to_string();
                    total_commits += 1;
                    if let Some(pr) = item.ref_value("pr:") {
                        all_prs.insert(format!("{repo}#{pr}"));
                    }
                    let rollup = repos.entry(repo.clone()).or_default();
                    rollup.git_source.get_or_insert_with(|| item.source.clone());
                    rollup.commits += 1;
                    if let Some(pr) = item.ref_value("pr:") {
                        rollup.prs.insert(pr.to_string());
                    }
                    rollup
                        .highlights
                        .push((item.ts.clone(), item.title.clone()));
                    timeline.push(TimelineEntry {
                        at: item.ts.clone(),
                        actor: repo,
                        kind: "commit".to_string(),
                        summary: item.title.clone(),
                        link: None,
                        detail: vec![],
                    });
                }
                "pr-ref" => {
                    let repo = item.ref_value("repo:").unwrap_or("unknown").to_string();
                    if let Some(pr) = item.ref_value("pr:") {
                        all_prs.insert(format!("{repo}#{pr}"));
                        let rollup = repos.entry(repo).or_default();
                        rollup.git_source.get_or_insert_with(|| item.source.clone());
                        rollup.prs.insert(pr.to_string());
                    }
                }
                _ => {}
            }
        } else if item.source.starts_with("powder:") {
            let repo = item.ref_value("repo:").unwrap_or("unknown").to_string();
            let card_id = item.ref_value("card:").unwrap_or("unknown").to_string();
            total_card_movements += 1;
            all_cards.insert(card_id.clone());
            let rollup = repos.entry(repo.clone()).or_default();
            rollup.cards.insert(card_id.clone());
            rollup
                .highlights
                .push((item.ts.clone(), format!("{card_id}: {}", item.excerpt)));
            timeline.push(TimelineEntry {
                at: item.ts.clone(),
                actor: repo,
                kind: item.kind.clone(),
                summary: item.title.clone(),
                link: None,
                detail: vec![],
            });
        } else if item.source.starts_with("bb:") {
            total_bb_runs += 1;
            let task = item.ref_value("task:").unwrap_or("unknown");
            let state = item.ref_value("state:").unwrap_or("unknown");
            let repo = format!("bb:{task}");
            let rollup = repos.entry(repo.clone()).or_default();
            rollup
                .highlights
                .push((item.ts.clone(), format!("{task} run {state}")));
            timeline.push(TimelineEntry {
                at: item.ts.clone(),
                actor: repo,
                kind: "bb-run".to_string(),
                summary: item.title.clone(),
                link: None,
                detail: vec![],
            });
        } else if item.source.starts_with("receipt:") {
            total_receipts += 1;
            let path = item.ref_value("path:").unwrap_or("").to_string();
            let cards = item
                .ref_values("card:")
                .into_iter()
                .map(str::to_string)
                .collect();
            receipt_rows.push(ReceiptRow {
                title: item.title.clone(),
                excerpt: item.excerpt.clone(),
                path,
                cards,
                at: item.ts.clone(),
            });
        } else if item.source.starts_with("feed:") {
            total_feed_events += 1;
            let agent = item.ref_value("agent:").unwrap_or("");
            let repo = derive_feed_repo(agent);
            let link = item
                .ref_values("link:")
                .into_iter()
                .next()
                .map(str::to_string);
            let rollup = repos.entry(repo.clone()).or_default();
            rollup
                .highlights
                .push((item.ts.clone(), item.title.clone()));
            timeline.push(TimelineEntry {
                at: item.ts.clone(),
                actor: repo,
                kind: item.kind.clone(),
                summary: item.title.clone(),
                link,
                detail: vec![],
            });
        } else if item.source.starts_with("moment:") {
            // Rolls up into the same `bb:{task}` bucket a regular `bb-run`
            // item uses (both describe that task's own runs) rather than a
            // separate "moments" repo row -- an anomaly card is a fact
            // *about* that task's runs, not a distinct activity source.
            total_moments += 1;
            let task = item.ref_value("task:").unwrap_or("unknown");
            let repo = format!("bb:{task}");
            let rollup = repos.entry(repo.clone()).or_default();
            rollup
                .highlights
                .push((item.ts.clone(), format!("{}: {}", item.kind, item.excerpt)));
            timeline.push(TimelineEntry {
                at: item.ts.clone(),
                actor: repo,
                kind: format!("moment-{}", item.kind),
                summary: item.title.clone(),
                link: None,
                detail: vec![],
            });
        }
        // An item from an unrecognized source is ignored rather than
        // failing the whole report, matching every collector's own "one
        // bad input doesn't blank the source" rule.
    }

    // Per-repo git provenance notes, in repo-name order. This coincides with
    // the discovery order collectors originally pushed these notes in
    // (`git::discover_repos` sorts by path, and repo names share a common
    // dev-root prefix so name order and path order agree in practice) --
    // covered by this module's byte-identical regression test against a
    // pre-refactor baseline capture.
    for rollup in repos.values() {
        let Some(source) = &rollup.git_source else {
            continue;
        };
        if rollup.commits == 0 && rollup.prs.is_empty() {
            continue;
        }
        notes.push(SourceNote::new(
            source.clone(),
            format!(
                "{} commits, {} PR reference(s) in window",
                rollup.commits,
                rollup.prs.len()
            ),
        ));
    }
    if total_bb_runs > 0 {
        notes.push(SourceNote::new(
            "bb",
            format!("{total_bb_runs} plane run(s) in window"),
        ));
    }
    if total_feed_events > 0 {
        notes.push(SourceNote::new(
            "feed",
            format!("{total_feed_events} feed event(s) in window"),
        ));
    }
    if total_card_movements > 0 {
        notes.push(SourceNote::new(
            "powder",
            format!(
                "{total_card_movements} card movement(s) across {} card(s) in window",
                all_cards.len()
            ),
        ));
    }
    if total_receipts > 0 {
        notes.push(SourceNote::new(
            "receipts",
            format!("{total_receipts} campaign receipt(s) in window"),
        ));
    }
    if total_moments > 0 {
        notes.push(SourceNote::new(
            "moments",
            format!("{total_moments} moment-scorer anomaly card(s) in window"),
        ));
    }

    timeline.sort_by(|a, b| a.at.cmp(&b.at));
    timeline.reverse();
    let truncated = timeline.len().saturating_sub(TIMELINE_LIMIT);
    timeline.truncate(TIMELINE_LIMIT);
    if truncated > 0 {
        notes.push(SourceNote::new(
            "assemble",
            format!("timeline truncated to the {TIMELINE_LIMIT} most recent entries ({truncated} older entries omitted)"),
        ));
    }

    // A repo with zero commits, zero PR references, and zero cards touched
    // carries no signal -- listing it as a dead table row is exactly the
    // "mostly whitespace-padded silence" a live designer critique flagged.
    // Split it into `quiet_repos` instead of a row; `render.rs` folds that
    // list into a single muted note beneath the table, the same demotion
    // this function's own zero-repo provenance note already applies below.
    let mut quiet_repos: Vec<String> = Vec::new();
    let rows: Vec<Row> = repos
        .into_iter()
        .filter_map(|(repo, mut rollup)| {
            let no_signal = rollup.commits == 0
                && rollup.prs.is_empty()
                && rollup.cards.is_empty()
                && rollup.highlights.is_empty();
            if no_signal {
                quiet_repos.push(repo);
                return None;
            }
            rollup.highlights.sort_by(|a, b| b.0.cmp(&a.0));
            let highlights = rollup
                .highlights
                .into_iter()
                .take(HIGHLIGHTS_PER_REPO)
                .map(|(_, text)| text)
                .collect();
            Some(repo_activity_row_to_table_row(RepoActivityRow {
                repo,
                commits: rollup.commits,
                prs: rollup.prs.len(),
                cards_touched: rollup.cards.len(),
                highlights,
            }))
        })
        .collect();

    let notes_view: Vec<ProvenanceNote> = notes
        .into_iter()
        .map(|n| ProvenanceNote {
            source: n.source,
            note: n.note,
        })
        .collect();

    // All 8 pre-aesthetic-926 StatCallout items, now merged into Hero.stats
    // (glance-catalog's Hero has no upper bound -- see that crate's fix
    // commit -- so nothing here is truncated or demoted).
    let stats = vec![
        Metric {
            label: "Commits".into(),
            value: total_commits.to_string(),
        },
        Metric {
            label: "PRs".into(),
            value: all_prs.len().to_string(),
        },
        Metric {
            label: "Cards touched".into(),
            value: all_cards.len().to_string(),
        },
        Metric {
            label: "bb runs".into(),
            value: total_bb_runs.to_string(),
        },
        Metric {
            label: "Feed events".into(),
            value: total_feed_events.to_string(),
        },
        Metric {
            label: "Receipts".into(),
            value: total_receipts.to_string(),
        },
        Metric {
            label: "Moments".into(),
            value: total_moments.to_string(),
        },
        Metric {
            label: "Window".into(),
            value: format!("{}h", window.duration_hours()),
        },
    ];

    let repo_table_empty_note = if rows.is_empty() {
        Some("No repo activity in this window.".to_string())
    } else {
        None
    };
    let repo_table_demoted_note = if quiet_repos.is_empty() {
        None
    } else {
        Some(format!(
            "{} repo(s) swept with no activity: {}",
            quiet_repos.len(),
            quiet_repos.join(", ")
        ))
    };
    let timeline_empty_note = if timeline.is_empty() {
        Some("No dated events in this window.".to_string())
    } else {
        None
    };

    let spec = RetroSpec {
        catalog_version: CATALOG_VERSION.to_string(),
        title: format!("Fleet retro — {}", window.label),
        window_label: window.label.clone(),
        since: window.since.to_rfc3339(),
        until: window.until.to_rfc3339(),
        generated_at: generated_at.to_string(),
        components: vec![
            Component::Hero(Hero {
                title: format!("Fleet retro — {}", window.label),
                summary: vec![InlineNode::Text {
                    text: format!(
                        "{} → {} ({}h window)",
                        window.since.to_rfc3339(),
                        window.until.to_rfc3339(),
                        window.duration_hours()
                    ),
                }],
                stats,
                image_intent: None,
            }),
            // Narrative leads; the tables below it are the appendix (card
            // acceptance: "Daily retro leads with a cited narrative section;
            // tables demoted to appendix").
            Component::Narrative {
                narrative,
                citations: narrative_citations,
            },
            Component::Table(Table {
                heading: "Repo activity".to_string(),
                columns: repo_table_columns(),
                rows,
                empty_note: repo_table_empty_note,
                demoted_note: repo_table_demoted_note,
            }),
            Component::Timeline(Timeline {
                heading: "Timeline".to_string(),
                entries: timeline,
                empty_note: timeline_empty_note,
            }),
            Component::Receipts(Receipts {
                items: receipt_rows,
            }),
            Component::Footer(footer),
            Component::Provenance(Provenance { notes: notes_view }),
        ],
    };
    spec.validate()?;
    Ok(spec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pack::build_pack;
    use crate::sources::bb::BbRun;
    use crate::sources::feed::FeedEvent;
    use crate::sources::git::{RepoActivity, RepoCommit, ts};
    use crate::sources::powder::CardMovement;
    use crate::sources::receipts::ReceiptItem;
    use glance_catalog::structural::NarrativeStatus;

    fn window() -> RetroWindow {
        RetroWindow::custom(ts(2026, 7, 4, 21, 0, 0), ts(2026, 7, 5, 21, 0, 0)).unwrap()
    }

    /// A neutral stand-in for the synthesis stage's output -- these tests
    /// exercise repo/timeline/receipts/provenance assembly, not narrative
    /// rendering (that's `synthesis.rs`/`citation_gate.rs`'s own test
    /// coverage), so a fixed fail-open stub keeps every call site here
    /// honest about what it's actually testing.
    fn stub_narrative() -> Narrative {
        Narrative {
            heading: "What mattered".to_string(),
            status: NarrativeStatus::Unavailable {
                reason: "not exercised by this test".to_string(),
            },
        }
    }

    fn stub_footer() -> Footer {
        Footer {
            judge: "none".to_string(),
            gate_status: "not exercised by this test".to_string(),
            prompt_version: "test".to_string(),
            pack_schema_version: "test".to_string(),
            pack_assembly_ms: 0,
        }
    }

    fn cell_text<'a>(row: &'a Row, column_key: &str) -> &'a str {
        match &row
            .cells
            .iter()
            .find(|cell| cell.column_key == column_key)
            .unwrap_or_else(|| panic!("expected a cell for column {column_key}"))
            .value
        {
            CellValue::Text { text } => text.as_str(),
            other => panic!("expected CellValue::Text for {column_key}, got {other:?}"),
        }
    }

    fn cell_list<'a>(row: &'a Row, column_key: &str) -> &'a [String] {
        match &row
            .cells
            .iter()
            .find(|cell| cell.column_key == column_key)
            .unwrap_or_else(|| panic!("expected a cell for column {column_key}"))
            .value
        {
            CellValue::List { items } => items.as_slice(),
            other => panic!("expected CellValue::List for {column_key}, got {other:?}"),
        }
    }

    #[test]
    fn assembles_a_spec_that_names_every_source() {
        let activity = vec![RepoActivity {
            repo: "landmark".into(),
            source: "git:/dev/landmark".into(),
            commits: vec![RepoCommit {
                hash: "abc123".into(),
                subject: "fix(synthesis): ground release-note sections (#200)".into(),
                pr_number: Some("200".into()),
                at: "2026-07-05T04:20:00+00:00".into(),
            }],
            pr_numbers: vec!["200".into()],
        }];
        let pack = build_pack(&window(), &activity, &[], &[], &[], &[], &[]);
        let spec = build_spec(
            &window(),
            "2026-07-05T21:00:05Z",
            &pack,
            stub_narrative(),
            vec![],
            stub_footer(),
            vec![],
        )
        .unwrap();

        assert!(spec.validate().is_ok());
        let Component::Table(table) = &spec.components[2] else {
            panic!("expected repo table at index 2");
        };
        assert_eq!(table.rows.len(), 1);
        assert_eq!(cell_text(&table.rows[0], "repo"), "landmark");
        assert_eq!(cell_text(&table.rows[0], "commits"), "1");
        assert_eq!(cell_text(&table.rows[0], "prs"), "1");

        let Component::Provenance(provenance) = spec.components.last().unwrap() else {
            panic!("expected provenance last");
        };
        assert!(
            provenance
                .notes
                .iter()
                .any(|n| n.source.contains("landmark")),
            "provenance must name the git source that contributed activity"
        );
    }

    #[test]
    fn empty_evidence_still_produces_a_valid_spec_with_explicit_zeros() {
        let pack = build_pack(&window(), &[], &[], &[], &[], &[], &[]);
        let spec = build_spec(
            &window(),
            "2026-07-05T21:00:05Z",
            &pack,
            stub_narrative(),
            vec![],
            stub_footer(),
            vec![],
        )
        .unwrap();
        assert!(spec.validate().is_ok());
        let Component::Hero(hero) = &spec.components[0] else {
            panic!("expected hero at index 0");
        };
        assert_eq!(hero.stats[0].value, "0");
    }

    #[test]
    fn receipts_become_a_dedicated_component_and_a_provenance_note() {
        let receipts = vec![ReceiptItem {
            path: "/receipts/weave-908-report.md".into(),
            title: "weave-908 — daily retro shipped".into(),
            excerpt: "Shipped the daily/weekly retro end to end.".into(),
            cards: vec!["weave-908".into()],
            ts: "2026-07-05T04:00:00+00:00".into(),
            source: "receipt:/receipts/weave-908-report.md".into(),
        }];
        let pack = build_pack(&window(), &[], &[], &[], &[], &receipts, &[]);
        let spec = build_spec(
            &window(),
            "2026-07-05T21:00:05Z",
            &pack,
            stub_narrative(),
            vec![],
            stub_footer(),
            vec![],
        )
        .unwrap();

        assert!(spec.validate().is_ok());
        let Component::Receipts(receipts_component) = &spec.components[4] else {
            panic!("expected receipts component at index 4");
        };
        assert_eq!(receipts_component.items.len(), 1);
        assert_eq!(
            receipts_component.items[0].title,
            "weave-908 — daily retro shipped"
        );
        assert_eq!(receipts_component.items[0].cards, vec!["weave-908"]);

        let Component::Provenance(provenance) = spec.components.last().unwrap() else {
            panic!("expected provenance last");
        };
        assert!(
            provenance.notes.iter().any(|n| n.source == "receipts"),
            "provenance must name the receipts source when it contributed"
        );
    }

    #[test]
    fn feed_events_kind_receipt_is_not_confused_with_a_campaign_receipt() {
        // Regression: feed-post's own KNOWN_KINDS enum reserves "receipt" as
        // a valid feed-post kind (receipt mirrors), which collides with the
        // campaign-receipts collector's "receipt" kind. Dispatch must key on
        // `source` prefix, not `kind` alone, or a feed-sourced receipt
        // mirror silently becomes a fake campaign receipt (and vanishes
        // from the feed-events count) -- found live via the byte-identical
        // regression diff against the pre-refactor baseline.
        let feed_events = vec![FeedEvent {
            ts: "2026-07-05T07:00:00+00:00".into(),
            agent: "release-events".into(),
            kind: "receipt".into(),
            title: "release receipt mirrored".into(),
            body: None,
            links: vec![],
            source: "feed:/day.jsonl".into(),
        }];
        let pack = build_pack(&window(), &[], &[], &[], &feed_events, &[], &[]);
        let spec = build_spec(
            &window(),
            "2026-07-05T21:00:05Z",
            &pack,
            stub_narrative(),
            vec![],
            stub_footer(),
            vec![],
        )
        .unwrap();

        let Component::Receipts(receipts_component) = &spec.components[4] else {
            panic!("expected receipts component at index 4");
        };
        assert!(
            receipts_component.items.is_empty(),
            "a feed-sourced item with kind=receipt must not count as a campaign receipt"
        );
    }

    #[test]
    fn quiet_repo_is_demoted_out_of_the_table_and_gets_no_provenance_note() {
        let activity = vec![RepoActivity {
            repo: "quiet-repo".into(),
            source: "git:/dev/quiet-repo".into(),
            commits: vec![],
            pr_numbers: vec![],
        }];
        let pack = build_pack(&window(), &activity, &[], &[], &[], &[], &[]);
        let spec = build_spec(
            &window(),
            "2026-07-05T21:00:05Z",
            &pack,
            stub_narrative(),
            vec![],
            stub_footer(),
            vec![],
        )
        .unwrap();

        let Component::Table(table) = &spec.components[2] else {
            panic!("expected repo table at index 2");
        };
        assert!(
            table.rows.is_empty(),
            "an all-zero repo must not get a dead table row"
        );
        assert_eq!(
            table.demoted_note.as_deref(),
            Some("1 repo(s) swept with no activity: quiet-repo")
        );

        let Component::Provenance(provenance) = spec.components.last().unwrap() else {
            panic!("expected provenance last");
        };
        assert!(
            !provenance
                .notes
                .iter()
                .any(|n| n.source.contains("quiet-repo")),
            "a repo with zero commits and zero PR references gets no provenance note"
        );
    }

    #[test]
    fn merge_only_pr_counts_toward_the_repo_row_with_zero_commits() {
        let activity = vec![RepoActivity {
            repo: "landmark".into(),
            source: "git:/dev/landmark".into(),
            commits: vec![],
            pr_numbers: vec!["34".into()],
        }];
        let pack = build_pack(&window(), &activity, &[], &[], &[], &[], &[]);
        let spec = build_spec(
            &window(),
            "2026-07-05T21:00:05Z",
            &pack,
            stub_narrative(),
            vec![],
            stub_footer(),
            vec![],
        )
        .unwrap();

        let Component::Table(table) = &spec.components[2] else {
            panic!("expected repo table at index 2");
        };
        assert_eq!(cell_text(&table.rows[0], "commits"), "0");
        assert_eq!(cell_text(&table.rows[0], "prs"), "1");
        assert!(
            cell_list(&table.rows[0], "highlights").is_empty(),
            "a merge-only PR reference has no commit timestamp to become a highlight"
        );

        let Component::Timeline(timeline) = &spec.components[3] else {
            panic!("expected timeline at index 3");
        };
        assert!(
            timeline.entries.is_empty(),
            "a merge-only PR reference is never a timeline entry"
        );
    }

    #[test]
    fn all_sources_share_one_repo_row_and_a_capped_highlight_list() {
        let activity = vec![RepoActivity {
            repo: "landmark".into(),
            source: "git:/dev/landmark".into(),
            commits: vec![RepoCommit {
                hash: "abc123".into(),
                subject: "fix: ground release notes".into(),
                pr_number: None,
                at: "2026-07-05T04:20:00+00:00".into(),
            }],
            pr_numbers: vec![],
        }];
        let card_movements = vec![CardMovement {
            card_id: "landmark-907".into(),
            repo: "landmark".into(),
            event_type: "complete".into(),
            actor: "lane-x".into(),
            at: "2026-07-05T05:00:00+00:00".into(),
            summary: "completed".into(),
            source: "powder:card:landmark-907".into(),
        }];
        let pack = build_pack(&window(), &activity, &card_movements, &[], &[], &[], &[]);
        let spec = build_spec(
            &window(),
            "2026-07-05T21:00:05Z",
            &pack,
            stub_narrative(),
            vec![],
            stub_footer(),
            vec![],
        )
        .unwrap();

        let Component::Table(table) = &spec.components[2] else {
            panic!("expected repo table at index 2");
        };
        assert_eq!(table.rows.len(), 1, "git and powder activity share one row");
        assert_eq!(cell_text(&table.rows[0], "commits"), "1");
        assert_eq!(cell_text(&table.rows[0], "cards_touched"), "1");
        assert_eq!(cell_list(&table.rows[0], "highlights").len(), 2);
    }

    #[test]
    fn bb_and_feed_sources_get_timeline_entries_and_notes() {
        let bb_runs = vec![BbRun {
            id: "run-1".into(),
            task: "build".into(),
            agent: "vulcan".into(),
            state: "done".into(),
            created_at: "2026-07-05T06:00:00+00:00".into(),
            source: "bb:test-plane".into(),
        }];
        let feed_events = vec![FeedEvent {
            ts: "2026-07-05T07:00:00+00:00".into(),
            agent: "linejam-overhaul".into(),
            kind: "shipped".into(),
            title: "linejam overhaul complete".into(),
            body: None,
            links: vec![],
            source: "feed:/day.jsonl".into(),
        }];
        let pack = build_pack(&window(), &[], &[], &bb_runs, &feed_events, &[], &[]);
        let spec = build_spec(
            &window(),
            "2026-07-05T21:00:05Z",
            &pack,
            stub_narrative(),
            vec![],
            stub_footer(),
            vec![],
        )
        .unwrap();

        let Component::Timeline(timeline) = &spec.components[3] else {
            panic!("expected timeline at index 3");
        };
        assert_eq!(timeline.entries.len(), 2);

        let Component::Provenance(provenance) = spec.components.last().unwrap() else {
            panic!("expected provenance last");
        };
        assert!(provenance.notes.iter().any(|n| n.source == "bb"));
        assert!(provenance.notes.iter().any(|n| n.source == "feed"));
    }

    #[test]
    fn moment_items_roll_up_into_the_same_bb_task_bucket_and_get_a_provenance_note() {
        let bb_runs = vec![BbRun {
            id: "run-1".into(),
            task: "build".into(),
            agent: "vulcan".into(),
            state: "done".into(),
            created_at: "2026-07-05T06:00:00+00:00".into(),
            source: "bb:test-plane".into(),
        }];
        let moments = vec![crate::sources::moments::MomentCard {
            run_id: "run-1".into(),
            task: "build".into(),
            class: "failure".into(),
            excerpt: "attempt 1 failed: timeout".into(),
            run_link: "bb runs show run-1 --json".into(),
            created_at: "2026-07-05T06:05:00+00:00".into(),
            source: "moment:test-plane/.bb/moments.db".into(),
        }];
        let pack = build_pack(&window(), &[], &[], &bb_runs, &[], &[], &moments);
        let spec = build_spec(
            &window(),
            "2026-07-05T21:00:05Z",
            &pack,
            stub_narrative(),
            vec![],
            stub_footer(),
            vec![],
        )
        .unwrap();

        let Component::Table(table) = &spec.components[2] else {
            panic!("expected repo table at index 2");
        };
        assert_eq!(
            table.rows.len(),
            1,
            "the moment card rolls into the existing bb:build row, not a new one"
        );
        assert_eq!(cell_text(&table.rows[0], "repo"), "bb:build");

        let Component::Timeline(timeline) = &spec.components[3] else {
            panic!("expected timeline at index 3");
        };
        assert!(
            timeline.entries.iter().any(|e| e.kind == "moment-failure"),
            "a moment card becomes its own timeline entry, kind-prefixed with moment-"
        );

        let Component::Provenance(provenance) = spec.components.last().unwrap() else {
            panic!("expected provenance last");
        };
        assert!(provenance.notes.iter().any(|n| n.source == "moments"));
    }

    #[test]
    fn narrative_and_footer_pass_through_to_the_expected_component_slots() {
        let pack = build_pack(&window(), &[], &[], &[], &[], &[], &[]);
        let narrative = Narrative {
            heading: "What mattered".to_string(),
            status: NarrativeStatus::Ok {
                paragraphs: vec![vec![
                    InlineNode::Text {
                        text: "Landmark shipped a fix today ".to_string(),
                    },
                    InlineNode::Cite {
                        text: "[aaaaaaaaaaaaaaaa]".to_string(),
                        ref_id: "aaaaaaaaaaaaaaaa".to_string(),
                    },
                ]],
            },
        };
        let citations = vec![Citation {
            id: "aaaaaaaaaaaaaaaa".to_string(),
            title: "landmark shipped a fix".to_string(),
        }];
        let footer = Footer {
            judge: "deepseek/deepseek-v4-flash".to_string(),
            gate_status: "passed on attempt 1 of 3".to_string(),
            prompt_version: "weave-fleet-retro-narrative-v1".to_string(),
            pack_schema_version: pack.schema_version.clone(),
            pack_assembly_ms: 42,
        };

        let spec = build_spec(
            &window(),
            "2026-07-05T21:00:05Z",
            &pack,
            narrative,
            citations,
            footer,
            vec![],
        )
        .unwrap();

        let Component::Narrative {
            narrative: rendered_narrative,
            citations: rendered_citations,
        } = &spec.components[1]
        else {
            panic!("expected narrative at index 1, right after hero");
        };
        assert!(matches!(
            rendered_narrative.status,
            NarrativeStatus::Ok { .. }
        ));
        assert_eq!(rendered_citations.len(), 1);

        let Component::Footer(rendered_footer) = &spec.components[5] else {
            panic!("expected footer at index 5, right before provenance");
        };
        assert_eq!(rendered_footer.judge, "deepseek/deepseek-v4-flash");
        assert_eq!(rendered_footer.pack_assembly_ms, 42);

        assert!(matches!(
            spec.components.last(),
            Some(Component::Provenance(_))
        ));
    }
}
