# Fleet retro

`apps/fleet-retro` generates a retro over an arbitrary time window covering
everything the fleet's agents did: git commits/PRs per repo, Powder card
movements, Bitterblossom plane runs, and Bridge feed events. It renders the
result through a spec-first page spec (`src/spec.rs`) and a deterministic
HTML renderer (`src/render.rs`) styled with the Misty Step Aesthetic, then
pushes it to the bastion artifact shelf and posts a `kind=report` entry to
the Bridge feed so it shows up at Sanctum → Bridge.

Built for weave-908 (operator directive 2026-07-04): a daily ~21:00 + weekly
Sunday + arbitrary-window retro that "very clearly and accurately describes
everything that all of our agents across the weave did across all of our
applications over the past 24 hours."

## Why this shape

- **Spec-first, not template-first.** `assemble.rs` turns collected evidence
  into a validated `RetroSpec` before any HTML exists; `render.rs` is a pure
  function of that spec. This is the same pattern glance-gen's
  `PageSpec`/`Component` catalog uses (prior art named directly in the
  weave-908 card, since misty-step-911 -- which repo owns the shared
  report-rendering primitive -- was unresolved at claim time). The renderer
  seam is narrow (`render::render_html(&RetroSpec) -> String`), so retargeting
  a future shared primitive means swapping one function, not rewriting the
  collectors.
- **Sources named per claim.** Every collector attaches a `source` string
  (`git:/path/to/repo`, `powder:card:landmark-907`, `bb:<plane>`,
  `feed:/path/day.jsonl`) to what it produces, and the assembled spec's
  `Provenance` component is a required, structurally-enforced last component
  (`spec.rs::validate`) -- a retro that can't name where a claim came from
  fails validation, it doesn't render silently.
- **Explicit gaps, not silent omissions.** An unconfigured source (no
  `--bb-plane`, no `POWDER_API_BASE_URL`) reports "not configured" as a
  provenance note. A quiet repo (swept, zero commits) is named in the
  provenance, not left out. This is what "accuracy beats coverage" means in
  the card: a narrower honest retro over a false-confident one.

## Running it

```bash
# On demand, any window, print the assembled spec without rendering:
cargo run --release -p weave-fleet-retro -- --window daily --dry-run
cargo run --release -p weave-fleet-retro -- --window custom --since 2026-07-01T00:00:00Z --until 2026-07-02T00:00:00Z --dry-run

# Render locally without publishing:
cargo run --release -p weave-fleet-retro -- --window daily --out /tmp/retro --no-publish

# Full run: render, publish to the shelf, post to the Bridge feed:
cargo run --release -p weave-fleet-retro -- --window daily
cargo run --release -p weave-fleet-retro -- --window weekly

# Scheduled mode (what the LaunchAgent calls): always daily, plus weekly on
# Sundays, in one process invocation:
cargo run --release -p weave-fleet-retro -- --scheduled
```

`POWDER_API_BASE_URL`/`POWDER_API_KEY`/`ARTIFACTS_API_TOKEN` are read from
the environment first, falling back to `~/.secrets` (`src/secrets.rs`) --
required because the LaunchAgent-scheduled run does not inherit an
interactively-sourced shell environment. Never printed, never embedded in
generated output.

## Published locations

Daily and weekly windows publish to **distinct** shelf paths so a Sunday
weekly run never overwrites the daily page underneath it -- the operator
needs to read the last daily *and* the last weekly retro simultaneously
(acceptance criterion 5), which requires both to persist side by side:

- `https://bastion.tail5f5eb4.ts.net/artifacts/a/fleet-retro/daily/index.html`
- `https://bastion.tail5f5eb4.ts.net/artifacts/a/fleet-retro/weekly/index.html`

Each publish also posts a `kind=report` entry to
`~/.factory-lanes/feed/*.jsonl` via the existing `feed-post` script, which
`bridge.py` (the Bridge page generator, `~/.factory-lanes/scripts/bridge.py`)
already renders generically -- no bridge.py changes were needed.

## Scheduling

`~/Library/LaunchAgents/com.phaedrus.fleet-retro.plist` runs
`cargo run --release -p weave-fleet-retro -- --scheduled` daily at 21:00
local, following the same `StartCalendarInterval` pattern as the retired
`nightly-digest`/`bridge-regen` agents. One calendar trigger covers both
cadences: the binary always generates the daily retro, and additionally the
weekly retro when `RetroWindow::is_weekly_day` says today is Sunday.

## Supersedes nightly-digest

Per the weave-908 card ("SEED: the existing fleet-digest feed poster... grow
it into the real retro, or replace it; do not run both"), `fleet-retro`
replaces `~/.factory-lanes/scripts/nightly-digest.py` and its LaunchAgent
(`com.phaedrus.nightly-digest.plist`, unloaded and moved to Trash 2026-07-05).
nightly-digest only swept a hardcoded repo list's git log at 07:00 and posted
a plain digest note; fleet-retro dynamically discovers every repo checkout
under `~/Development`, adds Powder/bb/feed sources, and renders a real page
instead of a feed-only text blob. The script file itself
(`nightly-digest.py`) was left in place, unscheduled, in case anything else
references it.

## Data sources and known gaps

| Source | Collector | Notes |
| --- | --- | --- |
| Git commits/PRs | `sources::git` | Discovers every git checkout under `--dev-root` (default `~/Development`) dynamically; `--first-parent` log, `(#123)`/`Merge pull request #123` parsing for PR references. |
| Powder card movements | `sources::powder` | HTTP `list_cards` + `get_card` per card, filtered to events/comments in-window. Falls back to the `<repo>-<number>` card-id convention when a card's own `repo` field is empty. |
| Bitterblossom plane runs | `sources::bb` | Shells out to `bb --config <plane> runs list --json`. No default plane is assumed -- pass `--bb-plane <path>` (or `FLEET_RETRO_BB_PLANE`) explicitly; there is no single fleet-wide plane in active use as of this writing (`bb-dashboard/plane` exists locally but had zero run history at build time). |
| Bridge feed events | `sources::feed` | Reads `~/.factory-lanes/feed/*.jsonl`, filtering to feed-post's known `kind` set. **Important:** `~/.factory-lanes/feed/*.jsonl` is shared with at least one other producer (`counterspell`'s `weave.remote_event.v1` session-routing telemetry) whose lines are valid JSON but not feed-post entries -- the parser filters on a closed `kind` allowlist rather than "did it parse as JSON" specifically to exclude that noise (`sources::feed::tests::skips_foreign_schema_lines_sharing_the_same_file` is the regression test for this, built from a real line observed in `~/.factory-lanes/feed/2026-07-05.jsonl`). |
| Deploys | *(not yet a dedicated source)* | Surfaces indirectly today via feed entries whose title/body mention a deploy, and via PR merges. A dedicated Fly/deploy-log collector is a natural follow-up but was out of scope for this pass; noted here rather than silently claimed as covered. |

## Tests

`cargo test -p weave-fleet-retro` covers: window arithmetic (half-open
`contains`, weekly-day detection), the git collector against a disposable
fixture repo (including the discovery that git's `--since`/`--until` date
parser silently rejects years >= 2100 -- see the comment in
`sources::git::collect_repo_activity`), the feed parser's schema-filtering
against a real foreign-schema line, Powder movement extraction (including a
regression for a UTF-8 char-boundary panic on multi-byte comment text found
on the first live run against tonight's actual data), bb's flexible JSON
shape handling, spec validation, and renderer escaping/empty-state behavior.
