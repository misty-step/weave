# Research: the right remote + review primitive for an agent-majority fleet

Priority: P2 · Status: pending operator decision · Estimate: M

## Goal
A defended position on where fleet code coordination lives (GitHub vs Gitea/GitLab/self-hosted) and whether the PR is the right merge primitive when agents write 1000x the code (JJ, stacked diffs, patch queues, agent-native alternatives).

## Oracle
- [x] A written memo comparing >=3 remotes and >=3 review/merge primitives against fleet requirements (BB triggers, cerberus hooks, outage tolerance, API rate limits, cost).
- [x] Recommendation with migration cost estimate exists in `docs/remote-and-review-primitives.md`.
- [ ] Recommendation adopted or explicitly rejected by operator.

## Notes
Operator, 2026-07-01: "PRs are made up by GitHub, not a git-native primitive... be open to the idea that the old way is not best." GitHub outage frequency is a live pain. Any move must keep cerberus/BB/landmark triggers working.

2026-07-01 factory lane recommendation: keep GitHub as v1 source of truth, extract `weave.remote_event.v1`, adopt stacked PR discipline, pilot `jj` locally, and mirror to Forgejo/Gitea before any source-of-truth migration.
