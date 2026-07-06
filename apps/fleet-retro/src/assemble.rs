use std::collections::BTreeMap;

use crate::sources::SourceNote;
use crate::sources::bb::BbRun;
use crate::sources::feed::FeedEvent;
use crate::sources::git::{RepoActivity, parse_commit_time};
use crate::sources::powder::CardMovement;
use crate::sources::receipts::ReceiptItem;
use crate::spec::*;
use crate::window::RetroWindow;

const TIMELINE_LIMIT: usize = 300;
const HIGHLIGHTS_PER_REPO: usize = 4;

/// Pure assembly: turn already-collected evidence into a validated
/// `RetroSpec`. Nothing in here does I/O -- every input is data a collector
/// already fetched, so this function (and therefore the report's shape) is
/// fully unit-testable against hand-built fixtures, independent of live
/// git/Powder/bb state.
#[allow(clippy::too_many_arguments)]
pub fn build_spec(
    window: &RetroWindow,
    generated_at: &str,
    repo_activity: &[RepoActivity],
    card_movements: &[CardMovement],
    bb_runs: &[BbRun],
    feed_events: &[FeedEvent],
    receipts: &[ReceiptItem],
    mut notes: Vec<SourceNote>,
) -> anyhow::Result<RetroSpec> {
    #[derive(Default)]
    struct RepoRollup {
        commits: usize,
        prs: usize,
        cards: std::collections::BTreeSet<String>,
        highlights: Vec<(String, String)>, // (at, text) for sort-then-truncate
    }

    let mut repos: BTreeMap<String, RepoRollup> = BTreeMap::new();
    let mut timeline: Vec<TimelineEntry> = Vec::new();
    let mut total_commits = 0usize;
    let mut all_prs: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut all_cards: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

    for activity in repo_activity {
        let rollup = repos.entry(activity.repo.clone()).or_default();
        rollup.commits += activity.commits.len();
        rollup.prs = activity.pr_numbers.len();
        total_commits += activity.commits.len();
        for pr in &activity.pr_numbers {
            all_prs.insert(format!("{}#{}", activity.repo, pr));
        }
        for commit in &activity.commits {
            let at = parse_commit_time(commit)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default();
            rollup.highlights.push((at.clone(), commit.subject.clone()));
            timeline.push(TimelineEntry {
                at,
                repo: activity.repo.clone(),
                kind: "commit".to_string(),
                summary: commit.subject.clone(),
                source: activity.source.clone(),
                link: None,
            });
        }
        if activity.commits.is_empty() && activity.pr_numbers.is_empty() {
            continue;
        }
        notes.push(SourceNote::new(
            activity.source.clone(),
            format!(
                "{} commits, {} PR reference(s) in window",
                activity.commits.len(),
                activity.pr_numbers.len()
            ),
        ));
    }

    for movement in card_movements {
        all_cards.insert(movement.card_id.clone());
        let rollup = repos.entry(movement.repo.clone()).or_default();
        rollup.cards.insert(movement.card_id.clone());
        rollup.highlights.push((
            movement.at.clone(),
            format!("{}: {}", movement.card_id, movement.summary),
        ));
        timeline.push(TimelineEntry {
            at: movement.at.clone(),
            repo: movement.repo.clone(),
            kind: format!("card-{}", movement.event_type),
            summary: format!(
                "{} ({}) — {}",
                movement.card_id, movement.actor, movement.summary
            ),
            source: movement.source.clone(),
            link: None,
        });
    }

    for run in bb_runs {
        let rollup = repos.entry(format!("bb:{}", run.task)).or_default();
        rollup.highlights.push((
            run.created_at.clone(),
            format!("{} run {}", run.task, run.state),
        ));
        timeline.push(TimelineEntry {
            at: run.created_at.clone(),
            repo: run.task.clone(),
            kind: "bb-run".to_string(),
            summary: format!("{} ({}) — {}", run.id, run.agent, run.state),
            source: run.source.clone(),
            link: None,
        });
    }

    for event in feed_events {
        let repo_key = derive_feed_repo(event);
        let rollup = repos.entry(repo_key.clone()).or_default();
        rollup
            .highlights
            .push((event.ts.clone(), event.title.clone()));
        let link = event.links.first().map(|l| l.url.clone());
        timeline.push(TimelineEntry {
            at: event.ts.clone(),
            repo: repo_key,
            kind: event.kind.clone(),
            summary: event.title.clone(),
            source: event.source.clone(),
            link,
        });
    }

    if !bb_runs.is_empty() {
        notes.push(SourceNote::new(
            "bb",
            format!("{} plane run(s) in window", bb_runs.len()),
        ));
    }
    if !feed_events.is_empty() {
        notes.push(SourceNote::new(
            "feed",
            format!("{} feed event(s) in window", feed_events.len()),
        ));
    }
    if !card_movements.is_empty() {
        notes.push(SourceNote::new(
            "powder",
            format!(
                "{} card movement(s) across {} card(s) in window",
                card_movements.len(),
                all_cards.len()
            ),
        ));
    }
    if !receipts.is_empty() {
        notes.push(SourceNote::new(
            "receipts",
            format!("{} campaign receipt(s) in window", receipts.len()),
        ));
    }

    let receipt_rows: Vec<ReceiptRow> = receipts
        .iter()
        .map(|item| ReceiptRow {
            title: item.title.clone(),
            excerpt: item.excerpt.clone(),
            path: item.path.clone(),
            cards: item.cards.clone(),
            at: item.ts.clone(),
        })
        .collect();

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

    let rows: Vec<RepoActivityRow> = repos
        .into_iter()
        .map(|(repo, mut rollup)| {
            rollup.highlights.sort_by(|a, b| b.0.cmp(&a.0));
            let highlights = rollup
                .highlights
                .into_iter()
                .take(HIGHLIGHTS_PER_REPO)
                .map(|(_, text)| text)
                .collect();
            RepoActivityRow {
                repo,
                commits: rollup.commits,
                prs: rollup.prs,
                cards_touched: rollup.cards.len(),
                highlights,
            }
        })
        .collect();

    let notes_view: Vec<ProvenanceNote> = notes
        .into_iter()
        .map(|n| ProvenanceNote {
            source: n.source,
            note: n.note,
        })
        .collect();

    let stat_items = vec![
        StatCallout {
            label: "Commits".into(),
            value: total_commits.to_string(),
        },
        StatCallout {
            label: "PRs".into(),
            value: all_prs.len().to_string(),
        },
        StatCallout {
            label: "Cards touched".into(),
            value: all_cards.len().to_string(),
        },
        StatCallout {
            label: "bb runs".into(),
            value: bb_runs.len().to_string(),
        },
        StatCallout {
            label: "Feed events".into(),
            value: feed_events.len().to_string(),
        },
        StatCallout {
            label: "Receipts".into(),
            value: receipts.len().to_string(),
        },
        StatCallout {
            label: "Window".into(),
            value: format!("{}h", window.duration_hours()),
        },
    ];

    let spec = RetroSpec {
        catalog_version: CATALOG_VERSION.to_string(),
        title: format!("Fleet retro — {}", window.label),
        window_label: window.label.clone(),
        since: window.since.to_rfc3339(),
        until: window.until.to_rfc3339(),
        generated_at: generated_at.to_string(),
        components: vec![
            Component::Hero(Hero {
                headline: format!("Fleet retro — {}", window.label),
                subhead: format!(
                    "{} → {} ({}h window)",
                    window.since.to_rfc3339(),
                    window.until.to_rfc3339(),
                    window.duration_hours()
                ),
            }),
            Component::StatCallouts(StatCallouts { items: stat_items }),
            Component::RepoActivityTable(RepoActivityTable { rows }),
            Component::Timeline(Timeline { entries: timeline }),
            Component::Receipts(Receipts {
                items: receipt_rows,
            }),
            Component::Provenance(Provenance { notes: notes_view }),
        ],
    };
    spec.validate()?;
    Ok(spec)
}

fn derive_feed_repo(event: &FeedEvent) -> String {
    if event.agent != "fleet-digest" && event.agent != "fleet-retro" && !event.agent.is_empty() {
        return event.agent.clone();
    }
    "fleet".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::git::{RepoCommit, ts};

    fn window() -> RetroWindow {
        RetroWindow::custom(ts(2026, 7, 4, 21, 0, 0), ts(2026, 7, 5, 21, 0, 0)).unwrap()
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
        let spec = build_spec(
            &window(),
            "2026-07-05T21:00:05Z",
            &activity,
            &[],
            &[],
            &[],
            &[],
            vec![],
        )
        .unwrap();

        assert!(spec.validate().is_ok());
        let Component::RepoActivityTable(table) = &spec.components[2] else {
            panic!("expected repo table at index 2");
        };
        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.rows[0].repo, "landmark");
        assert_eq!(table.rows[0].commits, 1);
        assert_eq!(table.rows[0].prs, 1);

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
        let spec = build_spec(
            &window(),
            "2026-07-05T21:00:05Z",
            &[],
            &[],
            &[],
            &[],
            &[],
            vec![],
        )
        .unwrap();
        assert!(spec.validate().is_ok());
        let Component::StatCallouts(stats) = &spec.components[1] else {
            panic!("expected stats at index 1");
        };
        assert_eq!(stats.items[0].value, "0");
    }

    #[test]
    fn receipts_become_a_dedicated_component_and_a_provenance_note() {
        use crate::sources::receipts::ReceiptItem;

        let receipts = vec![ReceiptItem {
            path: "/receipts/weave-908-report.md".into(),
            title: "weave-908 — daily retro shipped".into(),
            excerpt: "Shipped the daily/weekly retro end to end.".into(),
            cards: vec!["weave-908".into()],
            ts: "2026-07-05T04:00:00+00:00".into(),
            source: "receipt:/receipts/weave-908-report.md".into(),
        }];

        let spec = build_spec(
            &window(),
            "2026-07-05T21:00:05Z",
            &[],
            &[],
            &[],
            &[],
            &receipts,
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
}
