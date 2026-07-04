# The Five Faces

**Every program in the Weave is a functional core wearing five interfaces: an agent
skill, a CLI, an API, an MCP server, and a UI.** Each face is an independent means of
accessing the core. A tool missing a face is incomplete — the gap is backlog, not
philosophy — *unless the exception is argued and ratified* (see Named exceptions).

Stated by the operator 2026-07-02; refined same day: the law is the default demand,
not a straitjacket. Supersedes the looser parenthetical in VISION.md
("one core → API + CLI + MCP + SDK + skill + thin UI") as the canonical form.

## Topology

```
                 ┌─────────┐
                 │  core   │   pure functionality, no interface opinions
                 └────┬────┘
                      │
                 ┌────▼────┐
                 │   API   │   the spine: in almost every case the core is
                 └─┬──┬──┬─┘   exposed to the other faces via the API
                   │  │  │
        ┌──────────┘  │  └──────────┐
    ┌───▼───┐    ┌────▼────┐    ┌───▼───┐
    │  CLI  │    │   MCP   │    │  UI   │    independent access paths,
    └───┬───┘    └─────────┘    └───────┘    all riding the API
        │
    ┌───▼───────────────────────────────┐
    │  skill: agent instructions for    │    almost always instructions for
    │  using the API or CLI (or SDK)    │    an existing face, not new code
    └───────────────────────────────────┘
```

Rules of construction:

1. **The core is interface-blind.** It owns functionality; it does not know which face
   is calling.
2. **The API is the spine, not a peer.** CLI, MCP, and UI are thin clients of the API
   in almost every case. Exceptions (a CLI that links the core directly for a
   no-server tool like landmark) are legitimate but named.
3. **The skill is prose, not plumbing.** It is agent instructions for driving the API
   or CLI (sometimes an SDK) — the harness-kit pattern. A skill that re-implements
   logic is a bug.
4. **Each face stands alone.** An agent with only MCP, a human with only the UI, a
   script with only the CLI, a service with only the API — all first-class.
5. **SDK is the optional sixth face**, minted when a consumer needs to embed rather
   than call (canary's TS SDK is the exemplar).

## Coverage matrix — evidence-based, refreshed 2026-07-04

From the showcase evidence pass (cold demos, `a/showcase/`) plus the 2026-07-03/04
fleet re-assessment (`~/.factory-lanes/assess/`, `weave-013`). ✓ proven · ◐ partial ·
✗ missing · n/a not applicable / not argued as a face for this tool. No `?` cells —
every tool below has been directly inspected (source, routes, crate graph, or a
live tool call) this pass, not carried over as "unverified."

SDK is the optional sixth face (rule 5) — only listed where a consumer-embed case
has actually been argued; `n/a` elsewhere means no such case has been made, not that
one was checked and rejected. Skill column reads: in-repo / in harness-kit catalog —
the split is a standing finding, not new this pass.

| Tool | Core | API | CLI | MCP | UI | Skill (repo / catalog) | SDK |
|------|------|-----|-----|-----|----|----|-----|
| bitterblossom | ✓ | ✓ `bb serve` | ✓ `bb` | ◐ 9 tools, read-only by construction (`backlog.d/_done/078`) | ✓ noir-ledger | ◐ / ✗ | ✗ |
| powder | ✓ | ✓ `powder-api` | ✓ `powder-cli` | ✓ `powder-mcp` (15 tools, now MCP-over-HTTP) | ✗ (backlog 006, Kanban) | ✓ / ✗ | ✗ |
| crucible | ✓ | ◐ localhost-only by design (`serve.rs`, `adjudication_server.rs`) | ✓ | ✓ 8 tools (`src/mcp.rs`) | ◐ adjudication panel only | ✓ (16k, not yet in catalog) / ✗ | ✗ |
| cerberus | ✓ | ✗ exception candidate (rule 2 memo still owed) | ✓ | ✓ 3 tools | ✗ | ✓ (×2) / ✗ | n/a |
| landmark | ✓ | ✗ named exception — RATIFIED 07-02 (rule 2's worked example) | ✓ + GH Action | ✗ (deliberate — `SKILL.md:45-48`, experiment later) | ✗ | ✓ / ✓ | ✓ crates.io (`cargo publish -p landmark`) |
| canary | ✓ | ✓ API-first + OpenAPI | ✓ | ✓ `canary mcp-server` (stdio, shipped 07-01) | ✗ gap (thesis revisit w/ operator) | ◐ name-drop / ✗ | ◐ **built, DONE-pipeline** (`canary-051`, PR misty-step/canary#231 merged) — publish is a 5-min operator step (npm org + `NPM_TOKEN`), not a code gap |
| bastion | ✓ | ◐ healthz + per-app | ✓ `bastion status` | ✓ via cairn | ✓ via cairn | ✗ / ✗ | n/a |
| harness-kit | ✓ | exception (ratified 07-02) | ✓ | experiment candidate (ticket owed) | ◐ bare docs site | ✓ (it ships skills) | n/a |
| aesthetic | ✓ CSS kit v2.8.1 | ◐ N-A by design, but ticket `aesthetic-021` wants `site/r/*.json` **named** as the API face in docs (undone) | ✗ deliberately sequenced after skill (`aesthetic-021`) | ✗ deliberately sequenced (`aesthetic-021`) | ✓ (site/) | ✓ (repo-local + catalog — closed; `aesthetic-021`'s claim it's still open is stale) | ◐ built (`package.json`, exports map), consumable via git-tag/CDN — **not on npm registry**, operator ruled the ambient 404 unacceptable 07-02, decision still pending (`aesthetic-021`) |
| threshold | ✓ | law deferred until graduation (its 066) | ✓ only face | ✗ | ✗ | ✗ / ✗ | n/a |
| weave organs (gazette, showcase) | ◐ scripts | ✗ | ◐ | ✗ | ✓ (they ARE pages) | ✗ | n/a |
| cairn | ✓ | ✓ `axum::serve`, `Cmd::Serve` | ✓ `Cmd::Habit` | ✓ 7 tools, live-verified this pass (`mcp__cairn__*`) | ✓ `static/` PWA (manifest + service worker) | ✓ repo-local (describes all 4 faces) / ✗ | ✗ |
| memory-engine | ✓ | ✓ `memory-engine-api` (accounts/sources/generate/review-flow/OpenAPI) | ◐ `memory-engine-cli` is a single dogfood-review harness (card 070), no clap/Subcommand operator surface yet | **✓ CLOSED this pass** — `memory-engine-mcp` (`memory-engine-071`, PR misty-step/memory-engine#31): 6 intent-shaped tools (create_deck/list_decks/invalidate_deck/list_due/review_next/submit_answer), cold-agent stdio transcript at `docs/dogfood/mcp-review-loop.md` | ✓ `memory-engine-web-shell` | ◐ only `.agents/skills/memory-engine-qa` (verification-oriented); no product-domain skill | ✗ |
| curb | ✓ | ✗ no HTTP surface (`curb-core` is a library; `ui`/`src-tauri` is a desktop shell) | ✓ real `clap::Subcommand` | ✗ (curb *monitors* other apps' MCP servers; exposes none of its own) | ✓ `ui/` Tauri desktop shell | **✓ CLOSED earlier this pass** — `curb-905`, repo skill covering status/watch, cold-agent transcript | ✗ |
| counterspell | ✓ | ✗ no server code | ✓ real `clap::Subcommand` (`src/cli.rs`) | ✗ | ◐ SwiftBar menubar plugin (`extras/swiftbar/counterspell.5m.sh`) only, not a full UI | ✗ not in catalog, no repo `SKILL.md` | ✗ |
| standby | ✓ | ✓ `standbyd` axum `Router` (meetings/jobs/capture) | **✓ CLOSED this pass** — `standby-024`, PR misty-step/standby#10: `meetings list/show`, `proposals approve/ignore`, `capture start/stop`, `results open`, e2e receipt at `docs/evidence/cli-face/` | ✗ (VISION.md names it a future capability, not shipped) | ✓ React `ui/` (per `AGENTS.md`) | ✗ no `SKILL.md` | ✗ |

**Progress this pass (weave-013, 2026-07-04):** of the prior "top 5 highest-leverage
closures," four are now shipped or pipeline-complete — memory-engine MCP (built),
standby CLI (built), curb skill (built, closed earlier the same day), canary SDK
(code + publish pipeline done, npm org creation is the sole remaining step and is an
operator action, not an agent one — confirmed no local `NPM_TOKEN`/npm login exists
to do it headlessly). Only aesthetic's SDK-publish decision remains genuinely open,
and it is explicitly bundled with the same operator npm session as canary's.

**The skill-column verdict (hk lane, 07-02, still true): zero real per-tool skills in
the harness-kit catalog for any fleet tool.** Tools carry their own SKILL.md in-repo;
the distribution layer distributes none of them. curb and cairn both closed their
*repo-local* skill gap this week; the catalog side is untouched.

Honest readings:

- **canary's missing UI was previously defended as thesis** ("agents read it, humans
  don't"). Under this law it's a gap: a thin status/incident UI riding the existing
  API. The API-first architecture makes it cheap. Still open — not touched this pass.
- **landmark and cerberus have no API face.** landmark's is a ratified exception;
  cerberus's rule-2 memo is still owed — a gap, not yet a decision.
- The **gazette and showcase** — the fleet's self-reporting organs — currently exist
  as a script + hand-assembled artifacts, and per operator directive 2026-07-03 the
  gazette is being collapsed into the Bridge rather than independently maturing its
  own five faces (see weave `collapse-gazette-into-bridge` card) — treat this row as
  transitional, not a stable target for face-closure work.
- **curb, counterspell, and standby remain the least-covered tools** (1-2 solid faces
  each even after this pass's closures) — proportionate to their youth, not neglect.
  counterspell in particular has zero face-closure backlog filed beyond an icon
  ticket; that absence is itself a finding, not evidence of a low bar.
- **SDK/npm publication is the fleet's single most common remaining gap** even where
  the code is done (canary, aesthetic both built-and-tested, both 404 on npm) — the
  pattern is a packaging/operator-action gap, not an engineering one.

## Named exceptions (ratified)

Exceptions are argued per-repo and recorded here — never defaulted into.

- **harness-kit: no API — ratified 2026-07-02.** The product is borderline a pile of
  skill directories: prepackaged, batteries-included instructions (SKILL.md +
  references/ + scripts/), not a service. Nothing an API would serve that the
  filesystem and CLI don't already. Its faces are: skill (it *ships* skills), CLI
  (bootstrap/checks), UI (the docs site — still owed polish), and MCP as an
  **experiment, not an obligation** — see below.

- **landmark: no API — ratified 2026-07-02.** Rule 2's own worked example promoted to
  a recorded exception: a no-server CLI whose core links directly; the composite GH
  Action is its service face. MCP (release-notes-on-demand) stays a later experiment,
  sequenced after its 007/010 action-thinning work.

### The harness-kit MCP experiment

Worth running, not yet owed: an MCP face over the skill catalog, in the spirit of
skills.sh-style skill libraries. The pull: the end-state vision is a curated set of
high-quality, demonstrably useful skills — ours or externally sourced (skills.sh and
other published catalogs) — that orchestrator agents query to compose bespoke
harnesses: defining agents (e.g. in bitterblossom) whose harness is assembled from a
subset of the catalog. An MCP face is how an orchestrator browses/pulls that catalog
at composition time. Experiment ticket, evidence before promotion (crucible measures
whether catalog-composed harnesses beat hand-rolled ones).

## Relationship to other doctrine

- Extends VISION.md doctrine (tools-for-ourselves, model-native judgment,
  product/instance law). A face is still subject to all of them.
- The seam contracts (`composition-contracts.md`) govern tool↔tool; the five faces
  govern tool↔consumer. A seam schema may ride the API face.
- Organ promotion (`sdlc-organ-promotion.md`) applies to new organs; a promoted organ
  arrives owing five faces.
