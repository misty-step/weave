use chrono::{DateTime, Utc};

use crate::spec::{Component, NarrativeStatus, RetroSpec};

/// Page-specific CSS layered over the vendored `aesthetic.css`, in the same
/// pattern bridge.py uses (base kit + a small `<style>` override block).
/// Unlike the pre-`aesthetic-927` version of this file, this block carries
/// ONLY page-shell layout -- the page's own max-width, spacing, and heading
/// resets, exactly like bridge.py's own `.bridge-shell` page-local CSS.
/// Every component that has a kit primitive (`.ae-stat-badges`,
/// `.ae-table`/`.ae-plate`, `.ae-trail`, `.ae-wall`) now uses that primitive
/// instead of a hand-rolled `.retro-*` reinvention of it (designer critique,
/// aesthetic-927: "fleet-retro daily does not consume the Aesthetic
/// component vocabulary at all -- zero `.ae-*` classes appear anywhere").
const RETRO_CSS: &str = r#"
.retro-page{max-width:var(--ae-measure-wide);margin:0 auto;padding:var(--ae-space-6) var(--ae-space-4);}
.retro-page h1,.retro-page h2,.retro-page h3{font-size:16px;font-weight:var(--ae-w-medium);margin:0;}
.retro-hero p{margin:var(--ae-space-1) 0 var(--ae-space-4);}
.retro-section{margin:var(--ae-space-6) 0;}
.retro-section>h2{margin-bottom:var(--ae-space-3);}
.retro-narrative p{margin:0 0 var(--ae-space-3);line-height:1.55;}
.retro-narrative .citation{font-family:var(--ae-font-mono);font-size:11px;color:var(--ae-accent);text-decoration:none;}
.retro-narrative .banner{border:1px solid var(--ae-line);padding:var(--ae-space-3);color:var(--ae-ink-muted);}
.retro-cited-evidence{list-style:none;margin:var(--ae-space-3) 0 0;padding:0;font-size:13px;color:var(--ae-ink-muted);}
.retro-cited-evidence li{padding:.15rem 0;}
.retro-cited-evidence code{font-size:11px;}
.retro-quiet-note{margin-top:var(--ae-space-3);}
.retro-footer{font-size:13px;color:var(--ae-ink-faint);border-top:1px solid var(--ae-line);padding-top:var(--ae-space-3);}
.retro-provenance ul{margin:0;padding-left:1.1rem;font-size:13px;color:var(--ae-ink-muted);}
.retro-empty{color:var(--ae-ink-faint);font-style:italic;}
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

/// Relative-time rendering, ported from bridge.py's own `relative_time` (211
/// proven live renders there, zero raw ISO in reader-facing text) so
/// fleet-retro's timeline and receipts match the fleet's one established
/// convention instead of printing a raw `datetime="..."` string as visible
/// text (designer critique, aesthetic-927, finding #3). `now` is always
/// `spec.generated_at`, not the wall clock: this report is a static page
/// rendered once, so "how long ago" is relative to generation time, exactly
/// like bridge.py's own pages -- and pinning it to spec data keeps this
/// function a pure, golden-testable function of `RetroSpec` alone. Falls
/// back to the raw string when it fails to parse -- degraded, not a crash.
fn relative_time(raw: &str, now: DateTime<Utc>) -> String {
    let Ok(parsed) = DateTime::parse_from_rfc3339(raw) else {
        return raw.to_string();
    };
    let dt = parsed.with_timezone(&Utc);
    let delta = (now - dt).num_seconds();
    if delta < 60 {
        return "just now".to_string();
    }
    if delta < 3_600 {
        return format!("{}m ago", delta / 60);
    }
    if delta < 86_400 {
        return format!("{}h ago", delta / 3_600);
    }
    if delta < 7 * 86_400 {
        return format!("{}d ago", delta / 86_400);
    }
    dt.format("%b %-d").to_string()
}

fn render_component(component: &Component, now: DateTime<Utc>) -> String {
    match component {
        Component::Hero(hero) => format!(
            r#"<header class="retro-hero"><h1 class="ae-strong">{}</h1><p class="ae-dim">{}</p></header>"#,
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
            // The raw fail-open reason (e.g. "OPENROUTER_API_KEY not
            // configured; skipped") is implementation detail, not reader
            // content -- it stays available in the Footer's collapsed
            // diagnostics block, which already carries the equivalent
            // `gate_status` string. Surfacing an env-var name in the
            // primary lede was the "wrong place for a system's own 'why
            // didn't this feature run' apology" a live designer critique
            // named (aesthetic-927, finding #6): "no narrative this run" is
            // enough here.
            NarrativeStatus::FailedOpen { reason: _ } => {
                r#"<section class="retro-section retro-narrative"><h2>What mattered</h2><p class="banner">Narrative synthesis unavailable this run. Showing the deterministic tables below.</p></section>"#.to_string()
            }
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
                        r#"<span class="ae-stat-badge"><span class="ae-stat-value">{}</span><span class="ae-stat-label">{}</span></span>"#,
                        esc(&s.value),
                        esc(&s.label)
                    )
                })
                .collect();
            format!(r#"<div class="ae-stat-badges">{items}</div>"#)
        }
        Component::RepoActivityTable(table) => {
            if table.rows.is_empty() && table.quiet_repos.is_empty() {
                return r#"<section class="retro-section"><h2>Repo activity</h2><p class="retro-empty">No repo activity in this window.</p></section>"#.to_string();
            }
            // Zero-signal repos are demoted out of the table into this one
            // muted line -- the same demotion the Sources section already
            // applies to git-only quiet repos, now applied to the table
            // itself instead of re-listing every one as a dead row
            // (designer critique, aesthetic-927, finding #4).
            let quiet_note = if table.quiet_repos.is_empty() {
                String::new()
            } else {
                format!(
                    r#"<p class="ae-dim retro-quiet-note">{} repo(s) swept with no activity: {}</p>"#,
                    table.quiet_repos.len(),
                    esc(&table.quiet_repos.join(", "))
                )
            };
            if table.rows.is_empty() {
                return format!(
                    r#"<section class="retro-section"><h2>Repo activity</h2>{quiet_note}</section>"#
                );
            }
            let rows: String = table
                .rows
                .iter()
                .map(|row| {
                    format!(
                        r#"<tr><td class="ae-item" data-label="repo">{}</td><td class="num" data-label="commits">{}</td><td class="num" data-label="PRs">{}</td><td class="num" data-label="cards">{}</td><td data-label="highlights">{}</td></tr>"#,
                        esc(&row.repo),
                        row.commits,
                        row.prs,
                        row.cards_touched,
                        esc(&row.highlights.join("; "))
                    )
                })
                .collect();
            format!(
                r#"<section class="retro-section"><h2>Repo activity</h2><div class="ae-plate"><table class="ae-table"><thead><tr><th>repo</th><th class="num">commits</th><th class="num">PRs</th><th class="num">cards</th><th>highlights</th></tr></thead><tbody>{rows}</tbody></table></div>{quiet_note}</section>"#
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
                    let body = if let Some(link) = &entry.link {
                        format!(r#"<a href="{}">{}</a>"#, esc(link), esc(&entry.summary))
                    } else {
                        esc(&entry.summary)
                    };
                    format!(
                        r#"<li class="ae-trail-item"><div class="ae-trail-head"><time class="ae-trail-time" datetime="{}" title="{}">{}</time><span class="ae-trail-who">{}</span></div><div class="ae-trail-body"><span class="ae-dim">{}</span> {}</div></li>"#,
                        esc(&entry.at),
                        esc(&entry.at),
                        esc(&relative_time(&entry.at, now)),
                        esc(&entry.repo),
                        esc(&entry.kind),
                        body
                    )
                })
                .collect();
            format!(
                r#"<section class="retro-section"><h2>Timeline</h2><ul class="ae-trail">{items}</ul></section>"#
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
                        esc(&relative_time(&item.at, now))
                    )
                })
                .collect();
            format!(
                r#"<section class="retro-section"><h2>Receipts</h2><div class="ae-wall">{items}</div></section>"#
            )
        }
        Component::Footer(footer) => format!(
            r#"<footer class="retro-footer"><details><summary class="ae-dim">diagnostics</summary><p class="ae-dim">judge: <code>{}</code> · gate: <code>{}</code> · prompt: <code>{}</code> · pack: <code>{}</code> · pack assembly: {}ms</p></details></footer>"#,
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
    let now = DateTime::parse_from_rfc3339(&spec.generated_at)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    let body: String = spec
        .components
        .iter()
        .map(|c| render_component(c, now))
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
                    quiet_repos: vec![],
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
        assert!(html.contains("<h1 class=\"ae-strong\">Fleet retro</h1>"));
        assert!(html.contains("ae-stat-badges"));
        assert!(html.contains("landmark"));
        assert!(html.contains("PR #200 merged"));
        assert!(html.contains("href=\"https://github.com/misty-step/landmark/pull/200\""));
        assert!(html.contains("ae-trail"));
        assert!(html.contains("Sources"));
    }

    #[test]
    fn uses_kit_primitives_not_hand_rolled_component_classes() {
        // Regression for aesthetic-927 finding #1: the pre-fix page linked
        // aesthetic.css but reinvented every primitive locally, with zero
        // `.ae-*` classes anywhere in the document.
        let html = render_html(&sample_spec());
        assert!(html.contains("class=\"ae-plate\""));
        assert!(html.contains("class=\"ae-table\""));
        assert!(!html.contains("retro-stats"));
        assert!(!html.contains("retro-table"));
        assert!(!html.contains("retro-timeline"));
        assert!(!html.contains("retro-receipts"));
    }

    #[test]
    fn timeline_renders_relative_time_not_a_raw_iso_string_as_visible_text() {
        // Regression for aesthetic-927 finding #3: every timeline entry
        // printed its raw `datetime="..."` string as the *visible* text.
        // The raw ISO is still correct as the `<time>` element's own
        // machine-readable attribute; only the visible text must change.
        let html = render_html(&sample_spec());
        assert!(html.contains(r#"datetime="2026-07-05T04:25:01Z""#));
        assert!(
            html.contains(">16h ago<"),
            "expected a relative-time rendering in the visible text: {html}"
        );
        // The raw ISO appears exactly twice: as the `datetime` attribute
        // (machine-readable) and as the `title` attribute (exact time on
        // hover) -- never a third time as the element's own visible text.
        assert_eq!(html.matches("2026-07-05T04:25:01Z").count(), 2);
        assert!(!html.contains(">2026-07-05T04:25:01Z<"));
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
        spec.components[2] = Component::RepoActivityTable(RepoActivityTable {
            rows: vec![],
            quiet_repos: vec![],
        });
        spec.components[3] = Component::Timeline(Timeline { entries: vec![] });
        let html = render_html(&spec);
        assert!(html.contains("No repo activity in this window."));
        assert!(html.contains("No dated events in this window."));
    }

    #[test]
    fn quiet_repos_render_as_a_muted_note_not_dead_rows() {
        let mut spec = sample_spec();
        spec.components[2] = Component::RepoActivityTable(RepoActivityTable {
            rows: vec![],
            quiet_repos: vec!["glass".into(), "canary".into()],
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
            Component::Narrative(Narrative {
                status: NarrativeStatus::FailedOpen {
                    reason: "OPENROUTER_API_KEY not configured; skipped".into(),
                },
            }),
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
}
