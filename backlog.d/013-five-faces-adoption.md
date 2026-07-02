# Close the five-faces gaps across the fleet

Priority: P1 · Status: pending · Estimate: XL (epic)

## Goal
Every fleet tool exposes its core through all five faces — skill, CLI, API, MCP, UI —
per docs/the-five-faces.md, with named exceptions ratified by the operator rather than
defaulted into.

## Oracle
- [ ] The coverage matrix in docs/the-five-faces.md has no `?` cells — every cell
      verified against the live repo.
- [ ] Every ✗ cell is either (a) closed by a shipped face, or (b) converted to a
      named exception with operator sign-off recorded in the matrix.
- [ ] Each new face is a thin client of the tool's API (or a documented rule-2
      exception) — no logic re-implementation.

## Children
1. Verify the skill column: audit harness-kit's catalog for per-tool skills
   (bb/crucible/cerberus/landmark/canary/bastion); write the missing ones as
   API/CLI instructions per rule 3.
2. powder UI — the "gorgeous Kanban" (existing backlog 006 in powder; this epic
   tracks it, doesn't duplicate it).
3. canary thin UI — status + incidents riding the existing API. Revisit the
   "no dashboard by design" thesis with the operator first.
4. cerberus + landmark API decision — named exception or new face, one memo each.
5. crucible API/UI completion — generalize beyond the adjudication server.
6. harness-kit MCP + docs-site face (the bare GH Pages site is the UI gap).
7. MCP coverage verification for canary; bb MCP write-tools decision (read-only today).

## Notes
Born from the operator's 2026-07-02 statement of the law plus the showcase evidence
pass (a/showcase/ packets are the per-repo ground truth). Sequence children by daily
operator pain, not matrix completeness — the matrix is the map, not the priority order.
**Why:** operator doctrine 2026-07-02; coverage matrix from cold-run showcase evidence.
