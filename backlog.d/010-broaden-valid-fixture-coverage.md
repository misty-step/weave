# Broaden valid-fixture coverage for both weave-owned schemas

Priority: P3 · Status: done · Estimate: S

## Goal
Every valid fixture today exercises a "typical happy path" shape, not the
schema's actual optionality contract. `weave.remote_event.v1` has two valid
fixtures (`github-pr-opened`, `forgejo-pr-opened`) that both populate every
optional field (`source.url`, `subject.url`, `actor.kind`). Nothing proves a
fixture with only the `required` fields (no `source.url`, no `actor.kind`,
empty `payload: {}`) still validates. Same gap for
`weave.work_item_proposal.v1`'s single valid fixture — nothing proves a
minimal proposal (no `description_ref`, no `labels`, no `payload`) passes.
Contract rule 5 in `docs/composition-contracts.md` says "additive optional
fields are allowed within a major version" — that claim is currently
unverified by any fixture.

## Oracle
- [ ] A new valid fixture per schema contains only the `required` fields
      listed in that schema's JSON Schema `required` array (no optional
      properties present) and is named to signal that
      (`weave.remote_event.v1.minimal.json`,
      `weave.work_item_proposal.v1.minimal.json`).
- [ ] The existing full fixtures are confirmed (or relabeled) as the "full"
      counterexample — every optional property populated.
- [ ] `./scripts/verify.sh` passes with the new fixtures included, proving
      both the minimal and full shapes validate against the same schema.
- [ ] `docs/seam-reference.md`'s fixture lists for both cards are updated to
      mention the new fixtures.

## Notes
This directly tests the "Schema registry shape" doc's own `-minimal.json` /
`-full.json` naming convention (see ticket 009) — implementing the fixtures
this ticket asks for is a natural companion to that doc fix, but don't block
on it: `validate-contracts.cjs` matches fixtures by `schema_version + '.'`
prefix regardless of the `-minimal`/`-full` suffix, so this ticket can ship
fixtures under the current flat naming without waiting for 009 to land.

**Why:** a schema's declared optionality is a compatibility promise to every
future producer/consumer; an untested promise is just a comment.
