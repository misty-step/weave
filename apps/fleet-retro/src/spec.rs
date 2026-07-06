use anyhow::{Result, bail};
use serde::Serialize;

/// Catalog version for the retro's page spec. Bump this whenever the
/// `Component` catalog changes shape so a renderer can refuse a spec it
/// does not understand instead of silently mis-rendering it. This is the
/// same spirit as glance-gen's `PageSpec`/`catalog_version`
/// (glance-next/crates/glance-gen/src/spec.rs) -- prior art borrowed per
/// weave-908's card guidance while misty-step-911 (which repo owns the
/// shared report-rendering primitive) is unresolved. The renderer seam
/// (`render::render_html`) only depends on this module, so swapping in a
/// shared primitive later means retargeting one function, not rewriting the
/// collectors.
pub const CATALOG_VERSION: &str = "weave-fleet-retro-002";

/// Spec-first: every collector's output gets assembled into this typed
/// structure *before* any HTML is produced. The renderer is a pure function
/// of `RetroSpec` -- no collector logic leaks into `render.rs`, and a
/// `RetroSpec` can be golden-tested independent of live git/Powder/bb state.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RetroSpec {
    pub catalog_version: String,
    pub title: String,
    pub window_label: String,
    pub since: String,
    pub until: String,
    pub generated_at: String,
    pub components: Vec<Component>,
}

impl RetroSpec {
    pub fn validate(&self) -> Result<()> {
        if self.catalog_version != CATALOG_VERSION {
            bail!(
                "catalog_version must be {CATALOG_VERSION}, got {}",
                self.catalog_version
            );
        }
        if self.title.trim().is_empty() {
            bail!("title is required");
        }
        let Some(Component::Hero(_)) = self.components.first() else {
            bail!("hero must be the first component");
        };
        if !self
            .components
            .iter()
            .any(|c| matches!(c, Component::Provenance(_)))
        {
            bail!(
                "provenance is required -- every retro claim must be traceable to a named source"
            );
        }
        let provenance_is_last = matches!(self.components.last(), Some(Component::Provenance(_)));
        if !provenance_is_last {
            bail!("provenance must be the last component");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Component {
    Hero(Hero),
    Narrative(Narrative),
    StatCallouts(StatCallouts),
    RepoActivityTable(RepoActivityTable),
    Timeline(Timeline),
    Receipts(Receipts),
    Footer(Footer),
    Provenance(Provenance),
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Hero {
    pub headline: String,
    pub subhead: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct StatCallouts {
    pub items: Vec<StatCallout>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct StatCallout {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RepoActivityTable {
    pub rows: Vec<RepoActivityRow>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct RepoActivityRow {
    pub repo: String,
    pub commits: usize,
    pub prs: usize,
    pub cards_touched: usize,
    pub highlights: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Timeline {
    pub entries: Vec<TimelineEntry>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TimelineEntry {
    pub at: String,
    pub repo: String,
    pub kind: String,
    pub summary: String,
    pub source: String,
    pub link: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Receipts {
    pub items: Vec<ReceiptRow>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ReceiptRow {
    pub title: String,
    pub excerpt: String,
    pub path: String,
    pub cards: Vec<String>,
    pub at: String,
}

/// The model-synthesized "what mattered" section (weave-923): significance-
/// ranked prose over the window's `EvidencePack`, every factual claim
/// carrying an inline `[id]` citation to a pack item. `Ok` only after the
/// citation gate (`citation_gate.rs`) has accepted the text -- a citation
/// gate rejection or an unreachable model degrades to `FailedOpen` with a
/// visible reason, never a silently-empty or half-cited narrative.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Narrative {
    pub status: NarrativeStatus,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum NarrativeStatus {
    Ok {
        blocks: Vec<String>,
        citations: Vec<Citation>,
    },
    FailedOpen {
        reason: String,
    },
}

/// One pack item the narrative cited, carried alongside the narrative text
/// so `render.rs` can turn an inline `[id]` token into a hover/tap link
/// without needing direct access to the full `EvidencePack` (the renderer
/// stays a pure function of `RetroSpec` alone).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Citation {
    pub id: String,
    pub title: String,
}

/// Diagnosability metadata for the synthesis stage, rendered as a visible
/// report footer (oracle-ruled binding, weave-923 card comments): which
/// model judged the narrative (or "none" on fail-open), what the citation
/// gate decided and on which attempt, which prompt/pack schema versions
/// were in play, and how long pack assembly took -- the named falsifier for
/// pull-federation (if this ever exceeds report cadence, the fix is a
/// cached pull snapshot, not event-sourcing).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Footer {
    pub judge: String,
    pub gate_status: String,
    pub prompt_version: String,
    pub pack_schema_version: String,
    pub pack_assembly_ms: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Provenance {
    pub notes: Vec<ProvenanceNote>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProvenanceNote {
    pub source: String,
    pub note: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_valid_spec() -> RetroSpec {
        RetroSpec {
            catalog_version: CATALOG_VERSION.to_string(),
            title: "Fleet retro".to_string(),
            window_label: "daily".to_string(),
            since: "2026-07-04T21:00:00Z".to_string(),
            until: "2026-07-05T21:00:00Z".to_string(),
            generated_at: "2026-07-05T21:00:05Z".to_string(),
            components: vec![
                Component::Hero(Hero {
                    headline: "h".into(),
                    subhead: "s".into(),
                }),
                Component::Provenance(Provenance { notes: vec![] }),
            ],
        }
    }

    #[test]
    fn valid_minimal_spec_passes() {
        assert!(minimal_valid_spec().validate().is_ok());
    }

    #[test]
    fn rejects_wrong_catalog_version() {
        let mut spec = minimal_valid_spec();
        spec.catalog_version = "some-other-version".into();
        assert!(spec.validate().is_err());
    }

    #[test]
    fn rejects_missing_hero() {
        let mut spec = minimal_valid_spec();
        spec.components.remove(0);
        assert!(spec.validate().is_err());
    }

    #[test]
    fn rejects_missing_provenance() {
        let mut spec = minimal_valid_spec();
        spec.components.pop();
        assert!(spec.validate().is_err());
    }

    #[test]
    fn rejects_provenance_not_last() {
        let mut spec = minimal_valid_spec();
        spec.components
            .push(Component::StatCallouts(StatCallouts { items: vec![] }));
        assert!(spec.validate().is_err());
    }
}
