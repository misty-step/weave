# Seam reference

Per-seam reference cards expanding the [composition contracts](composition-contracts.md)
seam table. Each card names the schema version, producer, consumer, status,
required fields, and the contract test that would fail on a breaking change.

Notation: **owner repo** is where the schema file, fixtures, and validation
live. Consumers pin a producer release, tag, or commit — floating `main` is not
a contract. All payloads carry `schema_version` and reject unknown major
versions. No schema carries secrets, private instance data, local paths, or
tailnet-only hostnames.

---

## Remote host → BB / Cerberus / Landmark

**Schema:** `weave.remote_event.v1`
**Owner:** weave (`docs/schemas/weave.remote_event.v1.schema.json`)
**Consumers:** BB trigger ingress, Cerberus dispatch planner, Landmark release trigger
**Status:** specified; adapters pending

The host-neutral event envelope. GitHub, GitLab, Forgejo/Gitea, and Gerrit
adapters normalize native webhooks into this contract before any Weave consumer
touches them.

Required fields: `schema_version`, `id`, `producer`, `produced_at`,
`occurred_at`, `correlation_id`, `source` (`kind`, `external_id`), `subject`
(`repo`, `kind`, `id`), `actor` (`id`, `login`), `action`, `idempotency_key`,
`payload`.

Fixtures: `docs/fixtures/contracts/weave.remote_event.v1.github-pr-opened.json`,
`weave.remote_event.v1.forgejo-pr-opened.json`.

**Contract test:** a GitHub PR-open fixture and a Forgejo/Gitea fixture
normalize to the same contract and trigger the same dry-run BB decision.
Consumers reject events with `schema_version` ≠ `weave.remote_event.v1` and
surface the unsupported version.

---

## Powder ready work → BB

**Schema:** `powder.ready_work_event.v1`
**Owner:** powder
**Consumer:** BB workload dispatcher
**Status:** new — not yet implemented

Signals that a card is claimable for development. BB consumes this event; it
does not read Powder's database.

Required fields (contract target): `schema_version`, card id, lifecycle state
(`ready_for_dev`), priority, correlation id, idempotency key, minimal card
context (title, description ref, labels).

**Contract test:** a Powder ready-card fixture becomes a BB run request
without reading Powder DB internals. BB rejects an event missing
`idempotency_key`.

---

## BB claim / status / proof → Powder

**Schemas:** `bb.work_claim.v1`, `bb.run_status.v1`, `bb.proof_link.v1`,
`bb.input_request.v1`
**Owner:** bitterblossom
**Consumer:** powder
**Status:** new — not yet implemented

The work-loop quartet. `work_claim` reserves a card. `run_status` reports
progress through the run timeline. `proof_link` attaches the deliverable (PR,
diff, artifact URL). `input_request` pauses the card when the agent cannot
resolve an ambiguity — no silent stalls.

**Contract test:** Powder accepts claim, run-timeline, awaiting-input, and
proof fixtures from BB and rejects any missing an idempotency key. A status
transition that skips `claimed` → `in_progress` is rejected.

---

## BB → Cerberus review

**Schema:** `cerberus.review_request.v1`
**Owner:** cerberus (defines); BB produces conforming requests
**Consumer:** cerberus
**Status:** existing in Cerberus

BB composes a review request from the PR diff, metadata, and acceptance oracle,
then dispatches Cerberus.

**Contract test:** a BB fixture validates against the Cerberus request schema
before dispatch. Malformed requests (missing diff ref, missing oracle) are
rejected at the BB boundary, not inside Cerberus.

---

## Cerberus → BB / remote host

**Schemas:** `cerberus.review_artifact.v1`, `cerberus.review_receipt_bundle.v1`,
`cerberus.post_plan.v1`, `cerberus.post_result.v1`
**Owner:** cerberus
**Consumers:** BB, remote projection adapter
**Status:** existing in Cerberus

Cerberus returns a review artifact and a receipt bundle. The remote adapter
posts a comment **only** from a valid `cerberus.post_plan.v1` — the post plan
is the authorization boundary. No direct host writes from Cerberus.

**Contract test:** BB gates on artifact verdict and receipt schema before
posting. The remote adapter rejects a post plan with an unknown
`schema_version`. A `post_result` without a matching `post_plan` is rejected.

---

## Cerberus → Crucible

**Schemas:** `cerberus.review_receipt_bundle.v1`,
`cerberus.crucible_producer_manifest.v1`
**Owner:** cerberus
**Consumer:** crucible
**Status:** existing in Cerberus/Crucible

Crucible imports a Cerberus receipt bundle to score review quality over time.
The producer manifest declares which Cerberus versions emitted receipts in a
batch.

**Contract test:** Crucible imports a receipt bundle and rejects unsupported
schema versions with the exact version surfaced in the error. A manifest
referencing a non-existent bundle version fails.

---

## Crucible → Threshold

**Schemas:** `crucible.eval_spec.v1`, `crucible.run_report.v1`,
`crucible.harbor_eval_export.v1`
**Owner:** crucible
**Consumer:** threshold / Daedalus Harbor runner
**Status:** partly existing

Threshold scores a Crucible-authored Harbor export without importing Crucible
internals. `eval_spec` defines the arena; `run_report` is one scored run;
`harbor_eval_export` is the portable bundle Threshold optimizes against.

**Contract test:** Threshold scores a Harbor export fixture and produces a
recommendation without reaching into Crucible's database. An export missing
`eval_spec` reference is rejected.

---

## Threshold → BB

**Schema:** `threshold.agent_config_recommendation.v1`
**Owner:** threshold
**Consumer:** BB agent/workload registry
**Status:** new — parked until Crucible runs

Threshold searches the config space under a budget target and recommends the
best agent-config. BB records why a recommendation was accepted or refused.

**Contract test:** BB dry-runs a recommendation and records the accept/refuse
decision with a reason. A recommendation without a budget cap or CI evidence
ref is rejected.

---

## BB run telemetry → Crucible / Threshold

**Schema:** `bb.run_telemetry.v1`
**Owner:** bitterblossom
**Consumers:** crucible, threshold
**Status:** existing in BB

Run-level telemetry (model, token counts, latency, cost, verdict) joined to
eval/run records without relying on logs.

**Contract test:** Crucible/Threshold fixtures join run telemetry to eval/run
records by `run_id`. Telemetry without `schema_version` is rejected.

---

## Canary incident → BB

**Schema:** `canary.incident_event.v1`
**Owner:** canary
**Consumer:** BB canary triage workload
**Status:** partly existing (Canary webhook contract)

BB treats the webhook as a wake-up hint, then replays the Canary timeline before
triage — timeline before notification.

**Contract test:** BB replays the incident timeline before dispatching triage.
An incident event without a replayable timeline ref is held, not acted on.

---

## BB remediation → Canary

**Schemas:** `bb.remediation_claim.v1`, `bb.remediation_status.v1`
**Owner:** bitterblossom
**Consumer:** canary
**Status:** new

Canary records claim ownership/status and rejects duplicate active claims for
the same incident subject — one owner at a time.

**Contract test:** a second active claim for the same incident subject is
rejected. A `remediation_status` for an unclaimed incident is rejected.

---

## Landmark release intelligence → repos / Weave

**Schemas:** `landmark.run_evidence.v1`, `landmark.release_kit.v1`,
`landmark.synthesis_status.v1`, `landmark.release_entry.v1`
**Owner:** landmark
**Consumers:** consumer repo release workflow, Weave release ledger
**Status:** existing in Landmark schemas

Consumer release workflows validate Landmark evidence before mutating release
notes or feeds. Landmark is model-native and BYOK.

**Contract test:** a release workflow validates Landmark evidence before
writing release notes. Evidence with an unknown `schema_version` is rejected
and the release is not mutated.

---

## Harness Kit skill / eval → BB workloads

**Schemas:** `harness.skill_bundle_manifest.v1`,
`harness.skill_eval_result.v1`
**Owner:** harness-kit
**Consumers:** BB workload definitions, doc-sync, CI-auditor
**Status:** new

BB refuses a skill-bound workload if the skill manifest has no version or
declared eval evidence.

**Contract test:** BB rejects a workload binding a skill manifest without
`schema_version` or without a referenced eval result. A skill bundle with a
missing eval ref is refused at workload registration.

---

## BB doc-sync / CI-audit results → Powder / remote host

**Schema:** `bb.maintenance_result.v1`
**Owner:** bitterblossom
**Consumers:** powder, remote host projection
**Status:** new

A doc-sync or CI-audit run can open a PR, request input, or complete a Powder
card with proof — using one result schema.

**Contract test:** a maintenance result opens a PR, requests input, or completes
a Powder card — all from the same schema. A result missing the `outcome` enum
is rejected.

---

## Canary / remote incidents → Powder work proposals

**Schema:** `weave.work_item_proposal.v1`
**Owner:** weave (`docs/schemas/weave.work_item_proposal.v1.schema.json`)
**Consumer:** powder
**Status:** specified; producer implementation pending

A triage result proposes a Powder card without bypassing Powder's card
lifecycle rules — a proposal does not skip the pile.

Required fields: `schema_version`, `id`, `producer`, `produced_at`,
`correlation_id`, `source` (`kind`, `external_id`), `subject` (`repo`, `kind`,
`id`), `idempotency_key`, `proposed_card` (`title`, `priority`), `status`.

The `status` enum is `proposed` only — a proposal with `status: in_progress`
is rejected. Proposals enter the pile, not the active queue.

Fixtures: `docs/fixtures/contracts/weave.work_item_proposal.v1.canary-incident.json`
(valid), `weave.work_item_proposal.v1.missing-schema-version.json` (invalid),
`weave.work_item_proposal.v1.status-in-progress.json` (invalid).

**Contract test:** Powder accepts a proposal and applies card-lifecycle rules
(opening in backlog). A proposal with `status: in_progress` is rejected —
proposals enter the pile, not the active queue.

---

## Schema registry shape

Each producer repo exposes:

```
schemas/
  <schema_version>.schema.json
fixtures/
  contracts/
    valid/
      <schema_version>-minimal.json
      <schema_version>-full.json
    invalid/
      <schema_version>-missing-schema-version.json
      <schema_version>-unknown-major.json
```

Each consumer keeps a small fixture set pinned from the producer release. A
breaking field rename fails in CI because a consumer fixture no longer
validates or deserializes.
