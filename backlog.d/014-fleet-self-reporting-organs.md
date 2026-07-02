# Productize the gazette and showcase as fleet self-reporting organs

Priority: P2 · Status: pending · Estimate: L (epic)

## Goal
The Weave Gazette (nightly what-happened broadsheet) and the Showcase (evidence-backed
proof-it-works page) graduate from supervisor scripts + hand-assembled artifacts into
durable Weave organs — in this repo or as a distinct service — subject to the
five-faces law like everything else.

## Oracle
- [ ] Gazette generates nightly without a supervisor in the loop (trigger: cron or
      post-merge events), with the deterministic press + model-editorial split
      preserved (press never calls a model; editorial layer does).
- [ ] Showcase evidence packs regenerate on demand per repo (one command), with the
      evidence-gate contract (cold runs, honest trust-breaks, read-only live surfaces)
      encoded in the harness, not in supervisor prose.
- [ ] Both surfaces served durably (artifact server or successor), linked from the
      fleet's home surface.
- [ ] An operator decision records WHERE they live: weave-owned organ vs distinct
      service (the "reporter" microservice option).

## Notes
Origin: 2026-07-01 overnight — gazette built as supervisor 20% time
(~/.factory-lanes/scripts/gazette.py + editorial.json), showcase assembled 07-02 from
nine fresh-context verifier lanes (a/showcase/). Operator 2026-07-02: "this gazette,
this showcase, etc in their ultimate forms is going to be a key part of the weave,
whether directly in that repo or as a distinct service."
Prior art to fold in: the evidence-gate + demo-path contracts from the /showcase skill;
the gazette's editorial.json overlay pattern (deterministic backbone, model voice).
**Why:** operator directive 2026-07-02; both organs proved their value live this week.
