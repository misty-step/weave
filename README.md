# the weave

Opinionated composition of agent-first development tools. See [VISION.md](VISION.md).

Pieces: [bitterblossom](https://github.com/misty-step/bitterblossom) ·
[powder](https://github.com/misty-step/powder) · [cerberus](https://github.com/misty-step/cerberus) ·
[crucible](https://github.com/misty-step/crucible) · [canary](https://github.com/misty-step/canary) ·
[landmark](https://github.com/misty-step/landmark) · [threshold](https://github.com/misty-step/threshold) ·
[harness-kit](https://github.com/misty-step/harness-kit)

Status: pre-composition. The pieces are being hardened standalone; the loop is the spec.

## Docs

- [How the loop runs](docs/how-the-loop-runs.md) — onboarding narrative: card → build → review → merge → deploy → watch.
- [Composition contracts](docs/composition-contracts.md) — the seam map and versioned-schema rules.
- [Seam reference](docs/seam-reference.md) — per-seam cards: owner, fields, fixture expectations, contract-test oracle.
- [Remote and review primitives](docs/remote-and-review-primitives.md) — host/review research and recommendation.
- [SDLC organ promotion criteria](docs/sdlc-organ-promotion.md) — how organs are minted (and why Cerberus is still advisory).
- [Doc-sync flow](docs/doc-sync-flow.md) — the doc-drift maintenance loop (backlog 005).
- [CI-auditor flow](docs/ci-auditor-flow.md) — the gate-improvement maintenance loop (backlog 006).
- [Remote event schema](docs/schemas/weave.remote_event.v1.schema.json) — host-neutral event envelope.
- [Work item proposal schema](docs/schemas/weave.work_item_proposal.v1.schema.json) — triage→Powder proposal envelope.
- [Schema changelog](docs/schema-changelog.md) — version history for Weave-owned contracts.
- [Consumer conformance kit](docs/consumer-conformance-kit.md) — runnable starting point for consumer-side contract tests.

New here? Start with [onboarding](docs/onboarding.md). Contributing? Read [CONTRIBUTING.md](CONTRIBUTING.md).
