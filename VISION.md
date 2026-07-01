# The Weave

**An opinionated composition of agent-first development tools — the fabric the fleet runs on.**

In the Forgotten Realms, the Weave is the underlying structure that makes magic usable:
raw power, made safe and castable. This repo is that layer for the Misty Step toolchain.
Each tool below is built first as a tool for ourselves, standalone and composable
(one core → API + CLI + MCP + SDK + skill + thin UI). The Weave stitches them into one
opinionated software-development loop. Ship with our defaults; bring your own pieces later.

## The Loop

```
powder (pile) ──agentic enhancement──▶ ready-for-dev
     ▲                                      │
     │                            bitterblossom dispatch
  triage PR                    (complexity-matched models,
     │                          per-agent keys + budgets)
     │                                      │
canary incident ◀── deploy ◀── merge ◀── cerberus review (advisory)
     ▲                                      ▲
     └── every app reports here             └── crucible-measured quality,
         (errors, health, uptime)               threshold-optimized configs
```

- **powder** — work state. Deliberately dumb: tickets, rules, webhooks, a gorgeous Kanban.
  Never calls a model. Default work app; BYO (e.g. Linear) later.
- **bitterblossom** — the compute plane. Every model call the loop makes runs here:
  enhancement agents, build agents, review dispatch, triage, doc-sync, CI-audit loops.
  Governance lives here: per-agent OpenRouter keys, spend caps, anti-loop belts.
- **cerberus** — review organ. Orchestrator master composing bespoke reviewer subagents;
  advisory artifact on every merge. First of the SDLC organs; siblings (architect,
  builder, CI, QA, docs) are minted only when crucible evidence justifies promotion.
- **crucible** — measurement. Benchmarks defined/run/graded (deterministic + agentic +
  human tiers), every run in a database attached to its benchmark.
- **threshold** — optimization. Consumes crucible benchmarks, searches config space
  under budget targets. Parked until crucible runs for real.
- **canary** — watchtower. Every app in the loop reports errors/health/uptime;
  incidents trigger triage agents on bitterblossom.
- **landmark** — release intelligence at merge/deploy. Model-native, BYOK.
- **harness-kit** — the skill pile: harness/agent primitives, each shipped with an eval,
  synced in whole or part into system and repo harnesses.

## Doctrine

1. **Tools for ourselves first.** Adoption by others is downstream of daily use by us.
2. **Model-native where judgment lives.** The signature failure mode this repo exists to
   prevent: deterministic keyword heuristics where the product premise demands a model
   call. Cerberus reviews for it by name. The inverse failure (a model where
   deterministic code belongs — scoring, policy, persistence) is reviewed equally.
3. **Product/instance law.** Every repo public-able at any moment; instances hold data.
4. **BYOK.** Released tools bring-your-own-keys by default.
5. **Adversarial review everywhere.** Demos are not measurements; agents wirehead when
   nothing hostile reads their work. Cerberus runs on all work pre-merge (advisory).
6. **Execution is cheap, judgment is scarce.** Heavy execution on the plane;
   frontier tokens reserved for planning and review.

## Status

Composition spec + contracts + cross-cutting backlog live here. The pieces are being
hardened standalone first (see backlog.d/); the composition layer follows.
