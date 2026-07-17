use std::path::Path;
use std::process::Command;

/// Literal placeholder bearer Mint's artifacts proxy route recognizes as
/// "inject the real shelf credential here" (`deploy/policy.yaml`'s
/// `secret://artifacts/default` alias, service `sanctum.tail5f5eb4.ts.net`).
/// Harmless to hardcode and log -- Mint swaps in the actual token only
/// inside its own broker process, so this caller never holds it.
const MINT_ARTIFACTS_BEARER: &str = "Bearer __mint.artifacts.default__";

/// Mirror `local_dir` to the Sanctum artifact shelf at
/// `https://sanctum.tail5f5eb4.ts.net/artifacts/a/<slug>/`, matching
/// bridge.py's `publish_to_shelf`. Uploads are routed through Mint's
/// generic HTTPS proxy (`${MINT_BASE_URL}/proxy/https/<host>/<path>`)
/// rather than this process holding a raw artifact-shelf bearer token --
/// Mint owns the real credential; this caller only ever sends the harmless
/// placeholder bearer. Best-effort: a missing/invalid `MINT_BASE_URL`, or a
/// failed PUT, is logged to stderr and does not fail the run, since the
/// local `local_dir` output is canonical -- the shelf copy is delivery, not
/// the source of truth. Returns the public (non-Mint) URL for `index.html`
/// when at least one file published.
pub fn publish_to_shelf(slug: &str, local_dir: &Path) -> Option<String> {
    let Some(mint_base) = mint_base_url() else {
        eprintln!("fleet-retro: MINT_BASE_URL not set or invalid; skipping shelf publish");
        return None;
    };
    let base = shelf_base(slug);
    let mut published_any = false;
    for entry in walk_files(local_dir) {
        let Ok(rel) = entry.strip_prefix(local_dir) else {
            continue;
        };
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        let Ok(bytes) = std::fs::read(&entry) else {
            continue;
        };
        let url = mint_upload_url(&mint_base, slug, &rel_str);
        match ureq::put(&url)
            .set("Authorization", MINT_ARTIFACTS_BEARER)
            .send_bytes(&bytes)
        {
            Ok(_) => published_any = true,
            Err(err) => eprintln!("fleet-retro: shelf publish failed for {rel_str}: {err}"),
        }
    }
    published_any.then(|| format!("{base}/index.html"))
}

/// `MINT_BASE_URL` is plain non-secret configuration -- Mint's own
/// tailnet-reachable origin, not a credential -- so it is read from the
/// environment only, with no `~/.secrets` fallback. `None` covers both
/// "unset" and "not a well-formed http(s) origin"; both are treated the
/// same by the best-effort caller above.
fn mint_base_url() -> Option<String> {
    let raw = std::env::var("MINT_BASE_URL").ok()?;
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.is_empty() || !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return None;
    }
    Some(trimmed.to_string())
}

/// Public shelf origin, unchanged by the Mint cutover: this is what
/// `publish_to_shelf` returns to callers and what the Bridge feed links to.
fn shelf_base(slug: &str) -> String {
    format!("https://sanctum.tail5f5eb4.ts.net/artifacts/a/{slug}")
}

/// Mint-mediated upload target for one file: the same public shelf path,
/// routed through Mint's generic HTTPS proxy contract
/// (`${MINT_BASE_URL}/proxy/https/<host>/<path>`) so this process never
/// holds the real shelf bearer token. Deliberately distinct from
/// `shelf_base` -- callers must never mix the two up.
fn mint_upload_url(mint_base: &str, slug: &str, rel_path: &str) -> String {
    format!("{mint_base}/proxy/https/sanctum.tail5f5eb4.ts.net/artifacts/a/{slug}/{rel_path}")
}

fn walk_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return files;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(walk_files(&path));
        } else {
            files.push(path);
        }
    }
    files
}

/// Vendor Aesthetic's CSS into the output directory, same convention as
/// bridge.py: the report ships its own copy of the shared stylesheet rather
/// than linking a path that only exists on this machine.
pub fn vendor_aesthetic_css(out_dir: &Path, home: &Path) -> anyhow::Result<()> {
    let src = home
        .join("Development")
        .join("aesthetic")
        .join("aesthetic.css");
    let dest = out_dir.join("aesthetic.css");
    if src.is_file() {
        std::fs::copy(&src, &dest)?;
    } else {
        eprintln!(
            "fleet-retro: aesthetic.css not found at {}; report will render unstyled",
            src.display()
        );
    }
    Ok(())
}

/// Append a `kind=report` entry to the Bridge feed via the existing
/// `feed-post` script (`~/.factory-lanes/scripts/feed-post`), so the retro
/// shows up in the Bridge's feed section pointing at the rendered page.
/// Reuses the operator's existing feed pipeline rather than re-implementing
/// the JSONL append + media-staging logic in Rust -- `feed-post` is the one
/// writer every other producer already goes through.
pub fn post_feed_report(home: &Path, title: &str, body: &str, report_url: Option<&str>) {
    let script = home
        .join(".factory-lanes")
        .join("scripts")
        .join("feed-post");
    if !script.is_file() {
        eprintln!(
            "fleet-retro: feed-post script not found at {}; skipping feed post",
            script.display()
        );
        return;
    }
    let mut cmd = Command::new(script);
    cmd.args([
        "--agent",
        "fleet-retro",
        "--model",
        "none",
        "--kind",
        "report",
    ]);
    cmd.args(["--title", title]);
    cmd.args(["--body", body]);
    if let Some(url) = report_url {
        cmd.args(["--link", &format!("report={url}")]);
    }
    match cmd.output() {
        Ok(output) if output.status.success() => {}
        Ok(output) => eprintln!(
            "fleet-retro: feed-post exited nonzero: {}",
            String::from_utf8_lossy(&output.stderr)
        ),
        Err(err) => eprintln!("fleet-retro: feed-post failed to execute: {err}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walk_files_finds_nested_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("index.html"), "x").unwrap();
        std::fs::write(dir.path().join("sub").join("a.css"), "y").unwrap();

        let files = walk_files(dir.path());

        assert_eq!(files.len(), 2);
    }

    #[test]
    fn shelf_base_uses_the_canonical_sanctum_origin() {
        assert_eq!(
            shelf_base("fleet-retro/daily"),
            "https://sanctum.tail5f5eb4.ts.net/artifacts/a/fleet-retro/daily"
        );
    }

    #[test]
    fn mint_upload_url_proxies_the_public_shelf_path_without_becoming_it() {
        let mint = mint_upload_url("http://127.0.0.1:4949", "fleet-retro/daily", "index.html");
        assert_eq!(
            mint,
            "http://127.0.0.1:4949/proxy/https/sanctum.tail5f5eb4.ts.net/artifacts/a/fleet-retro/daily/index.html"
        );

        let public = format!("{}/index.html", shelf_base("fleet-retro/daily"));
        assert_eq!(
            public,
            "https://sanctum.tail5f5eb4.ts.net/artifacts/a/fleet-retro/daily/index.html"
        );
        // The Mint-proxied upload target and the public report URL returned
        // to callers cover the same shelf path but must never collapse into
        // the same string -- callers that mix them up would either publish
        // through the wrong route or hand the operator a `/proxy/...` link.
        assert_ne!(mint, public);
        assert!(!public.contains("/proxy/"));
    }

    #[test]
    fn mint_base_url_gates_publish_and_is_best_effort_when_missing_or_invalid() {
        // SAFETY: single test function owns every MINT_BASE_URL mutation
        // below sequentially (no other test touches this var), and the
        // original value is restored before returning.
        let original = std::env::var("MINT_BASE_URL").ok();

        unsafe {
            std::env::remove_var("MINT_BASE_URL");
        }
        assert_eq!(mint_base_url(), None, "unset MINT_BASE_URL must skip");

        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("index.html"), "x").unwrap();
        assert_eq!(
            publish_to_shelf("fleet-retro/daily", dir.path()),
            None,
            "publish_to_shelf must skip (not panic or fail) with no MINT_BASE_URL"
        );

        unsafe {
            std::env::set_var("MINT_BASE_URL", "   ");
        }
        assert_eq!(mint_base_url(), None, "blank MINT_BASE_URL must skip");

        unsafe {
            std::env::set_var("MINT_BASE_URL", "ftp://mint.internal");
        }
        assert_eq!(mint_base_url(), None, "non-http(s) MINT_BASE_URL must skip");

        unsafe {
            std::env::set_var("MINT_BASE_URL", "http://127.0.0.1:4949/");
        }
        assert_eq!(
            mint_base_url().as_deref(),
            Some("http://127.0.0.1:4949"),
            "a well-formed MINT_BASE_URL is accepted with its trailing slash trimmed"
        );

        unsafe {
            match &original {
                Some(v) => std::env::set_var("MINT_BASE_URL", v),
                None => std::env::remove_var("MINT_BASE_URL"),
            }
        }
    }

    #[test]
    fn vendor_aesthetic_css_is_best_effort_when_source_missing() {
        let out_dir = tempfile::tempdir().unwrap();
        let fake_home = tempfile::tempdir().unwrap();

        let result = vendor_aesthetic_css(out_dir.path(), fake_home.path());

        assert!(result.is_ok());
        assert!(!out_dir.path().join("aesthetic.css").exists());
    }
}
