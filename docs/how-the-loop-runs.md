# How the loop runs

A walkthrough of the Weave software-development loop, from a card in Powder to a
released change with evidence. This is the onboarding narrative: it names every
piece, the handoff between them, and the contract that crosses each seam.

For the contract map, see [composition contracts](composition-contracts.md).
For the remote/review primitive research, see
[remote and review primitives](remote-and-review-primitives.md).

## The loop at a glance

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

The loop is event-driven, not request/response. Each piece owns its state and
emits a versioned contract at each seam. No piece reads another piece's
database. The sections below walk one full cycle.

## 0. The pile

Work starts in **Powder** — the deliberately dumb work-state app. Powder holds
tickets, rules, webhooks, and a Kanban. It never calls a model. A card enters
the pile (operator-created, imported from an issue adapter, or proposed by a
triage agent) and sits in the backlog until it is ready for development.

Powder's responsibility in the loop is narrow: be the system of record for work
state and lifecycle rules. When a card is ready for dev, Powder emits a
`powder.ready_work_event.v1` — a versioned event that says "this card is
claimable." Bitterblossom consumes that event; it does not read Powder's
database.

## 1. Enhancement → ready-for-dev

When a card is claimed, **Bitterblossom** (the compute plane) dispatches an
enhancement agent. The enhancement agent reads the card context, expands it
into a development-ready spec, and writes the result back through a BB→Powder
proof link (`bb.proof_link.v1`). The card transitions from backlog → in-progress
→ ready-for-dev.

BB owns the model call here. Per-agent OpenRouter keys and spend caps govern
the dispatch. The enhancement agent is a cheap model — frontier tokens are
reserved for review and planning, not for card expansion.

Contract at this seam:

| Direction | Schema | Producer | Consumer |
| --- | --- | --- | --- |
| Powder → BB (ready work) | `powder.ready_work_event.v1` | Powder | BB workload dispatcher |
| BB → Powder (claim/status/proof) | `bb.work_claim.v1`, `bb.run_status.v1`, `bb.proof_link.v1`, `bb.input_request.v1` | Bitterblossom | Powder |

If the agent needs input it cannot resolve (ambiguity, missing credentials,
operator decision), it emits a `bb.input_request.v1` and the card pauses. No
silent stalls.

## 2. Build dispatch

A ready-for-dev card triggers a BB build agent. The build agent is
complexity-matched: a small fix gets a cheap model; a multi-file refactor gets a
frontier model. The dispatch decision is a BB policy, not a model call.

The build agent works on a branch, runs the repo's gate, and opens a pull
request when green. The PR-open event is the first seam that crosses into the
remote/review layer.

## 3. The remote event

When a PR is opened (or updated, or a check completes), the host emits a native
webhook. A Weave host adapter normalizes it into `weave.remote_event.v1` — the
host-neutral event envelope. This is the seam that keeps the fleet from being
locked to GitHub.

```
GitHub webhook ──▶ github-adapter ──▶ weave.remote_event.v1 ──▶ BB / Cerberus / Landmark
Forgejo webhook ──▶ forgejo-adapter ──▶ weave.remote_event.v1 ──▶ same consumers
```

The envelope carries `schema_version`, `source.kind`, `subject`, `actor`,
`action`, `idempotency_key`, and a `payload` with selected native details. It
must not carry secrets, private instance data, local paths, or tailnet-only
hostnames. See [the schema](schemas/weave.remote_event.v1.schema.json) and
[the GitHub PR-opened fixture](fixtures/contracts/weave.remote_event.v1.github-pr-opened.json).

BB, Cerberus, and Landmark consume `weave.remote_event.v1`, not raw host
payloads. This is what lets the fleet migrate hosts without rewriting the
consumers.

## 4. Cerberus review (advisory)

On PR open, BB dispatches **Cerberus** — the review organ. Cerberus composes
bespoke reviewer subagents for the change, produces a review artifact, and
returns it to BB and the remote host.

Cerberus is advisory. Its verdict does not block merge today — the promotion
criteria doc records that Cerberus's `pass^5` consistency (0.0434) is well below
the blocking floor (0.25). The verdict is posted as a comment; the merge
decision is still human (or BB-policy) gated. See
[SDLC organ promotion criteria](sdlc-organ-promotion.md) for the bar Cerberus
must clear before its verdict becomes blocking.

Contract at this seam:

| Direction | Schema | Producer | Consumer |
| --- | --- | --- | --- |
| BB → Cerberus (review request) | `cerberus.review_request.v1` | BB | Cerberus |
| Cerberus → BB / remote host | `cerberus.review_artifact.v1`, `cerberus.post_plan.v1`, `cerberus.post_result.v1` | Cerberus | BB, remote adapter |

The remote adapter posts a comment only from a valid `cerberus.post_plan.v1`.
No direct host writes from Cerberus — the post plan is the authorization
boundary.

## 5. Measurement

**Crucible** is the measurement layer. Benchmarks are defined, run, and graded
(deterministic + agentic + human tiers). Every run lives in a database
attached to its benchmark. Cerberus's review quality is scored here; so is any
agent-config that wants promotion evidence.

**Threshold** consumes Crucible benchmarks and searches the config space under
budget targets. Threshold is parked until Crucible runs for real, but the seam
is already contracted:

| Direction | Schema | Producer | Consumer |
| --- | --- | --- | --- |
| Cerberus → Crucible (receipt bundle) | `cerberus.review_receipt_bundle.v1`, `cerberus.crucible_producer_manifest.v1` | Cerberus | Crucible |
| Crucible → Threshold (eval/run) | `crucible.eval_spec.v1`, `crucible.run_report.v1`, `crucible.harbor_eval_export.v1` | Crucible | Threshold/Daedalus |
| Threshold → BB (config recommendation) | `threshold.agent_config_recommendation.v1` | Threshold | BB agent/workload registry |

The promotion bar (Gate 1) requires a mean reward whose 95% CI excludes zero,
adjudicated through the Crucible human queue — not self-reported. A score that
moves inside its noise floor is not a result.

## 6. Merge → release

When a PR merges, **Landmark** produces release intelligence: a release kit,
synthesis status, and release entry. The consumer repo's release workflow
validates Landmark evidence before mutating release notes or feeds.

| Direction | Schema | Producer | Consumer |
| --- | --- | --- | --- |
| Landmark → repos/Weave | `landmark.run_evidence.v1`, `landmark.release_kit.v1`, `landmark.release_entry.v1`, `landmark.synthesis_status.v1` | Landmark | Consumer repo release workflow, Weave release ledger |

Landmark is model-native and BYOK. It consumes the merge event (via
`weave.remote_event.v1`) and produces structured release intelligence, not free
prose.

## 7. Deploy → Canary

Every app in the loop reports errors, health, and uptime to **Canary** — the
watchtower. A deploy that breaks something triggers a Canary incident event.
Canary does not auto-page; it emits `canary.incident_event.v1` and BB treats it
as a wake-up hint, then replays the Canary timeline before triage.

If BB takes remediation ownership, it emits `bb.remediation_claim.v1`. Canary
rejects duplicate active claims for the same incident subject — one owner at a
time.

| Direction | Schema | Producer | Consumer |
| --- | --- | --- | --- |
| Canary → BB (incident) | `canary.incident_event.v1` | Canary | BB canary triage workload |
| BB → Canary (remediation) | `bb.remediation_claim.v1`, `bb.remediation_status.v1` | Bitterblossom | Canary |

Canary's rule: timeline before notification. BB replays the incident timeline
before deciding triage; it does not act on the webhook alone.

## 8. Triage → back to the pile

If a Canary incident or a triage result proposes new work, it emits a
`weave.work_item_proposal.v1` (see [the schema](schemas/weave.work_item_proposal.v1.schema.json))
to Powder. Powder applies its own card lifecycle rules — a proposal does not
bypass the pile. The loop closes: incident → triage → card → enhancement →
build → review → merge → deploy → (watch).

## Maintenance loops

Two BB-triggered maintenance agents run on the loop's periphery, not in the
main path:

- **Doc-sync agent** — runs the harness-kit document skill with an affordable
  model on PR merge or daily, per managed repo. Finds doc drift, opens PRs.
  First instance of the harness-kit-skill → focused-bespoke-agent pattern. See
  [doc-sync flow](doc-sync-flow.md).
- **CI/quality-gate auditor** — audits each repo's CI design, gates, tests,
  linter, build. Proposes enforcement increases, speedups, cost cuts via PRs.
  Never lowers gates (doctrine). See
  [CI-auditor flow](ci-auditor-flow.md).

Both emit `bb.maintenance_result.v1` — a result can open a PR, request input,
or complete a Powder card with proof, using one schema.

## Invariants

- Every cross-piece payload carries `schema_version`.
- No schema carries secrets, private instance data, local paths, or
  tailnet-only hostnames.
- Consumers reject unknown major versions and surface the exact unsupported
  `schema_version`.
- A breaking field rename fails a CI contract test on the consumer side, not at
  runtime.
- Organs are promoted, not declared: a workload becomes an organ only after it
  clears the four gates on live Crucible evidence.
- Cerberus's verdict stays advisory until its `pass^k` clears the floor.

## Reading order for a new contributor

1. [VISION.md](../VISION.md) — what the Weave is and why.
2. This doc — how the pieces connect.
3. [Composition contracts](composition-contracts.md) — the seam map and rules.
4. [SDLC organ promotion criteria](sdlc-organ-promotion.md) — how organs are
   minted (and why Cerberus is still advisory).
5. [Remote and review primitives](remote-and-review-primitives.md) — the
   host/review research and recommendation.
6. [The remote event schema](schemas/weave.remote_event.v1.schema.json) and
   [the work item proposal schema](schemas/weave.work_item_proposal.v1.schema.json)
   — the two Weave-owned contracts fully specified today. Fixtures in
   [fixtures/contracts/](fixtures/contracts/).
