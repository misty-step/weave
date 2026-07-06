mod assemble;
mod citation_gate;
mod pack;
mod publish;
mod render;
mod secrets;
mod sources;
mod spec;
mod synthesis;
mod window;

use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use clap::Parser;

use sources::{SourceNote, bb, feed, git, moments, powder, receipts};
use spec::{Footer, Narrative, NarrativeStatus};
use window::RetroWindow;

/// Generate a fleet-wide agent-activity retro over a time window (daily,
/// weekly, or an arbitrary custom range), render it through a spec-first
/// deterministic HTML renderer using the Misty Step Aesthetic, and
/// (unless --no-publish) push it to the bastion artifact shelf and post a
/// `kind=report` entry to the Bridge feed. See docs/fleet-retro.md.
#[derive(Parser, Debug)]
#[command(name = "fleet-retro", version, about)]
struct Cli {
    /// Which window to generate: daily (last 24h), weekly (last 7d), or custom.
    /// Ignored when --scheduled is set.
    #[arg(long, value_enum, default_value = "daily")]
    window: WindowKind,

    /// Scheduled mode: always generate the daily retro, and additionally the
    /// weekly retro when today is Sunday. This is the single entry point
    /// the LaunchAgent calls at ~21:00 local, so one calendar trigger covers
    /// both cadences without a second plist.
    #[arg(long)]
    scheduled: bool,

    /// Custom window start (RFC3339); required when --window=custom
    #[arg(long)]
    since: Option<DateTime<Utc>>,

    /// Custom window end (RFC3339); defaults to now when --window=custom
    #[arg(long)]
    until: Option<DateTime<Utc>>,

    /// Directory containing fleet repo checkouts to sweep for git activity
    #[arg(long, env = "FLEET_RETRO_DEV_ROOT")]
    dev_root: Option<PathBuf>,

    /// Directory containing the Bridge feed's *.jsonl files
    #[arg(long, env = "FLEET_RETRO_FEED_DIR")]
    feed_dir: Option<PathBuf>,

    /// Directory containing campaign receipt markdown files
    #[arg(long, env = "FLEET_RETRO_CAMPAIGN_DIR")]
    campaign_dir: Option<PathBuf>,

    /// Path to a bb plane.toml directory to read run history from (omit to skip bb)
    #[arg(long, env = "FLEET_RETRO_BB_PLANE")]
    bb_plane: Option<String>,

    /// Path to Bitterblossom's moment-scorer.py (omit to derive
    /// <dev-root>/bitterblossom/scripts/moment-scorer.py)
    #[arg(long, env = "FLEET_RETRO_MOMENT_SCORER_SCRIPT")]
    moment_scorer_script: Option<PathBuf>,

    /// Path to the moment-scorer's own moments.db (omit to derive
    /// <bb-plane>/.bb/moments.db; skipped entirely without --bb-plane)
    #[arg(long, env = "FLEET_RETRO_MOMENTS_DB")]
    moments_db: Option<PathBuf>,

    /// Max Powder cards to inspect for in-window movements
    #[arg(long, default_value_t = 300)]
    card_limit: u32,

    /// Skip the model synthesis stage entirely: always render the
    /// deterministic tables-only report, as if every synthesis attempt had
    /// failed open. Useful for dry runs and local iteration without
    /// spending OpenRouter budget.
    #[arg(long)]
    no_synthesis: bool,

    /// Directory for durable synthesis metrics (pack-assembly-latency.jsonl)
    /// -- recorded from the first run on, per the pull-federation falsifier
    /// (weave-923): if pack assembly ever exceeds report cadence, the fix is
    /// a cached pull snapshot, not event-sourcing.
    #[arg(long, env = "FLEET_RETRO_METRICS_DIR")]
    metrics_dir: Option<PathBuf>,

    /// Output directory for the rendered report (default: a dated dir under --out-root).
    /// Ignored when --scheduled is set (each window gets its own dated dir).
    #[arg(long)]
    out: Option<PathBuf>,

    /// Root directory for dated output dirs when --out is not given
    #[arg(long, env = "FLEET_RETRO_OUT_ROOT")]
    out_root: Option<PathBuf>,

    /// Print the assembled RetroSpec as JSON and exit before rendering
    #[arg(long)]
    dry_run: bool,

    /// Render and write locally but skip the shelf publish + feed post
    #[arg(long)]
    no_publish: bool,
}

#[derive(Copy, Clone, Debug, clap::ValueEnum)]
enum WindowKind {
    Daily,
    Weekly,
    Custom,
}

fn home_dir() -> Result<PathBuf> {
    std::env::var("HOME")
        .map(PathBuf::from)
        .context("HOME is not set")
}

/// Append one JSONL line recording this run's pack-assembly latency to a
/// durable metrics log, starting with the very first run (oracle finding
/// ruled binding, weave-923: "Pack-assembly latency (p95) recorded from the
/// first run — it is the named falsifier for pull-federation"). A true p95
/// needs many samples; this makes sure they exist to compute one from,
/// without blocking a single run on that computation. Best-effort: a
/// logging failure here must never fail the report itself.
fn record_pack_assembly_latency(cli: &Cli, home: &std::path::Path, window: &RetroWindow, ms: u64) {
    let dir = cli
        .metrics_dir
        .clone()
        .unwrap_or_else(|| home.join(".factory-lanes").join("fleet-retro"));
    if let Err(err) = std::fs::create_dir_all(&dir) {
        eprintln!(
            "fleet-retro: could not create metrics dir {}: {err}",
            dir.display()
        );
        return;
    }
    let line = serde_json::json!({
        "ts": Utc::now().to_rfc3339(),
        "window_label": window.label,
        "pack_assembly_ms": ms,
    });
    let path = dir.join("pack-assembly-latency.jsonl");
    let result = (|| -> std::io::Result<()> {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        writeln!(file, "{line}")
    })();
    if let Err(err) = result {
        eprintln!(
            "fleet-retro: could not append pack-assembly latency to {}: {err}",
            path.display()
        );
    }
}

fn resolve_window(cli: &Cli, now: DateTime<Utc>) -> Result<RetroWindow> {
    match cli.window {
        WindowKind::Daily => Ok(RetroWindow::daily(now)),
        WindowKind::Weekly => Ok(RetroWindow::weekly(now)),
        WindowKind::Custom => {
            let since = cli
                .since
                .context("--since is required when --window=custom")?;
            let until = cli.until.unwrap_or(now);
            RetroWindow::custom(since, until)
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let home = home_dir()?;
    let now = Utc::now();

    if cli.scheduled {
        eprintln!("fleet-retro: scheduled run at {now}");
        generate_and_publish(&cli, &home, RetroWindow::daily(now))?;
        if RetroWindow::is_weekly_day(now) {
            eprintln!("fleet-retro: today is the weekly day, running the weekly retro too");
            generate_and_publish(&cli, &home, RetroWindow::weekly(now))?;
        }
        return Ok(());
    }

    let window = resolve_window(&cli, now)?;
    generate_and_publish(&cli, &home, window)
}

fn generate_and_publish(cli: &Cli, home: &std::path::Path, window: RetroWindow) -> Result<()> {
    let now = Utc::now();
    let dev_root = cli
        .dev_root
        .clone()
        .unwrap_or_else(|| home.join("Development"));
    let feed_dir = cli
        .feed_dir
        .clone()
        .unwrap_or_else(|| home.join(".factory-lanes").join("feed"));
    let campaign_dir = cli
        .campaign_dir
        .clone()
        .unwrap_or_else(|| home.join(".factory-lanes").join("campaign"));

    eprintln!(
        "fleet-retro: window={} since={} until={}",
        window.label, window.since, window.until
    );

    let mut notes: Vec<SourceNote> = Vec::new();
    let assembly_start = std::time::Instant::now();

    // --- git activity across every discovered repo -------------------------
    let repos = git::discover_repos(&dev_root);
    if repos.is_empty() {
        notes.push(SourceNote::new(
            format!("git:{}", dev_root.display()),
            "no git checkouts discovered".to_string(),
        ));
    }
    let mut repo_activity = Vec::new();
    let mut quiet_repos = Vec::new();
    for repo_path in &repos {
        match git::collect_repo_activity(repo_path, &window) {
            Ok(activity) => {
                if activity.is_empty() {
                    quiet_repos.push(activity.repo.clone());
                }
                repo_activity.push(activity);
            }
            Err(err) => {
                let name = repo_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                eprintln!("fleet-retro: git collection failed for {name}: {err}");
                notes.push(SourceNote::new(
                    format!("git:{name}"),
                    format!("collection failed: {err}"),
                ));
            }
        }
    }
    if !quiet_repos.is_empty() {
        notes.push(SourceNote::new(
            format!("git:{}", dev_root.display()),
            format!(
                "{} repo(s) swept with no commits or PR references in window: {}",
                quiet_repos.len(),
                quiet_repos.join(", ")
            ),
        ));
    }

    // --- Powder card movements ----------------------------------------------
    let card_movements = match powder::PowderClient::from_env() {
        Some(client) => powder::collect_card_movements(&client, &window, cli.card_limit),
        None => {
            notes.push(SourceNote::new(
                "powder",
                "POWDER_API_BASE_URL/POWDER_API_KEY not set; skipped".to_string(),
            ));
            Vec::new()
        }
    };

    // --- bb plane runs -------------------------------------------------------
    let bb_runs = bb::collect_bb_runs(cli.bb_plane.as_deref(), &window);
    if cli.bb_plane.is_none() {
        notes.push(SourceNote::new(
            "bb",
            "no --bb-plane configured; skipped".to_string(),
        ));
    }

    // --- Bridge feed events ---------------------------------------------------
    let feed_events = feed::collect_feed_events(&feed_dir, &window);

    // --- campaign receipts ----------------------------------------------------
    let campaign_receipts = receipts::collect_receipts(&campaign_dir, &window);

    // --- moment-scorer anomaly cards (weave-923) -----------------------------
    let moment_scorer_script = cli.moment_scorer_script.clone().unwrap_or_else(|| {
        dev_root
            .join("bitterblossom")
            .join("scripts")
            .join("moment-scorer.py")
    });
    let moments_db = cli.moments_db.clone().or_else(|| {
        cli.bb_plane
            .as_deref()
            .map(|plane| std::path::Path::new(plane).join(".bb").join("moments.db"))
    });
    let moment_cards = moments::collect_moments(
        moment_scorer_script
            .is_file()
            .then_some(moment_scorer_script.as_path()),
        moments_db.as_deref(),
        &window,
    );
    if moments_db.is_none() {
        notes.push(SourceNote::new(
            "moments",
            "no --bb-plane/--moments-db configured; skipped".to_string(),
        ));
    }

    // --- evidence pack: the versioned intermediate every collector's
    // output projects into before RetroSpec assembly ever sees it. This is
    // the seam weave-923's synthesis stage and citation gate build on.
    let evidence_pack = pack::build_pack(
        &window,
        &repo_activity,
        &card_movements,
        &bb_runs,
        &feed_events,
        &campaign_receipts,
        &moment_cards,
    );
    let pack_assembly_ms = assembly_start.elapsed().as_millis() as u64;
    record_pack_assembly_latency(cli, home, &window, pack_assembly_ms);

    // --- synthesis stage + citation gate (weave-923) -------------------------
    // Cheap-default/escalate-on-failure model routing, bounded retries, and
    // fail-open to a deterministic tables-only report are all inside
    // `synthesis::synthesize` -- this call site only decides WHETHER to
    // attempt synthesis at all (`--no-synthesis`, or no OpenRouter key
    // configured, both degrade to the same fail-open shape `synthesize`
    // itself produces on exhausted attempts).
    let (narrative, judge, gate_status) = if cli.no_synthesis {
        (
            Narrative {
                status: NarrativeStatus::FailedOpen {
                    reason: "--no-synthesis set; skipped".to_string(),
                },
            },
            "none".to_string(),
            "skipped: --no-synthesis".to_string(),
        )
    } else {
        match synthesis::OpenRouterClient::from_env() {
            Some(client) => {
                let outcome = synthesis::synthesize(&client, &evidence_pack);
                (outcome.narrative, outcome.judge, outcome.gate_status)
            }
            None => (
                Narrative {
                    status: NarrativeStatus::FailedOpen {
                        reason: "OPENROUTER_API_KEY not configured; skipped".to_string(),
                    },
                },
                "none".to_string(),
                "fail-open: OPENROUTER_API_KEY not configured".to_string(),
            ),
        }
    };
    let footer = Footer {
        judge,
        gate_status,
        prompt_version: synthesis::PROMPT_VERSION.to_string(),
        pack_schema_version: evidence_pack.schema_version.clone(),
        pack_assembly_ms,
    };

    let generated_at = now.to_rfc3339();
    let retro_spec = assemble::build_spec(
        &window,
        &generated_at,
        &evidence_pack,
        narrative,
        footer,
        notes,
    )?;

    if cli.dry_run {
        println!("{}", serde_json::to_string_pretty(&retro_spec)?);
        return Ok(());
    }

    let html = render::render_html(&retro_spec);

    let out_dir = cli.out.clone().unwrap_or_else(|| {
        let root = cli
            .out_root
            .clone()
            .unwrap_or_else(|| home.join(".factory-lanes").join("fleet-retro"));
        root.join(format!(
            "{}-{}",
            window.label,
            now.format("%Y-%m-%dT%H%M%SZ")
        ))
    });
    std::fs::create_dir_all(&out_dir)
        .with_context(|| format!("creating output dir {}", out_dir.display()))?;
    std::fs::write(out_dir.join("index.html"), &html)
        .with_context(|| format!("writing {}/index.html", out_dir.display()))?;
    // Structured sibling to index.html so a consumer (weave-mcp's
    // get_latest_fleet_retro tool, or any future data surface) can read the
    // assembled evidence directly instead of scraping rendered HTML.
    std::fs::write(
        out_dir.join("spec.json"),
        serde_json::to_string_pretty(&retro_spec)?,
    )
    .with_context(|| format!("writing {}/spec.json", out_dir.display()))?;
    // The versioned evidence intermediate the report was built from --
    // rides the same publish path as index.html/spec.json so it's the
    // citation gate's (weave-923) ground truth wherever the report lands.
    std::fs::write(
        out_dir.join("evidence-pack.json"),
        serde_json::to_string_pretty(&evidence_pack)?,
    )
    .with_context(|| format!("writing {}/evidence-pack.json", out_dir.display()))?;
    publish::vendor_aesthetic_css(&out_dir, home)?;
    println!("{}", out_dir.join("index.html").display());

    if cli.no_publish {
        eprintln!("fleet-retro: --no-publish set; skipped shelf publish and feed post");
        return Ok(());
    }

    // Each window gets its own shelf path (fleet-retro/daily, .../weekly,
    // .../custom) so a Sunday weekly run never overwrites the daily run's
    // page underneath it -- the operator reads "the last daily AND the last
    // weekly retro" (acceptance criterion 5), which requires both to keep
    // existing side by side, not last-write-wins on one shared path.
    let slug = format!("fleet-retro/{}", window.label);
    let report_url = publish::publish_to_shelf(&slug, &out_dir);
    if let Some(url) = &report_url {
        eprintln!("fleet-retro: published to {url}");
    }
    let feed_title = format!(
        "Fleet retro — {} ({}h, {} PRs referenced, {} cards touched)",
        window.label,
        window.duration_hours(),
        repo_activity
            .iter()
            .map(|a| a.pr_numbers.len())
            .sum::<usize>(),
        card_movements
            .iter()
            .map(|m| m.card_id.clone())
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    );
    let feed_body = format!(
        "Covers {} → {}. {} repos swept, {} feed events, {} bb runs, {} receipts, {} moment cards.",
        window.since,
        window.until,
        repo_activity.len(),
        feed_events.len(),
        bb_runs.len(),
        campaign_receipts.len(),
        moment_cards.len()
    );
    publish::post_feed_report(home, &feed_title, &feed_body, report_url.as_deref());

    Ok(())
}
