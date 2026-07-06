use crate::spec::{Component, NarrativeStatus, RetroSpec};

/// Page-specific CSS layered over the vendored `aesthetic.css`, in the same
/// pattern bridge.py uses (base kit + a small `<style>` override block).
/// Built entirely from Aesthetic's own custom properties (`--ae-*`) so the
/// retro inherits the fleet's dark/light theming instead of hardcoding
/// colors that would drift from the rest of the Misty Step surfaces.
const RETRO_CSS: &str = r#"
.retro-page{max-width:var(--ae-measure-wide,72rem);margin:0 auto;padding:var(--ae-space-6,2rem) var(--ae-space-4,1rem);}
.retro-hero h1{font-size:1.75rem;font-weight:var(--ae-w-black,800);margin:0 0 .25rem;}
.retro-hero p{color:var(--ae-ink-muted);margin:0 0 var(--ae-space-4,1rem);}
.retro-stats{display:grid;grid-template-columns:repeat(auto-fit,minmax(9rem,1fr));gap:var(--ae-space-3,.75rem);margin:var(--ae-space-4,1rem) 0;}
.retro-stat{border:1px solid var(--ae-line);border-radius:var(--ae-radius,.5rem);padding:var(--ae-space-3,.75rem);background:var(--ae-surface);}
.retro-stat .value{display:block;font-family:var(--ae-font-mono);font-size:1.5rem;font-weight:var(--ae-w-medium,600);}
.retro-stat .label{color:var(--ae-ink-muted);font-size:.8rem;text-transform:uppercase;letter-spacing:.03em;}
.retro-section{margin:var(--ae-space-6,2rem) 0;}
.retro-section h2{font-size:1.1rem;font-weight:var(--ae-w-medium,600);margin:0 0 var(--ae-space-3,.75rem);}
.retro-table{width:100%;border-collapse:collapse;font-size:.9rem;}
.retro-table th,.retro-table td{text-align:left;padding:.5rem .6rem;border-bottom:1px solid var(--ae-line);}
.retro-table th{color:var(--ae-ink-muted);font-weight:var(--ae-w-medium,600);}
.retro-table .highlights{color:var(--ae-ink-muted);font-size:.85rem;}
.retro-timeline{list-style:none;margin:0;padding:0;}
.retro-timeline li{display:grid;grid-template-columns:9rem 6rem 1fr;gap:.6rem;padding:.4rem 0;border-bottom:1px solid var(--ae-line);font-size:.88rem;align-items:start;}
.retro-timeline time{font-family:var(--ae-font-mono);color:var(--ae-ink-faint);}
.retro-timeline .kind{color:var(--ae-ink-muted);}
.retro-timeline a{color:var(--ae-accent);}
.retro-receipts{list-style:none;margin:0;padding:0;}
.retro-receipts li{padding:.5rem 0;border-bottom:1px solid var(--ae-line);}
.retro-receipts h3{font-size:.95rem;font-weight:var(--ae-w-medium,600);margin:0 0 .2rem;}
.retro-receipts p{margin:0 0 .2rem;color:var(--ae-ink-muted);font-size:.88rem;}
.retro-receipts .cards{font-family:var(--ae-font-mono);color:var(--ae-ink-faint);font-size:.8rem;}
.retro-provenance{font-size:.82rem;color:var(--ae-ink-muted);}
.retro-provenance ul{margin:0;padding-left:1.1rem;}
.retro-empty{color:var(--ae-ink-faint);font-style:italic;}
.retro-narrative p{margin:0 0 var(--ae-space-3,.75rem);line-height:1.55;}
.retro-narrative .citation{font-family:var(--ae-font-mono);font-size:.78em;color:var(--ae-accent);text-decoration:none;}
.retro-narrative .banner{border:1px solid var(--ae-line);border-radius:var(--ae-radius,.5rem);padding:var(--ae-space-3,.75rem);background:var(--ae-surface);color:var(--ae-ink-muted);font-style:italic;}
.retro-cited-evidence{list-style:none;margin:var(--ae-space-3,.75rem) 0 0;padding:0;font-size:.82rem;color:var(--ae-ink-muted);}
.retro-cited-evidence li{padding:.15rem 0;}
.retro-cited-evidence code{font-size:.78em;}
.retro-footer{font-size:.8rem;color:var(--ae-ink-faint);border-top:1px solid var(--ae-line);padding-top:var(--ae-space-3,.75rem);}
"#;

fn esc(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn citation_token_regex() -> regex::Regex {
    regex::Regex::new(r"\[([0-9a-f]{16})\]").expect("static citation-token pattern is valid")
}

/// Turn every `[id]` citation token in an already-HTML-escaped narrative
/// block into a hover/tap link anchored to that item's entry in the "Cited
/// evidence" list below the narrative. Escaping first is safe here: none of
/// `esc`'s substitutions touch `[`, `]`, or hex digits, so the citation
/// token survives escaping unchanged and this regex still matches it.
fn linkify_citations(escaped_block: &str) -> String {
    citation_token_regex()
        .replace_all(escaped_block, |caps: &regex::Captures| {
            let id = &caps[1];
            format!(r##"<a href="#cite-{id}" class="citation">[{id}]</a>"##)
        })
        .into_owned()
}

fn render_component(component: &Component) -> String {
    match component {
        Component::Hero(hero) => format!(
            r#"<header class="retro-hero"><h1>{}</h1><p>{}</p></header>"#,
            esc(&hero.headline),
            esc(&hero.subhead)
        ),
        Component::Narrative(narrative) => match &narrative.status {
            NarrativeStatus::Ok { blocks, citations } => {
                let paragraphs: String = blocks
                    .iter()
                    .map(|block| format!("<p>{}</p>", linkify_citations(&esc(block))))
                    .collect();
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
                format!(
                    r#"<section class="retro-section retro-narrative"><h2>What mattered</h2>{paragraphs}<ul class="retro-cited-evidence">{cited}</ul></section>"#
                )
            }
            NarrativeStatus::FailedOpen { reason } => format!(
                r#"<section class="retro-section retro-narrative"><h2>What mattered</h2><p class="banner">Narrative synthesis unavailable this run ({}). Showing the deterministic tables below.</p></section>"#,
                esc(reason)
            ),
        },
        Component::StatCallouts(stats) => {
            if stats.items.is_empty() {
                return String::new();
            }
            let items: String = stats
                .items
                .iter()
                .map(|s| {
                    format!(
                        r#"<div class="retro-stat"><span class="value">{}</span><span class="label">{}</span></div>"#,
                        esc(&s.value),
                        esc(&s.label)
                    )
                })
                .collect();
            format!(r#"<div class="retro-stats">{items}</div>"#)
        }
        Component::RepoActivityTable(table) => {
            if table.rows.is_empty() {
                return r#"<section class="retro-section"><h2>Repo activity</h2><p class="retro-empty">No repo activity in this window.</p></section>"#.to_string();
            }
            let rows: String = table
                .rows
                .iter()
                .map(|row| {
                    format!(
                        r#"<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td class="highlights">{}</td></tr>"#,
                        esc(&row.repo),
                        row.commits,
                        row.prs,
                        row.cards_touched,
                        esc(&row.highlights.join("; "))
                    )
                })
                .collect();
            format!(
                r#"<section class="retro-section"><h2>Repo activity</h2><table class="retro-table"><thead><tr><th>Repo</th><th>Commits</th><th>PRs</th><th>Cards touched</th><th>Highlights</th></tr></thead><tbody>{rows}</tbody></table></section>"#
            )
        }
        Component::Timeline(timeline) => {
            if timeline.entries.is_empty() {
                return r#"<section class="retro-section"><h2>Timeline</h2><p class="retro-empty">No dated events in this window.</p></section>"#.to_string();
            }
            let items: String = timeline
                .entries
                .iter()
                .map(|entry| {
                    let summary = if let Some(link) = &entry.link {
                        format!(r#"<a href="{}">{}</a>"#, esc(link), esc(&entry.summary))
                    } else {
                        esc(&entry.summary)
                    };
                    format!(
                        r#"<li><time datetime="{}">{}</time><span class="kind">{}</span><span>{} — {}</span></li>"#,
                        esc(&entry.at),
                        esc(&entry.at),
                        esc(&entry.kind),
                        esc(&entry.repo),
                        summary
                    )
                })
                .collect();
            format!(
                r#"<section class="retro-section"><h2>Timeline</h2><ul class="retro-timeline">{items}</ul></section>"#
            )
        }
        Component::Receipts(receipts) => {
            if receipts.items.is_empty() {
                return r#"<section class="retro-section"><h2>Receipts</h2><p class="retro-empty">No campaign receipts in this window.</p></section>"#.to_string();
            }
            let items: String = receipts
                .items
                .iter()
                .map(|item| {
                    let cards = if item.cards.is_empty() {
                        String::new()
                    } else {
                        format!(
                            r#"<span class="cards">{}</span>"#,
                            esc(&item.cards.join(", "))
                        )
                    };
                    format!(
                        r#"<li><h3>{}</h3><p>{}</p>{}</li>"#,
                        esc(&item.title),
                        esc(&item.excerpt),
                        cards
                    )
                })
                .collect();
            format!(
                r#"<section class="retro-section"><h2>Receipts</h2><ul class="retro-receipts">{items}</ul></section>"#
            )
        }
        Component::Footer(footer) => format!(
            r#"<footer class="retro-footer">judge: <code>{}</code> · gate: {} · prompt: <code>{}</code> · pack: <code>{}</code> · pack assembly: {}ms</footer>"#,
            esc(&footer.judge),
            esc(&footer.gate_status),
            esc(&footer.prompt_version),
            esc(&footer.pack_schema_version),
            footer.pack_assembly_ms
        ),
        Component::Provenance(provenance) => {
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
                r#"<section class="retro-section retro-provenance"><h2>Sources</h2><ul>{items}</ul></section>"#
            )
        }
    }
}

/// Render a validated `RetroSpec` to a self-contained HTML page. Callers
/// must run `spec.validate()` first; this function does not re-validate so
/// a malformed spec fails loudly at the call site instead of rendering a
/// half-broken page.
pub fn render_html(spec: &RetroSpec) -> String {
    let body: String = spec.components.iter().map(render_component).collect();
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
                    headline: "Fleet retro".into(),
                    subhead: "24h ending 2026-07-05T21:00:00Z".into(),
                }),
                Component::StatCallouts(StatCallouts {
                    items: vec![StatCallout {
                        label: "PRs".into(),
                        value: "3".into(),
                    }],
                }),
                Component::RepoActivityTable(RepoActivityTable {
                    rows: vec![RepoActivityRow {
                        repo: "landmark".into(),
                        commits: 5,
                        prs: 1,
                        cards_touched: 1,
                        highlights: vec!["landmark-907 fixed".into()],
                    }],
                }),
                Component::Timeline(Timeline {
                    entries: vec![TimelineEntry {
                        at: "2026-07-05T04:25:01Z".into(),
                        repo: "landmark".into(),
                        kind: "pr-merged".into(),
                        summary: "PR #200 merged".into(),
                        source: "git:landmark".into(),
                        link: Some("https://github.com/misty-step/landmark/pull/200".into()),
                    }],
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
        assert!(html.contains("<h1>Fleet retro</h1>"));
        assert!(html.contains("retro-stats"));
        assert!(html.contains("landmark"));
        assert!(html.contains("PR #200 merged"));
        assert!(html.contains("href=\"https://github.com/misty-step/landmark/pull/200\""));
        assert!(html.contains("retro-timeline"));
        assert!(html.contains("Sources"));
    }

    #[test]
    fn escapes_untrusted_text_content() {
        let mut spec = sample_spec();
        if let Component::Hero(hero) = &mut spec.components[0] {
            hero.headline = "<script>alert(1)</script>".into();
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
        assert!(html.contains("retro-receipts"));
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
        spec.components[2] = Component::RepoActivityTable(RepoActivityTable { rows: vec![] });
        spec.components[3] = Component::Timeline(Timeline { entries: vec![] });
        let html = render_html(&spec);
        assert!(html.contains("No repo activity in this window."));
        assert!(html.contains("No dated events in this window."));
    }
}
