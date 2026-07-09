# Fleet retro

`apps/fleet-retro` generates a retro over an arbitrary time window covering
everything the fleet's agents did: git commits/PRs per repo, Powder card
movements, Bitterblossom plane runs and moment-scorer anomaly cards, Bridge
feed events, and campaign receipts. Every collector projects into a
versioned `EvidencePack` (`src/pack.rs`, `weave.evidence-pack.v1` -- see the
schema changelog), which a model synthesis stage (`src/synthesis.rs`) turns
into a cited significance-ranked narrative gated by a deterministic citation
gate (`src/citation_gate.rs`), and `assemble.rs` turns the pack plus that
narrative into a spec-first page spec (`src/spec.rs`) rendered by a
deterministic HTML renderer (`src/render.rs`) styled with the Misty Step
Aesthetic, then pushed to the bastion artifact shelf with a `kind=report`
entry posted to the Bridge feed so it shows up at Sanctum → Bridge.

Built for weave-908 (operator directive 2026-07-04): a daily ~21:00 + weekly
Sunday + arbitrary-window retro that "very clearly and accurately describes
everything that all of our agents across the weave did across all of our
applications over the past 24 hours." Extended by weave-920 (Evidence
Pipeline v2, operator directive 2026-07-06): collectors read sources of
truth at synthesis time (pull-federation, not event-sourcing -- no mirrors,
no capture daemons), pack into one versioned intermediate (weave-922), then
(weave-923) a model writes the narrative and a deterministic gate proves
every claim cites a real pack item -- the same spine this repo's other
report kinds (briefings, incidents, audits) can ride too.

## Synthesis stage + citation gate (weave-923)

The report's lead section is no longer a formatted log -- it's a model-
written narrative (`synthesis::synthesize`) that reads the whole
`EvidencePack` and writes what mattered, significance-ranked, every claim
carrying an inline `[id]` citation. Routing is cheap-default/escalate-on-
failure, oracle findings ruled binding 2026-07-06:

1. Try a cheap OpenRouter model (`deepseek/deepseek-v4-flash`) twice --
   the second attempt absorbs a transient formatting miss, not a capability
   gap.
2. Escalate once to a stronger model (`moonshotai/kimi-k2.7-code`) if both
   cheap attempts failed the gate.
3. **Fail open** to the deterministic tables-only report with a visible
   banner if all three attempts either can't reach the model or keep
   failing the gate. Never a half-cited narrative, never more than three
   attempts.

The citation gate (`citation_gate::validate_citations`) is deterministic,
external to the synthesis prompt, and stays that way forever (Governance
Decay, arXiv:2606.22528, cited in the oracle research this card's design
rests on) -- it is tier 1 of a cheapest-first cascade only: an existence
check (every `[id]` token must name a real pack item; catches most
fabrication per CiteCheck, arXiv:2605.27700) plus a structural check that
every non-heading narrative line carries at least one citation. Claim-level
entailment (does the cited item actually *support* this specific sentence)
is an explicitly deferred later child, not folded into this gate.

Every run's judge model, gate outcome, prompt version
(`synthesis::PROMPT_VERSION`), pack schema version, and pack-assembly
latency ride the rendered report's footer (`Component::Footer`) --
diagnosability convention borrowed from SRE postmortems: snapshot every
input version so a bad report can be traced back later. Pack-assembly
latency is also appended to a durable `pack-assembly-latency.jsonl` starting
from the very first run (`--metrics-dir`/`FLEET_RETRO_METRICS_DIR`) -- it is
the named falsifier for pull-federation: if it ever exceeds report cadence,
the fix is a cached pull snapshot, not event-sourcing.

Live-verified (not just unit-tested): a real run against live fleet data
produced a genuine fabricated citation on attempt 1, correctly rejected by
the gate, then a legitimate gate-passing narrative on attempt 2 -- proving
the escalation-before-failure routing end to end against a real model, not
a scripted double. A second live run with `OPENROUTER_API_KEY` unset proved
the fail-open path renders the deterministic tables-only report with the
banner. See `~/.factory-lanes/campaign/weave-923-evidence/` for the captured
HTML/evidence-pack artifacts from both runs.

## Why this shape

- **Spec-first, not template-first.** `assemble.rs` turns an `EvidencePack`
  into a validated `RetroSpec` before any HTML exists; `render.rs` is a pure
  function of that spec. This is the same pattern glance-gen's
  `PageSpec`/`Component` catalog uses (prior art named directly in the
  weave-908 card, since misty-step-911 -- which repo owns the shared
  report-rendering primitive -- was unresolved at claim time). The renderer
  seam is narrow (`render::render_html(&RetroSpec) -> String`), so retargeting
  a future shared primitive means swapping one function, not rewriting the
  collectors.
- **One versioned intermediate between collectors and everything
  downstream.** Every collector projects its native output into
  `pack::EvidenceItem`s (`{id, ts, source, kind, title, refs, excerpt}`)
  rather than `assemble.rs` reading five differently-shaped collector
  structs directly. A new collector, or a future report kind (briefing,
  incident, audit) built on the same pipeline, extends the pack instead of
  touching every downstream consumer. `refs` is a small ad-hoc tag list
  (`"repo:landmark"`, `"card:landmark-907"`) rather than per-source struct
  fields, so a consumer needing source-specific structure (this repo's
  per-repo rollups) parses the tags it recognizes without the schema itself
  growing per-source fields.
- **Sources named per claim.** Every collector attaches a `source` string
  (`git:/path/to/repo`, `powder:card:landmark-907`, `bb:<plane>`,
  `feed:/path/day.jsonl`, `receipt:/path/to/file.md`) to what it produces,
  and `assemble.rs` dispatches on that prefix before `kind` (feed-post's own
  `KNOWN_KINDS` reserves `"receipt"` as a valid feed-post kind, which
  collides with the campaign-receipts collector's own `"receipt"` kind --
  the source prefix, not `kind` alone, resolves it). The assembled spec's
  `Provenance` component is a required, structurally-enforced last component
  (`spec.rs::validate`) -- a retro that can't name where a claim came from
  fails validation, it doesn't render silently.
- **Explicit gaps, not silent omissions.** An unconfigured source (no
  `--bb-plane`, no `POWDER_API_BASE_URL`) reports "not configured" as a
  provenance note. A quiet repo (swept, zero commits) still gets a
  `repo-swept` pack item and an all-zero `RepoActivityRow`, not silent
  absence. This is what "accuracy beats coverage" means in the card: a
  narrower honest retro over a false-confident one.
- **Refactors of this pipeline are regression-tested by rendered-HTML byte
  identity, not just unit tests.** weave-922 (extracting the pack) captured
  a baseline `index.html` for a fixed past window before the refactor and
  diffed the post-refactor render against it byte-for-byte, over a frozen
  snapshot of the collector inputs (a live shared directory like
  `~/.factory-lanes/campaign/` keeps changing, so a true regression check
  freezes a copy rather than re-reading the moving live state twice). Any
  future change to `pack.rs` or `assemble.rs` should do the same before
  merging.

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

- `https://sanctum.tail5f5eb4.ts.net/artifacts/a/fleet-retro/daily/index.html`
- `https://sanctum.tail5f5eb4.ts.net/artifacts/a/fleet-retro/weekly/index.html`

Each run also writes `spec.json` (the assembled `RetroSpec`) and
`evidence-pack.json` (the versioned `EvidencePack` the spec was assembled
from) as siblings of `index.html`, and both ride the same shelf publish
path -- the pack is meant to be the citation gate's (weave-923) ground
truth wherever the report lands, not something only readable from the local
output directory.

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
| Campaign receipts | `sources::receipts` | Reads `~/.factory-lanes/campaign/*.md` (157+ files as of weave-921), the fleet's richest narrative source until now consumed by nothing. Reads a minimal frontmatter block (`ts`, `cards`; see the factory-ops repo's `docs/posting-contract.md`) new receipts write and backfilled receipts were given by `~/.factory-lanes/scripts/backfill-receipt-frontmatter.py`, falling back to file mtime for receipts that predate the convention. Renders as a dedicated "Receipts" section (title + ~40-word excerpt + cards), not folded into the timeline. |
| Moment-scorer anomaly cards | `sources::moments` | Shells out to `python3 <script> list --moments-db <db> --json`, reading Bitterblossom's flight-recorder scorer's own published (already ≤3/day-capped) review queue as an external contract. No single fleet-wide moments store exists yet -- pass `--bb-plane` (derives `<plane>/.bb/moments.db`) or explicit `--moment-scorer-script`/`--moments-db` (`FLEET_RETRO_MOMENT_SCORER_SCRIPT`/`FLEET_RETRO_MOMENTS_DB`). Rolls up into the same `bb:{task}` repo bucket a regular bb-run item uses; foregrounded explicitly in the synthesis prompt as curated anomaly signal. |
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
shape handling, the receipts collector's frontmatter parsing and mtime
fallback (fixture-based, real tempdirs, no mocks), the evidence pack's
per-collector projection and cross-source ordering (`pack::tests`,
including a regression for the feed/receipt `kind` collision), assemble's
reconstruction of repo rollups/timeline/notes from pack items (including a
merge-only-PR and an all-zero quiet-repo case), spec validation, and
renderer escaping/empty-state behavior. Beyond the unit suite, weave-922's
introduction of the pack was checked against a byte-identical rendered-HTML
diff over a frozen snapshot of live collector inputs, not unit tests alone
-- see "Why this shape" above.
