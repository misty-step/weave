# Schema changelog

A durable record of Weave-owned contract schema versions: when each was
introduced, what it covers, and which PR landed it. Consumers pin a producer
release, tag, or commit; this changelog answers "did anything change since I
pinned?" without re-diffing the schema file.

For versioning rules, see [composition contracts](composition-contracts.md).
For per-seam details, see [seam reference](seam-reference.md).

---

## weave.remote_event.v1

**Introduced:** 2026-07-01 · **PR:** [#3](https://github.com/misty-step/weave/pull/3) · **Commit:** `df89d91`

Host-neutral event envelope for remote code/work hosts. GitHub, GitLab,
Forgejo/Gitea, and Gerrit adapters normalize native webhooks into this
contract before BB, Cerberus, Landmark, or Canary consume them.

**Fields:** `schema_version`, `id`, `producer`, `produced_at`, `occurred_at`,
`correlation_id`, `source` (kind, host, external_id, url), `repository` (id,
full_name, default_branch, urls), `subject` (kind, id, number, ref, sha, url),
`actor` (id, login, kind), `action`, `idempotency_key`, `host_payload`
(event_name, delivery_id, links), optional `policy.merge_policy`, `payload`.

**Updated:** 2026-07-04 · `weave-017` tightened the v1 envelope around explicit
source host, repository identity, host-payload links, GitHub event-family
fixtures, and the raw GitHub webhook projection check.

---

## weave.work_item_proposal.v1

**Introduced:** 2026-07-02 · **PR:** [#9](https://github.com/misty-step/weave/pull/9) · **Commit:** `6b1e52b`

Triage-result → Powder proposal envelope. A proposal enters the pile — it
does not bypass Powder's card lifecycle rules. The `status` enum is
`proposed` only; `in_progress` is rejected by schema.

**Fields:** `schema_version`, `id`, `producer`, `produced_at`,
`correlation_id`, `source` (kind, external_id, url), `subject` (repo, kind,
id), `idempotency_key`, `proposed_card` (title, description_ref, priority,
labels), `status`, `payload`.

---

## weave.evidence-pack.v1

**Introduced:** 2026-07-06 · **Commit:** (this PR, weave-922, child of the
weave-920 Evidence Pipeline v2 epic)

Versioned intermediate between fleet-retro's collectors (git, Powder, bb,
feed, campaign receipts) and everything downstream of them: RetroSpec
assembly today, the weave-923 synthesis stage and citation gate next. Every
collector projects its native output into zero or more generic evidence
items rather than RetroSpec assembly reading five differently-shaped
collector structs directly — a new collector or a new report kind extends
the pack, not everything downstream of it. Serializes to
`evidence-pack.json` beside every rendered report
(`apps/fleet-retro/src/pack.rs`).

**Fields:** `schema_version`, `window` (`since`, `until`), `items[]` each
`{id, ts, source, kind, title, refs[], excerpt}`. `id` is a stable hash of
source-specific identifying parts (e.g. repo+commit-hash), not random, so
the same evidence always gets the same id. `refs` is a small ad-hoc tag list
(`"repo:landmark"`, `"card:landmark-907"`, `"pr:200"`) rather than
per-source struct fields — a consumer that only understands the five fixed
fields still gets `title`/`excerpt`/`ts`, while a consumer needing
source-specific structure (RetroSpec assembly's per-repo rollups) parses the
tags it recognizes.

**Dispatch note:** consumers must key on an item's `source` prefix
(`git:`/`powder:`/`bb:`/`feed:`/`receipt:`) before `kind` — feed-post's own
`KNOWN_KINDS` enum reserves `"receipt"` as a valid feed-post kind (receipt
mirrors), which collides with the campaign-receipts collector's own
`"receipt"` kind. Every collector stamps a distinct source prefix
specifically so this is resolvable without ambiguity.

**Regression discipline:** any change to `pack.rs` or `assemble.rs`'s
consumption of it must be checked against a byte-identical rendered-HTML
diff for a fixed past window before merging (weave-922's own acceptance
criterion) — the pack is an internal refactor of an already-shipped
renderer, not a new surface, so its introduction must not change what the
operator sees.
