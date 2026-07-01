# Research: the right remote + review primitive for an agent-majority fleet

Priority: P2 · Status: pending · Estimate: M

## Goal
A defended position on where fleet code coordination lives (GitHub vs Gitea/GitLab/self-hosted) and whether the PR is the right merge primitive when agents write 1000x the code (JJ, stacked diffs, patch queues, agent-native alternatives).

## Oracle
- [ ] A written memo comparing >=3 remotes and >=3 review/merge primitives against fleet requirements (BB triggers, cerberus hooks, outage tolerance, API rate limits, cost).
- [ ] Recommendation with migration cost estimate; adopted or explicitly rejected by operator.

## Notes
Operator, 2026-07-01: "PRs are made up by GitHub, not a git-native primitive... be open to the idea that the old way is not best." GitHub outage frequency is a live pain. Any move must keep cerberus/BB/landmark triggers working.
