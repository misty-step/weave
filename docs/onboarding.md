# Onboarding

A guided path from "what is the Weave?" to "I can work a ticket." Read in order;
each doc builds on the last.

## 1. What the Weave is

Read [VISION.md](../VISION.md). The Weave is the composition layer for the
Misty Step toolchain: an opinionated loop of agent-first development tools.
Each tool is built standalone first; the Weave stitches them into one
software-development loop with versioned contracts at every seam.

The eight pieces:

| Piece | Role |
| --- | --- |
| powder | Work state. Tickets, rules, webhooks, a Kanban. Never calls a model. |
| bitterblossom | Compute plane. Every model call, per-agent keys + budgets. |
| cerberus | Review organ. Bespoke reviewer subagents; advisory today. |
| crucible | Measurement. Benchmarks defined/run/graded (deterministic + agentic + human). |
| threshold | Optimization. Searches config space under budget targets. Parked. |
| canary | Watchtower. Every app reports errors/health/uptime here. |
| landmark | Release intelligence at merge/deploy. Model-native, BYOK. |
| harness-kit | Skill pile: harness/agent primitives, each shipped with an eval. |

## 2. How the pieces connect

Read [how the loop runs](how-the-loop-runs.md). It walks one full cycle:
card → enhancement → build → review → merge → release → deploy → watch →
triage → back to the pile. Every handoff is a versioned contract; no piece
reads another piece's database.

## 3. The seam map

Read [composition contracts](composition-contracts.md) for the rules and the
seam inventory table. Then read
[seam reference](seam-reference.md) for the per-seam cards: who owns the
schema, what fields are required, what fixtures exist, and what contract test
would break on a field rename.

## 4. Why Cerberus is still advisory

Read [SDLC organ promotion criteria](sdlc-organ-promotion.md). Organs are
promoted on Crucible evidence, not declared. Cerberus's `pass^5` consistency
(0.0434) is well below the blocking floor (0.25), so its verdict stays
advisory until the measurement loop catches up.

## 5. The remote/review question

Read [remote and review primitives](remote-and-review-primitives.md) for the
host and review-primitive research. The recommendation: keep GitHub as v1
source of truth, extract `weave.remote_event.v1` as the host-neutral envelope,
adopt stacked PR discipline, pilot `jj` locally, mirror to Forgejo/Gitea
before any source-of-truth migration.

## 6. Weave-owned contracts fully specified today

Two Weave-owned schemas have complete definitions and fixtures:

- [Remote event schema](schemas/weave.remote_event.v1.schema.json) — the
  host-neutral event envelope. Fixtures in
  [fixtures/contracts/](fixtures/contracts/), with GitHub projection details in
  [remote event projection](remote-event-projection.md).
- [Work item proposal schema](schemas/weave.work_item_proposal.v1.schema.json)
  — the triage→Powder proposal envelope. Fixtures in the same directory.

Every other seam is specified but not yet implemented in its producer repo.

## 7. Contributing

Read [CONTRIBUTING.md](../CONTRIBUTING.md). The short version: one PR per
coherent slice, validate your fixtures, update the seam reference when you
change a contract, never commit secrets or instance data.

## 8. The backlog

Read [backlog.d/](../backlog.d/). Each file is a ticket with a goal, an oracle
(checkable acceptance criteria), and notes. Pick an unblocked ticket and work
it.
