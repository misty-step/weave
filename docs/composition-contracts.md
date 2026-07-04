# Weave composition contracts

Status: draft
Date: 2026-07-01
Backlog: `backlog.d/001-composition-contracts.md`

This document names the seams between the current Weave pieces. It is not an
implementation schema registry yet. It is the contract map that tells each
producer repo which schema it must own and which consumers must pin and test.

Per-seam reference cards (owner repo, required fields, fixture expectations,
contract-test oracle) live in [seam reference](seam-reference.md).

## Contract rules

1. Every inter-piece payload MUST carry `schema_version`.
2. New Weave-owned cross-piece contracts use string versions:
   `<owner>.<noun>.v<major>`, for example `weave.remote_event.v1`.
3. A producer repo owns the schema file, fixtures, validation, and compatibility
   policy for the payload it emits.
4. A consumer pins the producer release, tag, commit, or schema package it
   accepts. Floating `main` is not a contract.
5. Additive optional fields are allowed within a major version. Removing,
   renaming, changing type, changing enum meaning, or changing idempotency
   semantics requires a new major version.
6. Consumers MUST reject unknown major versions and surface the exact
   unsupported `schema_version` in the error.
7. Schemas MUST NOT carry secrets, private instance data, local filesystem
   paths, or tailnet-only hostnames.
8. A live adapter may retain native host payloads for diagnostics, but the
   durable cross-piece payload is the versioned contract.

## Common envelope

For new cross-piece event contracts, use this envelope unless a producer already
has a stricter public schema:

```json
{
  "schema_version": "weave.remote_event.v1",
  "id": "evt_...",
  "producer": {
    "name": "github-adapter",
    "version": "0.1.0"
  },
  "produced_at": "2026-07-01T00:00:00Z",
  "correlation_id": "repo:subject:attempt",
  "source": {
    "kind": "github|gitlab|forgejo|gitea|gerrit|powder|canary|bb|cerberus|crucible|threshold|landmark|harness-kit",
    "external_id": "host-specific-id",
    "url": "https://example.invalid/item"
  },
  "subject": {
    "repo": "owner/name",
    "kind": "pull_request|change|card|incident|run|release|benchmark|agent_config",
    "id": "subject-id"
  },
  "idempotency_key": "stable-dedupe-key",
  "payload": {}
}
```

Producer-specific schemas may inline these fields instead of nesting them, but
the same data must be present unless explicitly waived in that schema.

## Seam inventory

| Seam | Producer owns | Consumer pins | Required `schema_version` | Status | Consumer test |
| --- | --- | --- | --- | --- | --- |
| Remote host -> BB/Cerberus/Landmark | Weave host adapter, then each remote adapter | BB trigger ingress, Cerberus dispatch planner, Landmark release trigger | `weave.remote_event.v1` | New | A GitHub PR-open fixture and a Forgejo/Gitea fixture normalize to the same contract and trigger the same dry-run BB decision. |
| Powder ready work -> BB | Powder | BB workload dispatcher | `powder.ready_work_event.v1` | New | A Powder ready-card fixture becomes a BB run request without reading Powder DB internals. |
| BB claim/status/proof -> Powder | Bitterblossom | Powder | `bb.work_claim.v1`, `bb.run_status.v1`, `bb.proof_link.v1`, `bb.input_request.v1` | New | Powder accepts claim, run timeline, awaiting-input, and proof fixtures from BB and rejects missing idempotency keys. |
| BB -> Cerberus review | Cerberus defines; BB produces conforming requests | Cerberus | `cerberus.review_request.v1` | Existing in Cerberus | BB fixture validates against Cerberus request schema before dispatch. |
| Cerberus -> BB/remote host | Cerberus | BB, remote projection adapter | `cerberus.review_artifact.v1`, `cerberus.review_receipt_bundle.v1`, `cerberus.post_plan.v1`, `cerberus.post_result.v1` | Existing in Cerberus | BB gates on artifact verdict and receipt schema, then remote adapter posts only from a valid post plan. |
| Cerberus -> Crucible | Cerberus | Crucible | `cerberus.review_artifact.v1`, `cerberus.review_receipt_bundle.v1`, `cerberus.crucible_producer_manifest.v1` | Existing in Cerberus/Crucible | Crucible imports a Cerberus receipt bundle and rejects unsupported schema versions. |
| Crucible benchmark/run -> Threshold | Crucible | Threshold Harbor runner | `crucible.eval_spec.v1`, `crucible.run_report.v1`, `crucible.harbor_eval_export.v1` | Partly existing | Threshold scores a Crucible-authored Harbor export without importing Crucible internals. |
| Threshold recommendation -> BB | Threshold | BB agent/workload registry | `threshold.agent_config_recommendation.v1` | New | BB can dry-run an agent config recommendation and record why it was accepted or refused. |
| BB run telemetry -> Crucible/Threshold | Bitterblossom | Crucible, Threshold | `bb.run_telemetry.v1` | Existing in BB | Crucible/Threshold fixtures join run telemetry to eval/run records without relying on logs. |
| Canary incident -> BB | Canary | BB canary triage workload | `canary.incident_event.v1` | Partly existing as Canary webhook contract | BB treats the webhook as a wake-up hint, then replays Canary timeline before triage. |
| BB remediation -> Canary | Bitterblossom | Canary | `bb.remediation_claim.v1`, `bb.remediation_status.v1` | New | Canary records claim ownership/status and rejects duplicate active claims for the same incident subject. |
| Landmark release intelligence -> repos/Weave | Landmark | Consumer repo release workflow, Weave release ledger | `landmark.run_evidence.v1`, `landmark.release_kit.v1`, `landmark.synthesis_status.v1`, `landmark.release_entry.v1` | Existing in Landmark schemas | Consumer release workflow validates Landmark evidence before mutating release notes or feeds. |
| Harness Kit skill/eval -> BB workloads | Harness Kit | BB workload definitions, doc-sync, CI-auditor | `harness.skill_bundle_manifest.v1`, `harness.skill_eval_result.v1` | New | BB refuses a skill-bound workload if the skill manifest has no version or declared eval evidence. |
| BB doc-sync / CI-audit results -> Powder/remote host | Bitterblossom | Powder, remote host projection | `bb.maintenance_result.v1` | New | A doc-sync or CI-audit run can either open a PR, request input, or complete a Powder card with proof using one result schema. |
| Canary/remote incidents -> Powder work proposals | Canary or BB triage workload | Powder | `weave.work_item_proposal.v1` | Specified | A triage result can propose a Powder card without bypassing Powder's card lifecycle rules. |

## Existing schema anchors

The following live repos already have pieces of the versioned contract posture:

- Weave owns `weave.remote_event.v1` (`docs/schemas/weave.remote_event.v1.schema.json`)
  `weave.work_item_proposal.v1` (`docs/schemas/weave.work_item_proposal.v1.schema.json`),
  and `weave.release_feed_row.v1`
  (`docs/schemas/weave.release_feed_row.v1.schema.json`), each with valid and
  invalid fixtures under `docs/fixtures/contracts/`, exercised by
  `./scripts/verify.sh`.
- Cerberus owns `cerberus.review_request.v1`, `cerberus.review_artifact.v1`,
  receipt bundle, producer manifest, and post plan/result shapes.
- Crucible owns `crucible.eval_spec.v1`, labels, calibration records, run
  reports, spec-run evidence, and Harbor export behavior.
- Bitterblossom owns `bb.run_telemetry.v1` and has `bb.command_result.v1`
  validation in the harness boundary.
- Canary exposes an OpenAPI contract and stable webhook delivery guidance;
  its MCP manifest and dogfood registry carry integer `schema_version` fields.
- Landmark has a checked schema registry for manifest, run evidence,
  release-kit, synthesis status, release entries, replay results, fleet plans,
  and failure envelopes.
- Weave pins fixture-validation snapshots for producer-owned
  `powder.card_event.v1` and `landmark.release-kit.v1` so
  `scripts/thread-replay.cjs` can fail Weave CI if the first incident ->
  Powder -> release-feed thread breaks across a schema seam.

## First implementation order

1. `weave.remote_event.v1`: this unlocks the remote/review memo's recommendation
   and prevents GitHub payloads from becoming the cross-piece contract.
2. Powder <-> BB: ready work, claims, run status, proof, and input requests are
   the composition's work loop.
3. BB <-> Cerberus: pin the existing Cerberus request/artifact/receipt schemas
   and add BB-side fixture validation.
4. Canary <-> BB: incident wake-up plus remediation claim/status, preserving
   Canary's timeline-before-notification rule.
5. Crucible <-> Threshold: freeze the Harbor export and recommendation loop
   before Threshold starts optimizing live agent configs.
6. Landmark release schemas: adopt Landmark evidence/kit/status as the release
   contract for every piece that publishes release intelligence.
7. Harness Kit -> BB: skill bundle and skill eval manifests for doc-sync and
   CI-auditor workloads.

## Schema registry shape

Each producer repo exposes schema files and contract fixtures in a flat
layout:

```text
docs/
  schemas/
    <schema_version>.schema.json
  fixtures/
    contracts/
      <schema_version>.<case>.json
```

Fixture validity is determined by a marker token in the filename, not by
directory. `scripts/validate-contracts.cjs` matches the `schema_version`
prefix (everything up to the first `.`) to find the schema, then treats a
fixture as invalid (must be rejected by the schema) if its filename contains
any of these markers:

- `missing-schema-version` — omits the `schema_version` field.
- `unknown-major` — sets `schema_version` to an unsupported major (e.g. `v2`).
- `status-in-progress` — carries a `status` value the schema rejects.

All other fixtures matching the prefix are valid (must pass). Add a new
invalid case by adding the marker to `INVALID_MARKERS` and creating the
fixture file — no directory restructuring needed.

Each consumer keeps a small fixture set pinned from the producer release. A
breaking field rename fails in CI because a consumer fixture no longer
validates or deserializes.

## Acceptance oracle for backlog 001

This draft satisfies the first contract-design slice when:

- every seam in the current Weave vision has a named producer, consumer, and
  `schema_version`;
- existing schema anchors are separated from new required schemas;
- implementation order is explicit;
- no schema is allowed to carry secrets, private instance data, local paths, or
  tailnet-only details;
- every seam names the consumer-side contract test that would fail on a breaking
  field rename.

The Weave-owned schema (`weave.remote_event.v1`) is exercised by
`./scripts/verify.sh`, which validates valid fixtures (must pass) and invalid
fixtures (missing `schema_version`, unknown major — must be rejected) against
the schema. Piece-owned schemas are validated in their producer repos.
