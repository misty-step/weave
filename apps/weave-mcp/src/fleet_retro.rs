use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

/// Locate the weave workspace's root `Cargo.toml` from `weave-mcp`'s own
/// build location (`apps/weave-mcp` -> up two levels), so `run_fleet_retro`
/// can invoke the sibling `weave-fleet-retro` crate via `cargo run
/// --manifest-path` without requiring a separately-installed binary path or
/// extra configuration. This only works when weave-mcp is built as a
/// workspace member (the normal case); `WEAVE_ROOT` overrides it for any
/// other layout.
fn workspace_manifest_path() -> PathBuf {
    if let Ok(root) = std::env::var("WEAVE_ROOT") {
        return PathBuf::from(root).join("Cargo.toml");
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(|root| root.join("Cargo.toml"))
        .unwrap_or_else(|| PathBuf::from("Cargo.toml"))
}

/// Trigger a fleet-retro assembly run and return the assembled `RetroSpec`
/// as JSON. Always runs with `--dry-run` -- this never publishes to the
/// bastion shelf or posts a Bridge feed entry, matching bitterblossom's own
/// MCP-dispatch-off-by-default caution (a real publish stays a CLI/
/// LaunchAgent-only action until an operator explicitly signs off on
/// MCP-driven publication). `window` is `daily`, `weekly`, or `custom`
/// (with `since`/`until` required for `custom`).
pub fn run_fleet_retro(
    window: &str,
    since: Option<&str>,
    until: Option<&str>,
    bb_plane: Option<&str>,
) -> Result<Value, String> {
    let manifest_path = workspace_manifest_path();
    let mut args: Vec<String> = vec![
        "run".into(),
        "--quiet".into(),
        "--release".into(),
        "--manifest-path".into(),
        manifest_path.to_string_lossy().into_owned(),
        "-p".into(),
        "weave-fleet-retro".into(),
        "--bin".into(),
        "fleet-retro".into(),
        "--".into(),
        "--window".into(),
        window.into(),
        "--dry-run".into(),
    ];
    if let Some(since) = since {
        args.push("--since".into());
        args.push(since.into());
    }
    if let Some(until) = until {
        args.push("--until".into());
        args.push(until.into());
    }
    if let Some(plane) = bb_plane {
        args.push("--bb-plane".into());
        args.push(plane.into());
    }

    let output = Command::new("cargo")
        .args(&args)
        .output()
        .map_err(|err| format!("failed to run cargo: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "fleet-retro exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|err| format!("fleet-retro dry-run output was not valid JSON: {err}"))
}

/// Read the most recently generated `spec.json` under `fleet_retro_dir`
/// (default `~/.factory-lanes/fleet-retro`), optionally filtered to one
/// window label (`daily`/`weekly`). This reads whatever the last real
/// (non-dry-run) CLI or LaunchAgent run actually published -- it is a pure
/// filesystem read, not a re-assembly, so it reflects exactly what shipped.
pub fn get_latest_fleet_retro(
    fleet_retro_dir: &Path,
    window: Option<&str>,
) -> Result<Value, String> {
    let entries = std::fs::read_dir(fleet_retro_dir)
        .map_err(|err| format!("reading {}: {err}", fleet_retro_dir.display()))?;
    let mut candidates: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .filter(|path| {
            window.is_none_or(|w| {
                path.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with(&format!("{w}-")))
            })
        })
        .collect();
    candidates.sort();
    let Some(latest) = candidates.pop() else {
        return Err(format!(
            "no fleet-retro output found under {}{}",
            fleet_retro_dir.display(),
            window
                .map(|w| format!(" for window '{w}'"))
                .unwrap_or_default()
        ));
    };
    let spec_path = latest.join("spec.json");
    let contents = std::fs::read_to_string(&spec_path)
        .map_err(|err| format!("reading {}: {err}", spec_path.display()))?;
    serde_json::from_str(&contents)
        .map_err(|err| format!("{} was not valid JSON: {err}", spec_path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn get_latest_fleet_retro_picks_the_lexicographically_last_matching_dir() {
        let root = tempfile::tempdir().unwrap();
        for (name, commits) in [
            ("daily-2026-07-04T210000Z", 10),
            ("daily-2026-07-05T210000Z", 20),
            ("weekly-2026-07-05T210000Z", 99),
        ] {
            let dir = root.path().join(name);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join("spec.json"),
                serde_json::to_string(&json!({"commits": commits})).unwrap(),
            )
            .unwrap();
        }

        let latest_daily = get_latest_fleet_retro(root.path(), Some("daily")).unwrap();
        assert_eq!(latest_daily["commits"], 20);

        let latest_any = get_latest_fleet_retro(root.path(), None).unwrap();
        assert_eq!(latest_any["commits"], 99);
    }

    #[test]
    fn get_latest_fleet_retro_errors_clearly_when_nothing_matches() {
        let root = tempfile::tempdir().unwrap();
        let result = get_latest_fleet_retro(root.path(), Some("weekly"));
        assert!(result.is_err());
    }
}
