use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::{DateTime, Utc};
use regex::Regex;
use serde::Serialize;

use crate::window::RetroWindow;

/// One commit reachable on a repo's default branch within the window.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RepoCommit {
    pub hash: String,
    pub subject: String,
    pub pr_number: Option<String>,
    pub at: String,
}

/// Everything a single local checkout contributed to the window: commits
/// (deduplicated of the "Merge pull request" bookkeeping commits, matching
/// nightly-digest.py's convention) plus the distinct PR numbers referenced
/// by those commits' `(#123)` suffixes or merge-commit bodies.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RepoActivity {
    pub repo: String,
    pub source: String,
    pub commits: Vec<RepoCommit>,
    pub pr_numbers: Vec<String>,
}

impl RepoActivity {
    pub fn is_empty(&self) -> bool {
        self.commits.is_empty() && self.pr_numbers.is_empty()
    }
}

/// Discover fleet repos to sweep: every subdirectory of `dev_root` that is a
/// git checkout. This is deliberately dynamic rather than a hardcoded repo
/// list (nightly-digest.py's approach) -- a repo added to ~/Development
/// shows up in the next retro without a code change, and a repo that gets
/// archived or moved silently drops out instead of erroring.
pub fn discover_repos(dev_root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dev_root) else {
        return Vec::new();
    };
    let mut repos: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir() && path.join(".git").exists())
        .collect();
    repos.sort();
    repos
}

fn pr_number_regex() -> Regex {
    Regex::new(r"\(#(\d+)\)\s*$").expect("static PR-number pattern is valid")
}

fn merge_pr_regex() -> Regex {
    Regex::new(r"^Merge pull request #(\d+)").expect("static merge-PR pattern is valid")
}

/// Collect one repo's first-parent commit history within `window` via a
/// real `git log` invocation. `--first-parent` matches the fleet's squash-
/// or-merge-commit convention (seen throughout landmark/canary/powder
/// history) so a feature branch's internal commits don't get double-counted
/// once the branch lands.
///
/// Known git limitation (verified live, git 2.54.0): `--until`/`--since`
/// silently fail to match anything once the year reaches 2100 -- no error,
/// just zero commits. Retro windows are always daily/weekly/near-present,
/// so this never bites in practice, but a caller constructing a `--custom`
/// window with a far-future `until` should not expect it to work.
pub fn collect_repo_activity(
    repo_path: &Path,
    window: &RetroWindow,
) -> anyhow::Result<RepoActivity> {
    let name = repo_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| repo_path.display().to_string());

    let since = window.since.to_rfc3339();
    let until = window.until.to_rfc3339();
    let output = Command::new("git")
        .args([
            "log",
            "--first-parent",
            &format!("--since={since}"),
            &format!("--until={until}"),
            "--pretty=%H\x1f%s\x1f%aI",
        ])
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "git log failed in {}: {}",
            repo_path.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let pr_re = pr_number_regex();
    let merge_re = merge_pr_regex();
    let mut commits = Vec::new();
    let mut prs = std::collections::BTreeSet::new();

    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let mut fields = line.splitn(3, '\u{1f}');
        let (Some(hash), Some(subject), Some(at)) = (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        if let Some(caps) = merge_re.captures(subject) {
            prs.insert(caps[1].to_string());
            continue;
        }
        if let Some(caps) = pr_re.captures(subject) {
            prs.insert(caps[1].to_string());
        }
        commits.push(RepoCommit {
            hash: hash.to_string(),
            subject: subject.to_string(),
            pr_number: pr_re.captures(subject).map(|c| c[1].to_string()),
            at: at.to_string(),
        });
    }

    Ok(RepoActivity {
        repo: name,
        source: format!("git:{}", repo_path.display()),
        commits,
        pr_numbers: prs.into_iter().collect(),
    })
}

pub fn parse_commit_time(commit: &RepoCommit) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(&commit.at)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

// Kept for readability at call sites that want an explicit "this is a real
// instant, not a guess" constructor in tests. Test-only across the whole
// crate (feed.rs/powder.rs/bb.rs/assemble.rs all import it under their own
// #[cfg(test)] modules), so it is gated the same way here.
#[cfg(test)]
pub fn ts(y: i32, m: u32, d: u32, h: u32, mi: u32, s: u32) -> DateTime<Utc> {
    use chrono::TimeZone;
    Utc.with_ymd_and_hms(y, m, d, h, mi, s).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::RetroWindow;
    use std::process::Command;
    use tempfile::TempDir;

    fn run(cmd: &[&str], cwd: &Path) {
        let status = Command::new(cmd[0])
            .args(&cmd[1..])
            .current_dir(cwd)
            .status()
            .expect("command runs");
        assert!(status.success(), "command failed: {cmd:?}");
    }

    fn init_fixture_repo() -> TempDir {
        let dir = tempfile::tempdir().unwrap();
        run(&["git", "init", "-q"], dir.path());
        run(&["git", "config", "user.name", "Retro Test"], dir.path());
        run(
            &["git", "config", "user.email", "retro@example.invalid"],
            dir.path(),
        );
        std::fs::write(dir.path().join("a.txt"), "one\n").unwrap();
        run(&["git", "add", "a.txt"], dir.path());
        run(
            &["git", "commit", "-q", "-m", "feat: add first thing (#12)"],
            dir.path(),
        );
        std::fs::write(dir.path().join("b.txt"), "two\n").unwrap();
        run(&["git", "add", "b.txt"], dir.path());
        run(
            &[
                "git",
                "commit",
                "-q",
                "-m",
                "Merge pull request #34 from x/y",
            ],
            dir.path(),
        );
        dir
    }

    #[test]
    fn discover_repos_finds_only_git_checkouts() {
        let dev_root = tempfile::tempdir().unwrap();
        let repo = dev_root.path().join("real-repo");
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        std::fs::create_dir_all(dev_root.path().join("not-a-repo")).unwrap();

        let repos = discover_repos(dev_root.path());

        assert_eq!(repos, vec![repo]);
    }

    #[test]
    fn collect_repo_activity_captures_commits_and_pr_numbers_in_window() {
        let repo = init_fixture_repo();
        // Note: git's date parser silently rejects --until years >= 2100
        // (verified live: 2099 matches, 2100 returns zero commits with no
        // error), so this uses a comfortably-future-but-parseable bound
        // rather than a symbolic "end of time" value.
        let window = RetroWindow::custom(ts(2000, 1, 1, 0, 0, 0), ts(2099, 1, 1, 0, 0, 0)).unwrap();

        let activity = collect_repo_activity(repo.path(), &window).unwrap();

        assert_eq!(
            activity.commits.len(),
            1,
            "merge commit is excluded from the commit list"
        );
        assert_eq!(activity.commits[0].subject, "feat: add first thing (#12)");
        assert_eq!(
            activity.pr_numbers,
            vec!["12".to_string(), "34".to_string()]
        );
    }

    #[test]
    fn collect_repo_activity_outside_window_is_empty() {
        let repo = init_fixture_repo();
        let window = RetroWindow::custom(ts(1990, 1, 1, 0, 0, 0), ts(1990, 1, 2, 0, 0, 0)).unwrap();

        let activity = collect_repo_activity(repo.path(), &window).unwrap();

        assert!(activity.is_empty());
    }
}
