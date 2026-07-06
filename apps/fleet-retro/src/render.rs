use chrono::{DateTime, Utc};
use glance_catalog::render::RenderContext;
use glance_catalog::structural::NarrativeStatus;

use crate::spec::{Component, Receipts, RetroSpec};

/// Page-specific CSS layered over the vendored `aesthetic.css`, in the same
/// pattern bridge.py uses (base kit + a small `<style>` override block).
/// This block carries ONLY page-shell layout (max-width, spacing, heading
/// resets) plus the two sections `glance_catalog::render` does not own
/// (`Footer`/`Receipts`/`Provenance` -- fleet-retro-specific extension
/// components, see `spec.rs`). Every primitive `glance_catalog::render`
/// itself renders (`Hero`/`Narrative`/`Table`/`Timeline`) rides that crate's
/// `.ae-hero`/`.ae-section`/`.ae-table`/`.ae-trail` classes -- consolidated
/// off this file's pre-aesthetic-926 `.retro-hero`/`.retro-section` names
/// (aesthetic-926: MERGE don't arbitrate).
const RETRO_CSS: &str = r#"
.retro-page{max-width:var(--ae-measure-wide);margin:0 auto;padding:var(--ae-space-6) var(--ae-space-4);}
.retro-page h1,.retro-page h2,.retro-page h3{font-size:16px;font-weight:var(--ae-w-medium);margin:0;}
.ae-hero p{margin:var(--ae-space-1) 0 var(--ae-space-4);}
.ae-section{margin:var(--ae-space-6) 0;}
.ae-section>h2{margin-bottom:var(--ae-space-3);}
/* glance_catalog::render emits .ae-cite for every InlineNode::Cite link
   (see crate::inline) with no styling of its own -- the shared crate only
   owns markup, not this page's chrome. Restyled here at the law's 13px
   chrome-register exception (the pre-aesthetic-926 .retro-narrative
   .citation rule this replaces used 11px, itself a pre-existing law
   violation this migration does not need to preserve). */
.ae-cite{font-family:var(--ae-font-mono);font-size:13px;color:var(--ae-accent);text-decoration:none;}
.retro-cited-evidence{list-style:none;margin:var(--ae-space-3) 0 0;padding:0;font-size:13px;color:var(--ae-ink-muted);}
.retro-cited-evidence li{padding:.15rem 0;}
.retro-cited-evidence code{font-size:11px;}
.retro-footer{font-size:13px;color:var(--ae-ink-faint);border-top:1px solid var(--ae-line);padding-top:var(--ae-space-3);}
.retro-provenance ul{margin:0;padding-left:1.1rem;font-size:13px;color:var(--ae-ink-muted);}
"#;

fn esc(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Resolves a narrative citation's opaque `ref_id` (the pack item's 16-hex
/// id, see `citation_gate.rs`) to a local anchor into the "cited evidence"
/// list this file renders beneath the narrative -- fleet-retro's own
/// citation scheme, distinct from glance-gen's path:lines-to-GitHub-blob
/// scheme, sharing only `glance_catalog::inline::InlineNode::Cite`'s type
/// (see that module's doc comment for why `ref_id` is opaque).
fn cite_href(ref_id: &str) -> String {
    format!("#cite-{ref_id}")
}

fn render_receipts(receipts: &Receipts) -> String {
    if receipts.items.is_empty() {
        return r#"<section class="ae-section"><h2>Receipts</h2><p class="ae-dim">No campaign receipts in this window.</p></section>"#.to_string();
    }
    let items: String = receipts
        .items
        .iter()
        .map(|item| {
            let cards: String = item
                .cards
                .iter()
                .map(|c| format!(r#"<span class="ae-tag">{}</span>"#, esc(c)))
                .collect();
            format!(
                r#"<article class="ae-wall-card"><div><div class="ae-wall-head"><span class="ae-strong">{}</span></div><div class="ae-wall-meta">{}</div>{}</div><div class="ae-wall-figure"><span class="ae-num">{}</span></div></article>"#,
                esc(&item.title),
                esc(&item.excerpt),
                cards,
                esc(&glance_catalog::relative_time(
                    &item.at,
                    Utc::now()
                ))
            )
        })
        .collect();
    format!(
        r#"<section class="ae-section"><h2>Receipts</h2><div class="ae-wall">{items}</div></section>"#
    )
}

fn render_footer(footer: &crate::spec::Footer) -> String {
    format!(
        r#"<footer class="retro-footer"><details><summary class="ae-dim">diagnostics</summary><p class="ae-dim">judge: <code>{}</code> · gate: <code>{}</code> · prompt: <code>{}</code> · pack: <code>{}</code> · pack assembly: {}ms</p></details></footer>"#,
        esc(&footer.judge),
        esc(&footer.gate_status),
        esc(&footer.prompt_version),
        esc(&footer.pack_schema_version),
        footer.pack_assembly_ms
    )
}

fn render_provenance(provenance: &crate::spec::Provenance) -> String {
    let items: String = provenance
        .notes
        .iter()
        .map(|note| {
            format!(
                "<li><code>{}</code> — {}</li>",
                esc(&note.source),
                esc(&note.note)
            )
        })
        .collect();
    format!(
        r#"<section class="ae-section retro-provenance"><h2>Sources</h2><ul>{items}</ul></section>"#
    )
}

fn render_narrative_cited_evidence(citations: &[crate::spec::Citation]) -> String {
    if citations.is_empty() {
        return String::new();
    }
    let cited: String = citations
        .iter()
        .map(|c| {
            format!(
                r#"<li id="cite-{}"><code>[{}]</code> {}</li>"#,
                esc(&c.id),
                esc(&c.id),
                esc(&c.title)
            )
        })
        .collect();
    format!(r#"<ul class="retro-cited-evidence">{cited}</ul>"#)
}

fn render_component(component: &Component, ctx: &RenderContext<'_>) -> String {
    match component {
        Component::Hero(hero) => {
            glance_catalog::render_component(&glance_catalog::Component::Hero(hero.clone()), ctx)
        }
        Component::Narrative {
            narrative,
            citations,
        } => {
            let mut html = glance_catalog::render_component(
                &glance_catalog::Component::Narrative(narrative.clone()),
                ctx,
            );
            if matches!(narrative.status, NarrativeStatus::Ok { .. }) {
                // Insert the cited-evidence list just before the closing
                // </section> tag glance_catalog::render's render_narrative
                // emits, matching this file's pre-aesthetic-926 layout
                // exactly (paragraphs, then the appendix list, in one
                // <section>).
                if let Some(pos) = html.rfind("</section>") {
                    html.insert_str(pos, &render_narrative_cited_evidence(citations));
                }
            }
            html
        }
        Component::Table(table) => {
            glance_catalog::render_component(&glance_catalog::Component::Table(table.clone()), ctx)
        }
        Component::Timeline(timeline) => glance_catalog::render_component(
            &glance_catalog::Component::Timeline(timeline.clone()),
            ctx,
        ),
        Component::Receipts(receipts) => render_receipts(receipts),
        Component::Footer(footer) => render_footer(footer),
        Component::Provenance(provenance) => render_provenance(provenance),
    }
}

/// Render a validated `RetroSpec` to a self-contained HTML page. Callers
/// must run `spec.validate()` first; this function does not re-validate so
/// a malformed spec fails loudly at the call site instead of rendering a
/// half-broken page.
pub fn render_html(spec: &RetroSpec) -> String {
    let now = DateTime::parse_from_rfc3339(&spec.generated_at)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    let ctx = RenderContext {
        now,
        cite_href: &cite_href,
    };
    let body: String = spec
        .components
        .iter()
        .map(|c| render_component(c, &ctx))
        .collect();
    format!(
        r#"<!doctype html><html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<link rel="stylesheet" href="aesthetic.css">
<style>{css}</style>
</head><body><main class="retro-page">{body}</main></body></html>"#,
        title = esc(&spec.title),
        css = RETRO_CSS,
        body = body
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::*;
    use glance_catalog::inline::InlineNode;
    use glance_catalog::structural::{
        Cell, CellValue, ColumnSpec, Hero, Narrative, Row, Table, Timeline, TimelineEntry,
    };

    fn text(s: &str) -> Vec<InlineNode> {
        vec![InlineNode::Text { text: s.into() }]
    }

    fn sample_spec() -> RetroSpec {
        RetroSpec {
            catalog_version: CATALOG_VERSION.to_string(),
            title: "Fleet retro — daily".to_string(),
            window_label: "daily".to_string(),
            since: "2026-07-04T21:00:00Z".to_string(),
            until: "2026-07-05T21:00:00Z".to_string(),
            generated_at: "2026-07-05T21:00:05Z".to_string(),
            components: vec![
                Component::Hero(Hero {
                    title: "Fleet retro".into(),
                    summary: text("24h ending 2026-07-05T21:00:00Z"),
                    stats: vec![glance_catalog::leaf::Metric {
                        label: "PRs".into(),
                        value: "3".into(),
                    }],
                    image_intent: None,
                }),
                Component::Table(Table {
                    heading: "Repo activity".into(),
                    columns: vec![
                        ColumnSpec {
                            key: "repo".into(),
                            label: "repo".into(),
                            numeric: false,
                            emphasize: false,
                        },
                        ColumnSpec {
                            key: "commits".into(),
                            label: "commits".into(),
                            numeric: true,
                            emphasize: false,
                        },
                    ],
                    rows: vec![Row {
                        cells: vec![
                            Cell {
                                column_key: "repo".into(),
                                value: CellValue::Text {
                                    text: "landmark".into(),
                                },
                            },
                            Cell {
                                column_key: "commits".into(),
                                value: CellValue::Text { text: "5".into() },
                            },
                        ],
                    }],
                    empty_note: None,
                    demoted_note: None,
                }),
                Component::Timeline(Timeline {
                    heading: "Timeline".into(),
                    entries: vec![TimelineEntry {
                        at: "2026-07-05T04:25:01Z".into(),
                        actor: "landmark".into(),
                        kind: "pr-merged".into(),
                        summary: "PR #200 merged".into(),
                        link: Some("https://github.com/misty-step/landmark/pull/200".into()),
                        detail: vec![],
                    }],
                    empty_note: None,
                }),
                Component::Provenance(Provenance {
                    notes: vec![ProvenanceNote {
                        source: "git:landmark".into(),
                        note: "1 repo swept".into(),
                    }],
                }),
            ],
        }
    }

    #[test]
    fn renders_every_component_kind_without_a_wall_of_text() {
        let html = render_html(&sample_spec());
        assert!(html.contains("Fleet retro"));
        assert!(html.contains("ae-stat-badges"));
        assert!(html.contains("landmark"));
        assert!(html.contains("PR #200 merged"));
        assert!(html.contains("href=\"https://github.com/misty-step/landmark/pull/200\""));
        assert!(html.contains("ae-trail"));
        assert!(html.contains("Sources"));
    }

    #[test]
    fn uses_kit_primitives_not_hand_rolled_component_classes() {
        // Regression for aesthetic-927 finding #1, now enforced by
        // construction: every structural component renders through
        // glance_catalog::render_component, which only ever emits `.ae-*`
        // classes -- there is no bespoke `.retro-hero`/`.retro-table`
        // definition left in this crate to regress back to.
        let html = render_html(&sample_spec());
        assert!(html.contains("ae-plate"));
        assert!(html.contains("class=\"ae-table\""));
        assert!(!html.contains("retro-hero"));
        assert!(!html.contains("retro-table"));
        assert!(!html.contains("retro-timeline"));
    }

    #[test]
    fn timeline_renders_relative_time_not_a_raw_iso_string_as_visible_text() {
        let html = render_html(&sample_spec());
        assert!(html.contains(r#"datetime="2026-07-05T04:25:01Z""#));
        assert!(
            html.contains(">16h ago<"),
            "expected a relative-time rendering in the visible text: {html}"
        );
        assert_eq!(html.matches("2026-07-05T04:25:01Z").count(), 2);
        assert!(!html.contains(">2026-07-05T04:25:01Z<"));
    }

    #[test]
    fn escapes_untrusted_text_content() {
        let mut spec = sample_spec();
        if let Component::Hero(hero) = &mut spec.components[0] {
            hero.title = "<script>alert(1)</script>".into();
        }
        let html = render_html(&spec);
        assert!(!html.contains("<script>alert(1)</script>"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn renders_receipts_with_title_excerpt_and_cards() {
        let mut spec = sample_spec();
        spec.components.insert(
            3,
            Component::Receipts(Receipts {
                items: vec![ReceiptRow {
                    title: "weave-908 — daily retro shipped".into(),
                    excerpt: "Shipped the daily/weekly retro end to end.".into(),
                    path: "/receipts/weave-908-report.md".into(),
                    cards: vec!["weave-908".into()],
                    at: "2026-07-05T04:00:00+00:00".into(),
                }],
            }),
        );
        let html = render_html(&spec);
        assert!(html.contains("weave-908 — daily retro shipped"));
        assert!(html.contains("Shipped the daily/weekly retro end to end."));
        assert!(html.contains("ae-wall"));
        assert!(html.contains(r#"<span class="ae-tag">weave-908</span>"#));
    }

    #[test]
    fn empty_receipts_render_explicit_empty_state() {
        let mut spec = sample_spec();
        spec.components
            .insert(3, Component::Receipts(Receipts { items: vec![] }));
        let html = render_html(&spec);
        assert!(html.contains("No campaign receipts in this window."));
    }

    #[test]
    fn empty_repo_table_and_timeline_render_explicit_empty_state_not_omission() {
        let mut spec = sample_spec();
        spec.components[1] = Component::Table(Table {
            heading: "Repo activity".into(),
            columns: vec![ColumnSpec {
                key: "repo".into(),
                label: "repo".into(),
                numeric: false,
                emphasize: false,
            }],
            rows: vec![],
            empty_note: Some("No repo activity in this window.".into()),
            demoted_note: None,
        });
        spec.components[2] = Component::Timeline(Timeline {
            heading: "Timeline".into(),
            entries: vec![],
            empty_note: Some("No dated events in this window.".into()),
        });
        let html = render_html(&spec);
        assert!(html.contains("No repo activity in this window."));
        assert!(html.contains("No dated events in this window."));
    }

    #[test]
    fn quiet_repos_render_as_a_muted_note_not_dead_rows() {
        let mut spec = sample_spec();
        spec.components[1] = Component::Table(Table {
            heading: "Repo activity".into(),
            columns: vec![ColumnSpec {
                key: "repo".into(),
                label: "repo".into(),
                numeric: false,
                emphasize: false,
            }],
            rows: vec![],
            empty_note: Some("No repo activity in this window.".into()),
            demoted_note: Some("2 repo(s) swept with no activity: glass, canary".into()),
        });
        let html = render_html(&spec);
        assert!(html.contains("2 repo(s) swept with no activity: glass, canary"));
        assert!(!html.contains("<table"));
    }

    #[test]
    fn narrative_fail_open_banner_omits_the_internal_reason_text() {
        // Regression for aesthetic-927 finding #6: the banner used to
        // interpolate the raw fail-open reason (e.g. an env-var name)
        // straight into the primary lede.
        let mut spec = sample_spec();
        spec.components.insert(
            1,
            Component::Narrative {
                narrative: Narrative {
                    heading: "What mattered".into(),
                    status: NarrativeStatus::Unavailable {
                        reason: "OPENROUTER_API_KEY not configured; skipped".into(),
                    },
                },
                citations: vec![],
            },
        );
        let html = render_html(&spec);
        assert!(!html.contains("OPENROUTER_API_KEY"));
        assert!(html.contains("Narrative synthesis unavailable this run."));
    }

    #[test]
    fn footer_collapses_diagnostics_behind_a_details_disclosure() {
        let mut spec = sample_spec();
        spec.components.insert(
            3,
            Component::Footer(Footer {
                judge: "deepseek/deepseek-v4-flash".into(),
                gate_status: "passed on attempt 1 of 3".into(),
                prompt_version: "weave-fleet-retro-narrative-v1".into(),
                pack_schema_version: "weave.evidence-pack.v1".into(),
                pack_assembly_ms: 42,
            }),
        );
        let html = render_html(&spec);
        assert!(html.contains("<details>"));
        assert!(html.contains("<summary"));
        assert!(html.contains("deepseek/deepseek-v4-flash"));
    }

    #[test]
    fn narrative_ok_renders_paragraphs_and_the_cited_evidence_appendix() {
        let mut spec = sample_spec();
        spec.components.insert(
            1,
            Component::Narrative {
                narrative: Narrative {
                    heading: "What mattered".into(),
                    status: NarrativeStatus::Ok {
                        paragraphs: vec![vec![
                            InlineNode::Text {
                                text: "Landmark shipped a fix today ".into(),
                            },
                            InlineNode::Cite {
                                text: "[aaaaaaaaaaaaaaaa]".into(),
                                ref_id: "aaaaaaaaaaaaaaaa".into(),
                            },
                        ]],
                    },
                },
                citations: vec![Citation {
                    id: "aaaaaaaaaaaaaaaa".into(),
                    title: "landmark shipped a fix".into(),
                }],
            },
        );
        let html = render_html(&spec);
        assert!(html.contains("Landmark shipped a fix today"));
        assert!(html.contains("href=\"#cite-aaaaaaaaaaaaaaaa\""));
        assert!(html.contains(r#"<li id="cite-aaaaaaaaaaaaaaaa">"#));
        assert!(html.contains("landmark shipped a fix"));
    }
}
