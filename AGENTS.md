# AGENTS.md — the Weave

The Weave is the opinionated composition layer for the Misty Step
agent-first toolchain: powder, bitterblossom, cerberus, crucible,
threshold, canary, landmark, harness-kit. See `VISION.md` for the loop
these pieces compose into, and `docs/the-five-faces.md` for the
functional-core-with-five-faces contract each piece is held to.

Status: pre-composition. This repo holds contracts, schemas, docs, and tiny
composition services that need to live at the Weave layer. The pieces
themselves are still hardened standalone in their own repos.

## Build / verify

- Gate: `./scripts/verify.sh` — validates JSON well-formedness and schema
  conformance for every fixture under `docs/fixtures/contracts/` against
  `docs/schemas/`, scans fixtures for leaked secrets/hostnames/local paths,
  and runs Rust workspace format, clippy, and tests when app crates exist.
- CI: `.github/workflows/verify.yml` runs the same script on push/PR
  (Ubuntu, Node 20).

## Layout

- `docs/schemas/`, `docs/fixtures/contracts/` — the versioned contract
  schemas and their pass/fail fixtures.
- `docs/*.md` — composition contracts, seam reference, onboarding,
  SDLC-organ promotion criteria, doc-sync flow.
- `apps/release-events/` — public Landmark release-event receiver for the
  Bridge feed, deployed as an independent DigitalOcean service.
- `apps/fleet-retro/` — daily/weekly/arbitrary-window agent-activity retro
  generator; see `docs/fleet-retro.md`.
- `apps/weave-mcp/` — read-only MCP server over fleet-retro and
  release-events; see `.agents/skills/weave-mcp/SKILL.md`.
- `backlog.d/` — numbered backlog items driving this repo's own work.

Repo-local skills: `.agents/skills/weave-mcp/`.
