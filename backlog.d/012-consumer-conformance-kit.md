# Ship a consumer-conformance kit for Weave-owned schemas

Priority: P2 · Status: done · Estimate: M

## Goal
`docs/composition-contracts.md` rule 4 says consumers must pin a producer
release/tag/commit, and the seam table names a consumer-side contract test
for every seam (e.g. "BB rejects an event missing `idempotency_key`" for
`weave.remote_event.v1`). Today those consumer-side tests only exist as
prose — no piece repo (BB, Cerberus, Landmark, Powder) has a copy-pasteable
starting point for actually writing that test. Weave owns the schema; it
should also own a minimal, portable "here is how you'd validate this in your
own repo's language/test suite" kit, so a consumer repo's first contract test
isn't written from scratch.

## Oracle
- [ ] A new `docs/consumer-conformance-kit.md` (or `scripts/consumer-kit/`
      directory, whichever fits better once drafted) ships one small,
      runnable example per Weave-owned schema showing: (a) load the schema
      from a pinned Weave ref, (b) validate a sample inbound payload, (c)
      assert rejection on an unknown major version with the exact version
      surfaced in the error — matching the seam table's named contract test.
- [ ] The example is language-agnostic in spirit but at least one concrete,
      runnable form exists (reuse the existing `ajv`-based approach from
      `scripts/validate-contracts.cjs` since that's already proven in this
      repo — do not introduce a second validation library).
- [ ] `docs/seam-reference.md`'s cards for `weave.remote_event.v1` and
      `weave.work_item_proposal.v1` link to the new kit instead of only
      stating the contract test in prose.
- [ ] `./scripts/verify.sh` (or a new, separate check) proves the kit's
      example actually runs and produces the stated pass/fail outcomes
      against the existing fixtures — a documented example that silently
      bit-rots is worse than none.

## Notes
Scope this to the two schemas Weave actually owns
(`weave.remote_event.v1`, `weave.work_item_proposal.v1`). Do not attempt to
write conformance kits for piece-owned schemas (`bb.*`, `cerberus.*`, etc.) —
per `CONTRIBUTING.md`, those belong in their own repos; a starter kit here
would immediately drift from schemas Weave doesn't control.

**Why:** consumer repos are supposed to reject unknown schema versions and
missing idempotency keys per the seam table, but "supposed to" isn't
verified anywhere yet — a runnable kit is what actually gets a consumer repo
from "read the seam card" to "have a real contract test" in one sitting.
