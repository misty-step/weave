# Add missing unknown-major invalid fixture for weave.work_item_proposal.v1

Priority: P2 · Status: done · Estimate: S

## Goal
`CONTRIBUTING.md` §"Adding or changing a contract" step 3 requires: "Add
invalid fixtures for the cases consumers must reject (missing
`schema_version`, unknown major version)." `weave.remote_event.v1` has both
(`weave.remote_event.v1.missing-schema-version.json` and
`weave.remote_event.v1.unknown-major.json`). `weave.work_item_proposal.v1`
only has `missing-schema-version` and `status-in-progress` — no
`unknown-major` fixture. The repo's own contribution guide isn't followed by
its own second schema.

## Oracle
- [ ] `docs/fixtures/contracts/weave.work_item_proposal.v1.unknown-major.json`
      exists, modeled on the existing
      `weave.remote_event.v1.unknown-major.json` (same required fields as the
      valid `canary-incident` fixture, but `schema_version` set to an
      unsupported value like `weave.work_item_proposal.v2`).
- [ ] `./scripts/verify.sh` runs the new fixture through
      `validate-contracts.cjs` and reports it correctly rejected (the
      `INVALID_MARKERS` array in `scripts/validate-contracts.cjs` already
      matches on the `unknown-major` substring — confirm it picks up the new
      file without a script change; if it doesn't, that's the actual bug to
      fix here).
- [ ] `docs/seam-reference.md`'s work-item-proposal card fixture list is
      updated to mention the new fixture, matching how the remote-event card
      lists all four of its fixtures.

## Notes
Verified live 2026-07-01: `ls docs/fixtures/contracts/` shows 4 remote_event
fixtures (2 valid + 2 invalid) vs. 3 work_item_proposal fixtures (1 valid + 2
invalid, missing the unknown-major case). `docs/schemas/weave.work_item_proposal.v1.schema.json`
uses `"const": "weave.work_item_proposal.v1"` for `schema_version`, so ajv
will already reject a wrong-version fixture at the schema layer — the value
of this ticket is fixture-coverage symmetry and following the repo's own
documented recipe, not discovering a new gap in the schema itself.

**Why:** an inconsistently-covered second schema undermines the "every seam
follows the same contract discipline" claim in VISION.md's doctrine, and it's
the kind of thing a future contributor copies without noticing it's
incomplete.
