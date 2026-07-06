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

**Introduced:** 2026-07-06 · **Commit:** (weave-922, child of the weave-920
Evidence Pipeline v2 epic)

**Extended:** 2026-07-06 · weave-923 added a sixth collector (moment-scorer
anomaly cards from Bitterblossom's flight-recorder, `source` prefix
`moment:`, `kind` is the scorer's own class name --
`failure`/`recovery`/`cost_anomaly`/`surprise`) as an additive item source,
and made the pack the input the synthesis stage and citation gate read
directly. The item shape (`{id, ts, source, kind, title, refs[], excerpt}`)
did not change, so the schema version stayed at v1 -- the same kind of
growth receipts (weave-921) already went through, 4 sources to 5, without a
version bump.

Versioned intermediate between fleet-retro's collectors (git, Powder, bb,
feed, campaign receipts, moment-scorer cards) and everything downstream of
them: RetroSpec assembly, and (weave-923) the model synthesis stage +
deterministic citation gate. Every collector projects its native output into
zero or more generic evidence items rather than RetroSpec assembly reading
differently-shaped collector structs directly — a new collector or a new
report kind extends the pack, not everything downstream of it. Serializes to
`evidence-pack.json` beside every rendered report
(`apps/fleet-retro/src/pack.rs`).

**Fields:** `schema_version`, `window` (`since`, `until`), `items[]` each
`{id, ts, source, kind, title, refs[], excerpt}`. `id` is a stable hash of
source-specific identifying parts (e.g. repo+commit-hash), not random, so
the same evidence always gets the same id. `refs` is a small ad-hoc tag list
(`"repo:landmark"`, `"card:landmark-907"`, `"pr:200"`) rather than
per-source struct fields — a consumer that only understands the six fixed
fields still gets `title`/`excerpt`/`ts`, while a consumer needing
source-specific structure (RetroSpec assembly's per-repo rollups) parses the
tags it recognizes.

**Dispatch note:** consumers must key on an item's `source` prefix
(`git:`/`powder:`/`bb:`/`feed:`/`receipt:`/`moment:`) before `kind` —
feed-post's own `KNOWN_KINDS` enum reserves `"receipt"` as a valid feed-post
kind (receipt mirrors), which collides with the campaign-receipts
collector's own `"receipt"` kind. Every collector stamps a distinct source
prefix specifically so this is resolvable without ambiguity.

**Regression discipline:** any change to `pack.rs` or `assemble.rs`'s
consumption of it must be checked against a byte-identical rendered-HTML
diff for a fixed past window before merging (weave-922's own acceptance
criterion) — the pack is an internal refactor of an already-shipped
renderer, not a new surface, so its introduction must not change what the
operator sees.

---

## weave-fleet-retro-002 (page-spec catalog version)

**Introduced:** 2026-07-06 · weave-923, child of the weave-920 Evidence
Pipeline v2 epic. Bumped from `weave-fleet-retro-001` (see `CATALOG_VERSION`
in `apps/fleet-retro/src/spec.rs`) because the `Component` catalog gained
two new variants: `Narrative` (the model-synthesized "what mattered"
section, leading the report; tables demoted to appendix below it) and
`Footer` (synthesis diagnosability metadata, immediately before
`Provenance`). Existing components (`Hero`, `StatCallouts`,
`RepoActivityTable`, `Timeline`, `Receipts`, `Provenance`) are unchanged.

**Synthesis stage (`apps/fleet-retro/src/synthesis.rs`):** an `EvidencePack`
goes in, a significance-ranked narrative comes out, every claim carrying an
inline `[id]` citation to a pack item. Cheap-default/escalate-on-failure
model routing: two attempts on a cheap OpenRouter model
(`deepseek/deepseek-v4-flash`), one escalated attempt
(`moonshotai/kimi-k2.7-code`) on repeated gate failure, then fail-open to a
deterministic tables-only report with a visible banner — never more than
three attempts, never a half-cited narrative. The judge model, gate outcome,
prompt version, pack schema version, and pack-assembly latency all ride the
report footer (oracle findings ruled binding 2026-07-06, SRE-postmortem
diagnosability convention).

**Citation gate (`apps/fleet-retro/src/citation_gate.rs`):** deterministic,
external, and forever tier-1-only per the same oracle ruling — an existence
check (every `[id]` token must name a real pack item) plus a structural
uncited-claim check (every non-heading narrative line must carry at least
one citation token). No claim-level entailment, no model judgment inside the
gate itself; a heavier entailment tier is an explicitly deferred later
child, not folded into this one.
