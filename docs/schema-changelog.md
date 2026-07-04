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
