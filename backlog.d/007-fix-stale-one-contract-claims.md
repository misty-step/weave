# Fix stale "one contract fully specified today" claims

Priority: P2 · Status: ready · Estimate: S

## Goal
`docs/onboarding.md` (§6 heading + body), `docs/how-the-loop-runs.md` (closing
reading-order list, item 6), and `README.md` (docs table, remote event schema
row) all still say `weave.remote_event.v1` is "the one contract fully
specified today." That was true when those docs were written (PRs #4-#6), but
PR #9 shipped a second Weave-owned schema, `weave.work_item_proposal.v1`, with
its own fixtures and `./scripts/verify.sh` coverage. The claim is now false —
exactly the doc-drift class the doc-sync-flow ticket (005) exists to catch,
except tonight nobody ran it on Weave's own docs.

## Oracle
- [ ] `grep -rn "one contract fully specified" docs/ README.md` returns zero
      matches (or the phrase is rewritten to name both schemas and is
      accurate as of the PR that closes this ticket).
- [ ] `docs/onboarding.md` §6 lists both `weave.remote_event.v1` and
      `weave.work_item_proposal.v1` with links to their schema files and
      fixture directories.
- [ ] `docs/how-the-loop-runs.md`'s reading-order list and §8 ("Triage → back
      to the pile") link the `weave.work_item_proposal.v1` schema file the
      same way §3 links the remote-event schema.
- [ ] `README.md`'s docs table gains a row (or amends the existing row) so a
      new reader can find both Weave-owned schemas from the README.
- [ ] `./scripts/verify.sh` still passes after the edit (docs-only change,
      but confirms nothing else broke).

## Notes
Verified live 2026-07-01: `grep -rn "fully specified" docs/ README.md` hits
exactly `docs/onboarding.md:56`, `docs/how-the-loop-runs.md:223`, and
`README.md:22`. `docs/composition-contracts.md` was correctly updated in PR #9
(it already lists both schemas under "Existing schema anchors") — only the
three onboarding-path docs missed the update. Docs-only change; no schema or
gate logic touched.

**Why:** New contributors following the onboarding reading order get told a
capability doesn't exist when it does, and the doc-sync agent (backlog 005)
is supposed to prevent exactly this drift — fixing it manually here also
gives that future agent a clean baseline to diff against.
