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

## Coverage matrix — evidence-based, 2026-07-02

From the showcase evidence pass (cold demos, `a/showcase/`). ✓ proven · ◐ partial ·
✗ missing · ? unverified this pass.

All `?` cells resolved by the 2026-07-02 fleet assessment (`~/.factory-lanes/assess/`).
Skill column reads: in-repo / in harness-kit catalog — the split is the finding.

| Tool | Core | API | CLI | MCP | UI | Skill (repo / catalog) |
|------|------|-----|-----|-----|----|----|
| bitterblossom | ✓ | ✓ `bb serve` | ✓ `bb` | ✓ 9 tools, read-only by construction | ✓ noir-ledger (accepted 07-02) | ◐ / ✗ |
| powder | ✓ | ✓ | ✓ | ✓ 15 tools | ✗ (backlog 006, Kanban) | ✓ / ✗ |
| crucible | ✓ | ◐ adjudication server only | ✓ | ✓ 8 tools | ◐ adjudication panel only | ✓ / ✗ |
| cerberus | ✓ | ✗ exception candidate (rule 2 memo owed) | ✓ | ✓ 3 tools | ✗ | ✓ (×2) / ✗ |
| landmark | ✓ | ✗ named exception — RATIFIED 07-02 (rule 2's worked example) | ✓ + GH Action | ✗ (experiment later, after 007/010) | ✗ | ✗ / ✗ |
| canary | ✓ | ✓ API-first + OpenAPI (+TS SDK) | ✓ | ✓ (052/057, mcp_stdio.rs) | ✗ gap (thesis revisit w/ operator) | ◐ name-drop / ✗ |
| bastion | ✓ | ◐ healthz + per-app | ✓ `bastion status` | ✓ via cairn | ✓ via cairn | ✗ / ✗ |
| harness-kit | ✓ | exception (ratified 07-02) | ✓ | experiment candidate (ticket owed) | ◐ bare docs site | ✓ (it ships skills) |
| aesthetic | ✓ CSS kit v2.8.1 | kit exception: SDK (law/) + JSON feed instead | n/a | n/a | ✓ (site/) | ✗ / ✗ |
| threshold | ✓ | law deferred until graduation (its 066) | ✓ only face | ✗ | ✗ | ✗ / ✗ |
| weave organs (gazette, showcase) | ◐ scripts | ✗ | ◐ | ✗ | ✓ (they ARE pages) | ✗ |

**The skill-column verdict (hk lane, 07-02): zero real per-tool skills in the catalog
for any fleet tool.** Tools carry their own SKILL.md in-repo; the distribution layer
distributes none of them. First children of the adoption epic: author bb + landmark
catalog skills.

Honest readings:

- The **skill column is mostly unverified** — the showcase pass didn't audit
  harness-kit's catalog for per-tool skills. First child of the adoption epic.
- **canary's missing UI was previously defended as thesis** ("agents read it, humans
  don't"). Under this law it's a gap: a thin status/incident UI riding the existing
  API. The API-first architecture makes it cheap.
- **landmark and cerberus have no API face.** Whether each is a named exception (rule
  2) or a gap is a per-repo decision for the epic, not a default.
- The **gazette and showcase** — the fleet's self-reporting organs — currently exist
  as a script + hand-assembled artifacts. In their ultimate form they are Weave
  organs (in this repo or as a distinct service) and subject to this same law.

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
