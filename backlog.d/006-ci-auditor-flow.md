# CI/quality-gate auditor agent: continuous gate improvement, on a loop

Priority: P2 · Status: pending · Estimate: M

## Goal
A BB loop agent that audits each repo's CI design, quality gates, tests, linter, build: is it performant, well designed, actually guaranteeing correctness/safety/security? Proposes (PRs) enforcement increases, speedups, cost cuts.

## Oracle
- [ ] Trigger + agent defined in BB with scoped key + budget.
- [ ] First month: >=3 merged gate improvements with before/after evidence (time, cost, or caught-failure class).

## Notes
Never lowers gates (doctrine). Pairs with the fingerprint-gate idea from the groom sweep (fail CI on tailnet names/personal paths).
