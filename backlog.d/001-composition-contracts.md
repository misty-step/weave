# Define the versioned contracts between the pieces

Priority: P1 · Status: draft · Estimate: L

## Goal
Every seam in the loop (powder→BB events, BB→cerberus dispatch, cerberus→crucible packets, canary→BB triage, crucible→threshold specs) is a versioned, schema-checked contract with a consumer test on each side — no coincidental JSON.

## Oracle
- [x] Draft every seam with a required `schema_version`, producer owner, consumer pin, and contract-test expectation in `docs/composition-contracts.md`.
- [ ] Each producer repo owns the implemented schema and fixtures.
- [ ] A breaking field rename on any side fails a CI contract test, not a runtime.

## Notes
Groom sweep evidence: only canary→BB carries schema_version today; "126" phantom-ref incident shows tracking drift. BB must resume releases so consumers can pin.

2026-07-01 factory lane: drafted the Weave seam map in `docs/composition-contracts.md`. Implementation remains producer-owned in the piece repos.

2026-07-02 overnight lane: added `docs/seam-reference.md` (per-seam cards),
`docs/how-the-loop-runs.md`, `CONTRIBUTING.md`, `docs/onboarding.md`, and the
`./scripts/verify.sh` gate. Implemented the second Weave-owned schema
(`weave.work_item_proposal.v1`) with valid + invalid fixtures. Both Weave-owned
schemas now pass the gate. Remaining: each producer repo owns its implemented
schema and fixtures, and a breaking rename fails a CI contract test in the
consumer repo.
