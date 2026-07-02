# Reconcile "Schema registry shape" docs with the actual fixture layout

Priority: P3 · Status: done · Estimate: S

## Goal
Both `docs/composition-contracts.md` and `docs/seam-reference.md` end with a
near-identical "Schema registry shape" section describing a target layout of
`fixtures/contracts/valid/<version>-minimal.json` and
`fixtures/contracts/invalid/<version>-missing-schema-version.json` (nested
`valid/`/`invalid/` subdirectories). The actual, working, CI-gated layout is
flat: `docs/fixtures/contracts/<schema_version>.<case>.json`, with validity
determined by `scripts/validate-contracts.cjs`'s `INVALID_MARKERS` substring
match, not by directory. The docs describe a structure the repo doesn't use
and never has.

## Oracle
- [ ] Both "Schema registry shape" sections are rewritten to describe the
      actual flat, marker-token-suffixed layout that
      `scripts/validate-contracts.cjs` implements (verified against the live
      `INVALID_MARKERS` array and `docs/fixtures/contracts/` listing).
- [ ] The two sections are de-duplicated: one canonical description lives in
      `docs/composition-contracts.md` (the contract-rules doc), and
      `docs/seam-reference.md` links to it instead of repeating a
      near-identical paragraph verbatim.
- [ ] `CONTRIBUTING.md` step 3 ("Add fixtures") is checked against the same
      reconciled description and updated if it also implies subdirectories.
- [ ] `./scripts/verify.sh` still passes (docs-only change).

## Notes
Verified live 2026-07-01 via
`diff <(sed -n '/Schema registry shape/,$p' docs/composition-contracts.md) <(sed -n '/Schema registry shape/,$p' docs/seam-reference.md)`
— the two sections differ only in wording, not in the (both wrong) described
shape. Do not change the actual fixture layout or `validate-contracts.cjs` —
that's a working, tested implementation; this ticket fixes the docs to match
reality, not the reverse. Changing the real layout would be a design decision
(new naming scheme, script rewrite) that belongs to a taste/architecture call,
not an overnight docs fix.

**Why:** a doc that describes an aspirational layout nobody implements is
worse than no doc — the next contributor who follows it literally will create
directories the validation script never looks at, and their fixtures will
silently not run.
