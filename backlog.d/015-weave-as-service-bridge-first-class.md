# Weave runs as a service; the Bridge is its first-class surface

Priority: P1 · Status: pending · Estimate: XL (epic)

## Goal
Weave graduates from a composition-contracts repo into a running service, and
the Bridge — the operator's central work surface — becomes a first-class
citizen of it: either a standalone factory application or a first-class Weave
feature. Concretely, that means launching and managing Weave as a service and
having that be where Bridge content is accessed, rather than a supervisor
regenerating static HTML by hand. The Bridge specifically represents **agents
doing work** — it must capture bitterblossom's activity (its primary source
today) but generalize beyond it to any agent-fleet lane, including local herdr
lanes. That agent-activity scope — not board/card state — is why the Bridge
belongs to Weave, not Powder.

## Oracle
- [ ] A documented decision states what "Weave runs as a service" means
      concretely (deployable process, always-on host, or scheduled job) and
      where the Bridge's runtime lives relative to it.
- [ ] The Bridge's current implementation
      (`~/.factory-lanes/scripts/bridge.py`, `bridge-poll.py`, `gazette.py`,
      `seed-bridge-questions.py`, `editorial.json`) is migrated into
      weave-owned code, not left as private factory-ops supervisor scripts.
- [ ] Bridge/Gazette regeneration runs on a cron or event trigger with no
      supervisor (human or agent) required in the loop for routine regen.
- [ ] The Bridge surfaces agent-fleet work broadly — bitterblossom activity
      plus at least one non-bitterblossom source (e.g. local herdr lanes) —
      not hardcoded to a single fleet member.
- [ ] The Bridge exposes the five faces per docs/the-five-faces.md, or carries
      a named, operator-ratified exception, consistent with epic 013.

## Children
1. Service runtime decision — define what "Weave as a service" is
   (deployable process vs. scheduled job vs. long-running host) and where the
   Bridge organ runs relative to it; likely absorbs the current
   bridge.py/bridge-poll.py/gazette.py scripts as the organ implementation.
   Supersedes/absorbs backlog 014 (fleet self-reporting organs) — 014's
   gazette+showcase-as-organs decision folds into this epic's runtime
   decision rather than staying separate.
2. Migrate the factory-ops scripts (bridge.py, bridge-poll.py, gazette.py,
   seed-bridge-questions.py, the editorial.json overlay pattern, STATUS.md
   parsing) from `~/.factory-lanes/scripts` (private factory-ops repo) into
   weave-owned code.
3. Cron-driven regen — wire nightly/periodic Bridge and Gazette regeneration
   to a scheduler (cron, post-merge webhook, or equivalent) with no
   supervisor in the loop.
4. The five faces for the Bridge/weave-as-service organ, coordinated with
   epic 013's coverage matrix.

## Notes
Born from the operator's 2026-07-02 statement: the Bridge is a first-class
citizen of the Weave — either a standalone factory application or a
first-class Weave feature, "which would just mean we should be launching and
managing Weave as a service and have that be where I'm accessing Bridge
content." The Bridge specifically represents agents doing work — capturing
bitterblossom work but not limited to it (also local herdr lanes etc.) —
which is why it's Weave's, not Powder's. Cross-reference: 014 (fleet
self-reporting organs), likely absorbed/superseded by this epic's runtime
decision rather than kept as a separate item.
**Why:** operator directive, verbal, 2026-07-02.
