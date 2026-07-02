# CI/quality-gate auditor agent: continuous gate improvement, on a loop

Priority: P2 · Status: draft · Estimate: M

## Goal
A BB loop agent that audits each repo's CI design, quality gates, tests, linter, build: is it performant, well designed, actually guaranteeing correctness/safety/security? Proposes (PRs) enforcement increases, speedups, cost cuts.

## Oracle
- [ ] Trigger + agent defined in BB with scoped key + budget.
- [ ] First month: >=3 merged gate improvements with before/after evidence (time, cost, or caught-failure class).

## Notes
Never lowers gates (doctrine). Pairs with the fingerprint-gate idea from the groom sweep (fail CI on tailnet names/personal paths).

2026-07-02 overnight lane: spec drafted in `docs/ci-auditor-flow.md`. Trigger,
flow, doctrine (never lower a gate, evidence over vibes, fingerprint-gate
alignment), contracts, model choice, budget, and acceptance oracle are
specified. Remaining: trigger + agent implemented in BB.
