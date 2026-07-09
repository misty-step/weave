use std::path::Path;
use std::process::Command;

/// Mirror `local_dir` to the Sanctum artifact shelf at
/// `https://sanctum.tail5f5eb4.ts.net/artifacts/a/<slug>/`, matching
/// bridge.py's `publish_to_shelf`. Best-effort: a failed PUT is logged to
/// stderr and does not fail the run, since the local `local_dir` output is
/// canonical -- the shelf copy is delivery, not the source of truth. Returns
/// the public URL for `index.html` when at least one file published.
pub fn publish_to_shelf(slug: &str, local_dir: &Path) -> Option<String> {
    let Some(token) = crate::secrets::env_or_secrets_file("ARTIFACTS_API_TOKEN") else {
        eprintln!("fleet-retro: ARTIFACTS_API_TOKEN not set; skipping shelf publish");
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
        let url = format!("{base}/{rel_str}");
        match ureq::put(&url)
            .set("Authorization", &format!("Bearer {token}"))
            .send_bytes(&bytes)
        {
            Ok(_) => published_any = true,
            Err(err) => eprintln!("fleet-retro: shelf publish failed for {rel_str}: {err}"),
        }
    }
    published_any.then(|| format!("{base}/index.html"))
}

fn shelf_base(slug: &str) -> String {
    format!("https://sanctum.tail5f5eb4.ts.net/artifacts/a/{slug}")
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
    fn vendor_aesthetic_css_is_best_effort_when_source_missing() {
        let out_dir = tempfile::tempdir().unwrap();
        let fake_home = tempfile::tempdir().unwrap();

        let result = vendor_aesthetic_css(out_dir.path(), fake_home.path());

        assert!(result.is_ok());
        assert!(!out_dir.path().join("aesthetic.css").exists());
    }
}
