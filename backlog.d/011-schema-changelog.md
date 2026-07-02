# Write a schema changelog for Weave-owned contracts

Priority: P3 · Status: done · Estimate: S

## Goal
`docs/composition-contracts.md` states the versioning *rules* (rule 5: additive
optional fields stay within a major version; anything else needs a new major)
but there is no durable record of the version *history* — when each
Weave-owned schema was introduced, what changed at each version bump, and
which producers/consumers are pinned to which version. As Weave-owned schemas
accumulate (two today, more as the "First implementation order" list in
`docs/composition-contracts.md` executes), a changelog is the only place a
consumer can answer "did anything change since I pinned this?" without
re-diffing the schema file.

## Oracle
- [ ] `docs/schema-changelog.md` exists with one entry per Weave-owned schema
      version: schema name, version, date introduced, PR/commit reference,
      and a one-line description of what it covers.
- [ ] Both current schemas (`weave.remote_event.v1`,
      `weave.work_item_proposal.v1`) have an entry, backdated to their actual
      introducing commits (`git log --oneline -- docs/schemas/`).
- [ ] `CONTRIBUTING.md` step 2 ("Draft the schema") gains a line: adding or
      bumping a Weave-owned schema requires a changelog entry in the same PR.
- [ ] `README.md`'s docs table links the new changelog.

## Notes
This is a docs-only addition — no schema or fixture logic changes. Keep
entries factual (date, PR, one line); this is not the place for design
rationale, which already lives in `docs/composition-contracts.md` and
`docs/seam-reference.md`.

**Why:** the first time a Weave-owned schema needs a breaking v2, whoever
writes it needs a place to record why v1 broke and what v2 fixes — starting
that ledger now, while there are only two entries, is cheap; reconstructing it
later from git blame is not.
