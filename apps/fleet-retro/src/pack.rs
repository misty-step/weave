use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

use crate::sources::bb::BbRun;
use crate::sources::feed::FeedEvent;
use crate::sources::git::{RepoActivity, parse_commit_time};
use crate::sources::moments::MomentCard;
use crate::sources::powder::CardMovement;
use crate::sources::receipts::ReceiptItem;
use crate::window::RetroWindow;

/// Schema version for the versioned intermediate between collectors and
/// everything downstream of them (RetroSpec assembly today; the weave-923
/// synthesis stage and citation gate next). Bump this whenever `EvidenceItem`
/// or `EvidencePack`'s shape changes, the same discipline `spec.rs`'s
/// `CATALOG_VERSION` already uses for the page spec.
pub const EVIDENCE_PACK_SCHEMA_VERSION: &str = "weave.evidence-pack.v1";

/// The time span the pack's items were collected over. A plain
/// `{since, until}` pair, not `RetroWindow` -- the pack is a durable,
/// serializable artifact (it rides the publish path to the shelf as
/// `evidence-pack.json`), while `RetroWindow` carries a `label` that is a
/// CLI/rendering concern, not evidence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackWindow {
    pub since: String,
    pub until: String,
}

/// One dated, sourced, citable fact. Every collector (git, Powder, bb, feed,
/// receipts) projects its native output into zero or more of these -- this
/// is the fixed vocabulary the rest of the pipeline (today: RetroSpec
/// assembly; later: weave-923's synthesis stage and citation gate) is built
/// against, so a new collector or a new report kind extends the pipeline
/// without touching everything downstream of it.
///
/// `refs` is a small ad-hoc tag list (`"repo:landmark"`, `"card:landmark-907"`,
/// `"pr:200"`) rather than per-source-specific struct fields. This keeps the
/// schema genuinely generic -- a consumer that only understands the five
/// fixed fields can still read `title`/`excerpt`/`ts`, while a consumer that
/// needs source-specific structure (RetroSpec assembly's repo rollups, in
/// particular) parses the tags it recognizes and ignores the rest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceItem {
    pub id: String,
    pub ts: String,
    pub source: String,
    pub kind: String,
    pub title: String,
    #[serde(default)]
    pub refs: Vec<String>,
    #[serde(default)]
    pub excerpt: String,
}

impl EvidenceItem {
    /// First ref matching `prefix` (e.g. `"repo:"`), with the prefix
    /// stripped. Used by consumers that need one specific tagged value back
    /// out of the generic `refs` list.
    pub fn ref_value(&self, prefix: &str) -> Option<&str> {
        self.refs.iter().find_map(|r| r.strip_prefix(prefix))
    }

    pub fn ref_values(&self, prefix: &str) -> Vec<&str> {
        self.refs
            .iter()
            .filter_map(|r| r.strip_prefix(prefix))
            .collect()
    }
}

/// A versioned, self-contained snapshot of everything a report run gathered
/// evidence for. Serializes to `evidence-pack.json` beside the rendered
/// report and is meant to be the ground truth a citation gate checks
/// rendered claims against (weave-923) -- so nothing downstream should read
/// git/Powder/bb/feed/receipts directly once a pack exists for the run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidencePack {
    pub schema_version: String,
    pub window: PackWindow,
    pub items: Vec<EvidenceItem>,
}

/// Deterministic id from stable inputs -- NOT a cryptographic hash, just a
/// fixed-seed hash (`DefaultHasher::new()` is documented to start from a
/// fixed state, unlike the randomized seed a `HashMap`'s `RandomState` uses)
/// so the same evidence always gets the same id across runs and processes.
/// That determinism is load-bearing: weave-922's regression contract is a
/// byte-identical rendered page for a fixed window, which only holds if
/// nothing in the pipeline introduces per-run randomness.
fn stable_id(parts: &[&str]) -> String {
    let mut hasher = DefaultHasher::new();
    for part in parts {
        part.hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}

fn git_items(window: &RetroWindow, repo_activity: &[RepoActivity]) -> Vec<EvidenceItem> {
    let mut items = Vec::new();
    for activity in repo_activity {
        // Every discovered repo gets at least one item -- including a quiet
        // repo with zero commits and zero PR references -- so a repo that
        // was swept but had nothing happen in it is present in the pack
        // (an explicit gap, matching this collector's existing "quiet repo"
        // provenance note) rather than silently absent from the evidence.
        items.push(EvidenceItem {
            id: stable_id(&["git", "swept", &activity.repo, &window.since.to_rfc3339()]),
            ts: window.until.to_rfc3339(),
            source: activity.source.clone(),
            kind: "repo-swept".to_string(),
            title: format!("{} swept", activity.repo),
            refs: vec![format!("repo:{}", activity.repo)],
            excerpt: String::new(),
        });

        for commit in &activity.commits {
            let at = parse_commit_time(commit)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_default();
            let mut refs = vec![format!("repo:{}", activity.repo)];
            if let Some(pr) = &commit.pr_number {
                refs.push(format!("pr:{pr}"));
            }
            items.push(EvidenceItem {
                id: stable_id(&["git", "commit", &activity.repo, &commit.hash]),
                ts: at,
                source: activity.source.clone(),
                kind: "commit".to_string(),
                title: commit.subject.clone(),
                refs,
                excerpt: String::new(),
            });
        }

        // `activity.pr_numbers` is the repo's full deduplicated PR-reference
        // set, including PRs only ever named on a "Merge pull request #N"
        // commit -- a commit git.rs deliberately excludes from `commits`, so
        // it has no timestamp of its own to give an item. These synthetic
        // items exist purely to carry that aggregate fact into the pack
        // (`window.until` is a deterministic stand-in ts, not a real event
        // time); the reconstruction below never puts a `pr-ref` item on the
        // timeline, only into the per-repo PR count.
        for pr in &activity.pr_numbers {
            items.push(EvidenceItem {
                id: stable_id(&["git", "pr-ref", &activity.repo, pr]),
                ts: window.until.to_rfc3339(),
                source: activity.source.clone(),
                kind: "pr-ref".to_string(),
                title: format!("PR #{pr} referenced — {}", activity.repo),
                refs: vec![format!("repo:{}", activity.repo), format!("pr:{pr}")],
                excerpt: String::new(),
            });
        }
    }
    items
}

fn powder_items(card_movements: &[CardMovement]) -> Vec<EvidenceItem> {
    card_movements
        .iter()
        .map(|movement| EvidenceItem {
            id: stable_id(&[
                "powder",
                &movement.card_id,
                &movement.event_type,
                &movement.at,
            ]),
            ts: movement.at.clone(),
            source: movement.source.clone(),
            kind: format!("card-{}", movement.event_type),
            title: format!(
                "{} ({}) — {}",
                movement.card_id, movement.actor, movement.summary
            ),
            refs: vec![
                format!("repo:{}", movement.repo),
                format!("card:{}", movement.card_id),
                format!("actor:{}", movement.actor),
            ],
            excerpt: movement.summary.clone(),
        })
        .collect()
}

fn bb_items(bb_runs: &[BbRun]) -> Vec<EvidenceItem> {
    bb_runs
        .iter()
        .map(|run| EvidenceItem {
            id: stable_id(&["bb", &run.id, &run.created_at]),
            ts: run.created_at.clone(),
            source: run.source.clone(),
            kind: "bb-run".to_string(),
            title: format!("{} ({}) — {}", run.id, run.agent, run.state),
            refs: vec![
                format!("task:{}", run.task),
                format!("agent:{}", run.agent),
                format!("state:{}", run.state),
            ],
            excerpt: String::new(),
        })
        .collect()
}

fn feed_items(feed_events: &[FeedEvent]) -> Vec<EvidenceItem> {
    feed_events
        .iter()
        .map(|event| {
            let mut refs = vec![format!("agent:{}", event.agent)];
            for link in &event.links {
                refs.push(format!("link:{}", link.url));
            }
            EvidenceItem {
                id: stable_id(&["feed", &event.ts, &event.kind, &event.title]),
                ts: event.ts.clone(),
                source: event.source.clone(),
                kind: event.kind.clone(),
                title: event.title.clone(),
                refs,
                excerpt: event.body.clone().unwrap_or_default(),
            }
        })
        .collect()
}

fn receipt_items(receipts: &[ReceiptItem]) -> Vec<EvidenceItem> {
    receipts
        .iter()
        .map(|item| {
            let mut refs = vec![format!("path:{}", item.path)];
            refs.extend(item.cards.iter().map(|c| format!("card:{c}")));
            EvidenceItem {
                id: stable_id(&["receipt", &item.path, &item.ts]),
                ts: item.ts.clone(),
                source: item.source.clone(),
                kind: "receipt".to_string(),
                title: item.title.clone(),
                refs,
                excerpt: item.excerpt.clone(),
            }
        })
        .collect()
}

/// `moment-scorer` items (weave-923): Bitterblossom's flight-recorder
/// anomaly scorer (bitterblossom-914) already does the deterministic
/// significance judgment (no model in that path at all) and publishes at
/// most 3 cards/day fleet-wide -- this projection carries that curated,
/// already-scored signal into the pack verbatim rather than re-deriving
/// "was this surprising" from raw run data a second time. `kind` is the
/// scorer's own class name (`failure`/`recovery`/`cost_anomaly`/`surprise`)
/// so a consumer reading `kind` alone still gets a meaningful label; the
/// `moment:` source prefix (not `kind`) is what `assemble.rs` dispatches on,
/// matching every other source family's existing convention.
fn moment_items(moments: &[MomentCard]) -> Vec<EvidenceItem> {
    moments
        .iter()
        .map(|card| EvidenceItem {
            id: stable_id(&["moment", &card.run_id, &card.created_at]),
            ts: card.created_at.clone(),
            source: card.source.clone(),
            kind: card.class.clone(),
            title: format!("{} — {} ({})", card.class, card.task, card.run_id),
            refs: vec![
                format!("task:{}", card.task),
                format!("run:{}", card.run_id),
                format!("class:{}", card.class),
            ],
            excerpt: card.excerpt.clone(),
        })
        .collect()
}

/// Assemble the versioned `EvidencePack` from every collector's already-
/// fetched output. Pure (no I/O): the same "collectors fetch, this just
/// projects" split `assemble::build_spec` already uses, so the projection
/// logic is unit-testable against hand-built fixtures.
#[allow(clippy::too_many_arguments)]
pub fn build_pack(
    window: &RetroWindow,
    repo_activity: &[RepoActivity],
    card_movements: &[CardMovement],
    bb_runs: &[BbRun],
    feed_events: &[FeedEvent],
    receipts: &[ReceiptItem],
    moments: &[MomentCard],
) -> EvidencePack {
    // Deliberately NOT sorted by timestamp: this order (git, then Powder,
    // then bb, then feed, then receipts, then moments -- each already in its
    // own collector's native order) is exactly the order `assemble::build_spec`
    // originally pushed same-timestamp evidence into a repo's highlight
    // list in, before this pack existed. `Vec::sort_by`'s stability means
    // insertion order is the tie-break whenever two items share a
    // timestamp, so re-sorting here would silently reorder tied highlights
    // in the rendered report -- caught live by weave-922's byte-identical
    // regression diff against the pre-refactor baseline.
    let mut items = Vec::new();
    items.extend(git_items(window, repo_activity));
    items.extend(powder_items(card_movements));
    items.extend(bb_items(bb_runs));
    items.extend(feed_items(feed_events));
    items.extend(receipt_items(receipts));
    items.extend(moment_items(moments));

    EvidencePack {
        schema_version: EVIDENCE_PACK_SCHEMA_VERSION.to_string(),
        window: PackWindow {
            since: window.since.to_rfc3339(),
            until: window.until.to_rfc3339(),
        },
        items,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::feed::FeedLink;
    use crate::sources::git::{RepoCommit, ts};

    fn window() -> RetroWindow {
        RetroWindow::custom(ts(2026, 7, 4, 21, 0, 0), ts(2026, 7, 5, 21, 0, 0)).unwrap()
    }

    #[test]
    fn stable_id_is_deterministic_across_calls() {
        let a = stable_id(&["git", "commit", "landmark", "abc123"]);
        let b = stable_id(&["git", "commit", "landmark", "abc123"]);
        assert_eq!(a, b);
    }

    #[test]
    fn stable_id_differs_for_different_inputs() {
        let a = stable_id(&["git", "commit", "landmark", "abc123"]);
        let b = stable_id(&["git", "commit", "landmark", "def456"]);
        assert_ne!(a, b);
    }

    #[test]
    fn every_repo_gets_a_swept_item_even_with_no_activity() {
        let activity = vec![RepoActivity {
            repo: "quiet-repo".into(),
            source: "git:/dev/quiet-repo".into(),
            commits: vec![],
            pr_numbers: vec![],
        }];
        let pack = build_pack(&window(), &activity, &[], &[], &[], &[], &[]);
        assert_eq!(pack.items.len(), 1);
        assert_eq!(pack.items[0].kind, "repo-swept");
        assert_eq!(pack.items[0].ref_value("repo:"), Some("quiet-repo"));
    }

    #[test]
    fn merge_only_pr_becomes_a_pr_ref_item_with_no_commit() {
        let activity = vec![RepoActivity {
            repo: "landmark".into(),
            source: "git:/dev/landmark".into(),
            commits: vec![],
            pr_numbers: vec!["34".into()],
        }];
        let pack = build_pack(&window(), &activity, &[], &[], &[], &[], &[]);
        let pr_refs: Vec<_> = pack.items.iter().filter(|i| i.kind == "pr-ref").collect();
        assert_eq!(pr_refs.len(), 1);
        assert_eq!(pr_refs[0].ref_value("pr:"), Some("34"));
    }

    #[test]
    fn all_six_collectors_emit_items() {
        let activity = vec![RepoActivity {
            repo: "landmark".into(),
            source: "git:/dev/landmark".into(),
            commits: vec![RepoCommit {
                hash: "abc123".into(),
                subject: "fix: ground release notes (#200)".into(),
                pr_number: Some("200".into()),
                at: "2026-07-05T04:20:00+00:00".into(),
            }],
            pr_numbers: vec!["200".into()],
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
            links: vec![FeedLink {
                label: "pr".into(),
                url: "https://example.invalid/pr/300".into(),
            }],
            source: "feed:/day.jsonl".into(),
        }];
        let receipts = vec![ReceiptItem {
            path: "/receipts/weave-908-report.md".into(),
            title: "weave-908 shipped".into(),
            excerpt: "Shipped the retro.".into(),
            cards: vec!["weave-908".into()],
            ts: "2026-07-05T08:00:00+00:00".into(),
            source: "receipt:/receipts/weave-908-report.md".into(),
        }];
        let moments = vec![MomentCard {
            run_id: "run-1".into(),
            task: "build".into(),
            class: "failure".into(),
            excerpt: "attempt 1 failed: timeout".into(),
            run_link: "bb runs show run-1 --json".into(),
            created_at: "2026-07-05T09:00:00+00:00".into(),
            source: "moment:test-plane/.bb/moments.db".into(),
        }];

        let pack = build_pack(
            &window(),
            &activity,
            &card_movements,
            &bb_runs,
            &feed_events,
            &receipts,
            &moments,
        );

        assert_eq!(pack.schema_version, EVIDENCE_PACK_SCHEMA_VERSION);
        for kind in [
            "repo-swept",
            "commit",
            "pr-ref",
            "card-complete",
            "bb-run",
            "shipped",
            "receipt",
            "failure",
        ] {
            assert!(
                pack.items.iter().any(|i| i.kind == kind),
                "expected an item of kind {kind}"
            );
        }
    }

    #[test]
    fn a_single_collectors_items_preserve_its_own_native_order() {
        // bb.rs's own `extract_runs_in_window` already sorts by
        // `created_at`; the pack must not disturb that.
        let bb_runs = vec![
            BbRun {
                id: "run-1".into(),
                task: "a".into(),
                agent: "x".into(),
                state: "done".into(),
                created_at: "2026-07-05T01:00:00+00:00".into(),
                source: "bb:p".into(),
            },
            BbRun {
                id: "run-2".into(),
                task: "b".into(),
                agent: "x".into(),
                state: "done".into(),
                created_at: "2026-07-05T12:00:00+00:00".into(),
                source: "bb:p".into(),
            },
        ];
        let pack = build_pack(&window(), &[], &[], &bb_runs, &[], &[], &[]);
        let timestamps: Vec<&str> = pack.items.iter().map(|i| i.ts.as_str()).collect();
        assert_eq!(
            timestamps,
            vec!["2026-07-05T01:00:00+00:00", "2026-07-05T12:00:00+00:00"]
        );
    }

    #[test]
    fn moment_item_carries_the_scorers_own_class_as_kind_and_task_run_class_as_refs() {
        let moments = vec![MomentCard {
            run_id: "run-42".into(),
            task: "review".into(),
            class: "surprise".into(),
            excerpt: "guard event circuit_breaker: 3 trips".into(),
            run_link: "bb runs show run-42 --json".into(),
            created_at: "2026-07-05T10:00:00+00:00".into(),
            source: "moment:plane/.bb/moments.db".into(),
        }];
        let pack = build_pack(&window(), &[], &[], &[], &[], &[], &moments);

        assert_eq!(pack.items.len(), 1);
        let item = &pack.items[0];
        assert_eq!(item.kind, "surprise");
        assert_eq!(item.excerpt, "guard event circuit_breaker: 3 trips");
        assert_eq!(item.ref_value("task:"), Some("review"));
        assert_eq!(item.ref_value("run:"), Some("run-42"));
        assert_eq!(item.ref_value("class:"), Some("surprise"));
        assert!(item.source.starts_with("moment:"));
    }

    #[test]
    fn items_preserve_collector_emission_order_git_then_powder_then_bb_then_feed_then_receipts() {
        // NOT timestamp order across sources: `assemble::build_spec` relies
        // on this exact cross-source order to reproduce the pre-refactor
        // tie-break behavior for same-timestamp highlights (Vec::sort_by is
        // stable, so insertion order is the tie-break). A pack that
        // re-sorted globally by timestamp would silently reorder tied
        // highlights in the rendered report -- caught live by weave-922's
        // byte-identical regression diff against the pre-refactor baseline.
        let card_movements = vec![CardMovement {
            card_id: "landmark-907".into(),
            repo: "landmark".into(),
            event_type: "complete".into(),
            actor: "lane-x".into(),
            at: "2026-07-05T09:00:00+00:00".into(),
            summary: "completed".into(),
            source: "powder:card:landmark-907".into(),
        }];
        let bb_runs = vec![BbRun {
            id: "run-1".into(),
            task: "build".into(),
            agent: "vulcan".into(),
            state: "done".into(),
            created_at: "2026-07-05T08:00:00+00:00".into(),
            source: "bb:p".into(),
        }];
        let feed_events = vec![FeedEvent {
            ts: "2026-07-05T07:00:00+00:00".into(),
            agent: "x".into(),
            kind: "shipped".into(),
            title: "shipped".into(),
            body: None,
            links: vec![],
            source: "feed:/d.jsonl".into(),
        }];
        let pack = build_pack(
            &window(),
            &[],
            &card_movements,
            &bb_runs,
            &feed_events,
            &[],
            &[],
        );
        let kinds: Vec<&str> = pack.items.iter().map(|i| i.kind.as_str()).collect();
        assert_eq!(kinds, vec!["card-complete", "bb-run", "shipped"]);
    }
}
