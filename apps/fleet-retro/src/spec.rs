use anyhow::{Result, bail};
use serde::Serialize;

use glance_catalog::structural::{Hero, Table, Timeline};

/// Catalog version for the retro's page spec. Bump this whenever the
/// `Component` catalog changes shape so a renderer can refuse a spec it
/// does not understand instead of silently mis-rendering it.
///
/// aesthetic-926: this crate's structural components (`Hero`, `Narrative`,
/// `Table`, `Timeline`) now come from the shared `glance_catalog` crate
/// instead of a second hand-rolled `RetroSpec`-only definition -- the
/// consolidation the epic exists to complete. `Footer`, `Receipts`, and
/// `Provenance` stay local: no second consumer independently built an
/// equivalent, so folding them into the shared catalog would be
/// arbitrating a winner where there is nothing to merge with, not
/// consolidating real convergent design.
pub const CATALOG_VERSION: &str = "weave-fleet-retro-003";

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
        for component in &self.components {
            component
                .catalog()
                .map(|catalog_component| catalog_component.validate())
                .transpose()
                .map_err(|error| anyhow::anyhow!(error.to_string()))?;
        }
        Ok(())
    }
}

/// fleet-retro's own component union: the shared catalog's structural
/// primitives (`Hero`/`Table`/`Timeline` reused directly, `Narrative`
/// reused for its paragraphs plus a local citations index for the "cited
/// evidence" appendix `render.rs` still owns) alongside three
/// report-specific extension sections that never converged with a second
/// implementation.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Component {
    Hero(Hero),
    Narrative {
        narrative: glance_catalog::structural::Narrative,
        citations: Vec<Citation>,
    },
    Table(Table),
    Timeline(Timeline),
    Receipts(Receipts),
    Footer(Footer),
    Provenance(Provenance),
}

impl Component {
    /// The subset of variants backed directly by a shared catalog type --
    /// used to run the catalog's own `validate()` over them instead of
    /// re-implementing that logic locally. `Narrative`'s inner
    /// `glance_catalog::structural::Narrative` is included; `citations` is
    /// fleet-retro's own local data, out of the catalog's concern.
    fn catalog(&self) -> Option<&dyn CatalogValidatable> {
        match self {
            Component::Hero(hero) => Some(hero),
            Component::Narrative { narrative, .. } => Some(narrative),
            Component::Table(table) => Some(table),
            Component::Timeline(timeline) => Some(timeline),
            Component::Receipts(_) | Component::Footer(_) | Component::Provenance(_) => None,
        }
    }
}

trait CatalogValidatable {
    fn validate(&self) -> Result<(), glance_catalog::CatalogError>;
}

impl CatalogValidatable for Hero {
    fn validate(&self) -> Result<(), glance_catalog::CatalogError> {
        Hero::validate(self)
    }
}
impl CatalogValidatable for glance_catalog::structural::Narrative {
    fn validate(&self) -> Result<(), glance_catalog::CatalogError> {
        glance_catalog::structural::Narrative::validate(self)
    }
}
impl CatalogValidatable for Table {
    fn validate(&self) -> Result<(), glance_catalog::CatalogError> {
        Table::validate(self)
    }
}
impl CatalogValidatable for Timeline {
    fn validate(&self) -> Result<(), glance_catalog::CatalogError> {
        Timeline::validate(self)
    }
}

/// One pack item the narrative cited, carried alongside the narrative so
/// `render.rs` can render fleet-retro's local "cited evidence" appendix
/// (an anchor-linked list beneath the narrative) without needing direct
/// access to the full `EvidencePack`. This is fleet-retro-specific
/// presentation, not part of the shared catalog's `Narrative` type -- no
/// second consumer has an equivalent appendix to merge with.
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
    use glance_catalog::inline::InlineNode;
    use glance_catalog::structural::{Narrative, NarrativeStatus};

    fn text(s: &str) -> Vec<InlineNode> {
        vec![InlineNode::Text { text: s.into() }]
    }

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
                    title: "h".into(),
                    summary: text("s"),
                    stats: vec![],
                    image_intent: None,
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
        spec.components.insert(
            1,
            Component::Table(Table {
                heading: "t".into(),
                columns: vec![],
                rows: vec![],
                empty_note: None,
                demoted_note: None,
            }),
        );
        // an empty-columns table is itself invalid, so insert provenance
        // twice instead to isolate the "not last" rule from table validity
        spec.components.remove(1);
        spec.components.push(Component::Footer(Footer {
            judge: "none".into(),
            gate_status: "n/a".into(),
            prompt_version: "n/a".into(),
            pack_schema_version: "n/a".into(),
            pack_assembly_ms: 0,
        }));
        assert!(spec.validate().is_err());
    }

    #[test]
    fn rejects_a_spec_whose_hero_fails_catalog_validation() {
        let mut spec = minimal_valid_spec();
        spec.components[0] = Component::Hero(Hero {
            title: String::new(), // invalid: catalog requires a non-empty title
            summary: text("s"),
            stats: vec![],
            image_intent: None,
        });
        assert!(spec.validate().is_err());
    }

    #[test]
    fn narrative_component_carries_a_catalog_narrative_plus_local_citations() {
        let mut spec = minimal_valid_spec();
        spec.components.insert(
            1,
            Component::Narrative {
                narrative: Narrative {
                    heading: "What mattered".into(),
                    status: NarrativeStatus::Ok {
                        paragraphs: vec![text("Landmark shipped a fix today [aaaaaaaaaaaaaaaa].")],
                    },
                },
                citations: vec![Citation {
                    id: "aaaaaaaaaaaaaaaa".into(),
                    title: "landmark shipped a fix".into(),
                }],
            },
        );
        assert!(spec.validate().is_ok());
    }
}
