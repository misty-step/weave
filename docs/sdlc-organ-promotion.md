# SDLC organ promotion criteria

Status: draft
Date: 2026-07-01
Work item: Powder card `weave-004`

This document defines the measurable bar an agent-config-with-responsibility
must clear before it graduates from a Bitterblossom workload to a first-class
organ repo (architect, builder, CI, QA, docs — the siblings named in
VISION.md). It then retroactively evaluates Cerberus against that bar as the
reference case, using its real measured numbers.

The bar exists to prevent snowflakes. The Factory will mint a new organ only
when Crucible + Threshold evidence says the agent-config is reliable enough to
own a responsibility, not because a workload looks useful in a demo. A
workload that does not clear the bar stays a BB agent — cheap to run, cheap
to kill, no repo of its own.

## Doctrine

1. **Organs are promoted, not declared.** A workload becomes an organ only
   after it clears every gate below on live Crucible evidence, not on
   architecture or roadmap.
2. **The bar is the same for every organ.** A docs-sync agent and a review
   organ face the same four gates; only the budget target and the seeded
   findings corpus differ.
3. **No promotion without a closed measurement loop.** Threshold must have
   searched the config space under the budget target and reported the best
   config's score with a confidence interval. A single arena run is not
   promotion evidence.
4. **The bar is ratcheted, never relaxed for a favorite.** If Cerberus at
   its current numbers would not clear a sibling's bar, that is the signal
   to fix Cerberus, not to lower the floor.
5. **Promotion is reversible.** An organ whose incident rate or pass^k
   regresses below the floor for a rolling window is demoted back to a BB
   workload until evidence recovers.

## The four gates

### Gate 1 — Crucible eval score under a budget target

The agent-config must achieve a mean reward on a seeded Crucible arena that
beats the null baseline by a margin whose 95% confidence interval excludes
zero, at a per-run cost at or under a budget cap declared in the promotion
request.

| Field | Requirement |
| --- | --- |
| Arena | A versioned, seeded findings corpus owned by the requesting organ's domain (e.g., review-defects for Cerberus, doc-drift for a docs organ). |
| Metric | Mean reward over the arena, with the null (no-agent or random) baseline run on the same seed. |
| Significance | 95% CI on `score − null` excludes zero. |
| Budget | Per-run cost ≤ declared cap; total arena cost reported. |
| Adjudication | False positives / false negatives adjudicated through the Crucible human queue so reward is graded, not self-reported. |

A score that moves inside its noise floor is not a result. The promotion
request names the sample size, the seed, and the budget cap, and Threshold
attests that the reported config is the best it found under the cap.

### Gate 2 — Run count

The arena must be run enough times that the confidence interval is
meaningful and the consistency floor (Gate 3) has enough samples to be
statistically real.

| Field | Requirement |
| --- | --- |
| Minimum independent runs | 30 on the seeded arena. |
| Across | At least 2 distinct prompts/model versions if the organ's responsibility is prompt-sensitive. |
| Reproducibility | Same seed + config reproduces the verdict distribution within tolerance. |

### Gate 3 — pass^k consistency floor

The organ's verdict on the same input must be stable across independent runs.
`pass^k` is the probability that `k` independent runs of the same config on
the same seeded input all return the same verdict. A high mean reward with a
low pass^k means the agent is right on average and wrong on any given call —
exactly the false-confidence failure mode the Factory exists to prevent.

| Field | Requirement |
| --- | --- |
| Metric | `pass^k` — fraction of seeded inputs where all `k` runs agree. |
| Floor (k=3) | ≥ 0.60 |
| Floor (k=5) | ≥ 0.25 |
| Gating | A BB auto-trigger must not treat the organ's verdict as blocking until pass^k clears the floor. |

The floors are deliberately the binding gate. An organ can have a strong mean
reward and still fail here; that organ stays advisory-only.

### Gate 4 — Incident rate

Once the organ is integrated (running on real PRs / real merges in BB), its
live behavior is watched by Canary. The incident rate is the fraction of the
organ's actions over a rolling 30-day window that triggered a Canary incident
or required human remediation (reverted verdict, hotfix within 14 days,
credential egress, runaway spend).

| Field | Requirement |
| --- | --- |
| Window | Rolling 30 days, post-integration. |
| Floor | Incident rate ≤ 5% of actions. |
| Hard stops | Any credential egress or unbounded-spend incident is an immediate demotion regardless of rate. |
| Source | Canary incident events, not the organ's self-report. |

Gate 4 is measured only after the organ is live in BB on real work; it is the
retention condition, not the entry condition. An organ clears Gates 1–3 to be
promoted and must keep Gate 4 to stay promoted.

## Promotion process

1. **Workload proves useful** as a BB agent (triggered, scoped, receipts
   captured). No repo yet.
2. **Owner opens a promotion request** naming the responsibility, the
   seeded arena, the budget cap, and the proposed organ repo name.
3. **Crucible runs the arena** at the requested sample size; Threshold
   searches the config space under the cap and reports the best config's
   score, CI, and pass^k.
4. **The four gates are checked.** All four must pass; a failure at any gate
   blocks promotion and the workload stays on BB.
5. **On promotion**, the organ repo is created (or promoted from a stub),
   the agent-config is pinned to the Threshold-reported best config, and
   Canary begins incident tracking.
6. **Quarterly re-eval.** An organ that regresses below any floor for a
   rolling window is demoted back to a BB workload.

## Retroactive evaluation: Cerberus as the reference case

Cerberus is the first SDLC organ. It was promoted before this bar existed.
Evaluating it retroactively against the four gates is the honesty check: the
bar is only real if the reference organ can be measured against it, including
where it fails.

Evidence source: `~/.factory-lanes/groom/cerberus.md` (salvage groom report,
2026-07-01), citing the Threshold arena run `20260623T183514Z` and the
Crucible round-trip delivered 2026-06-30.

| Gate | Cerberus evidence | Verdict |
| --- | --- | --- |
| 1. Score under budget | Best config mean reward **0.7544**, **+0.5878** vs null, 95% CI **[+0.229, +0.947]**, total cost **$2.52**. CI excludes zero; cost is low. Crucible FP→TP adjudication round-trip delivered 2026-06-30 (flipped reward 0.8→1.0), so reward is graded, not self-reported. | **PASS** — score clears the bar with significance and adjudication. |
| 2. Run count | The arena run reports a mean reward with a 95% CI, implying sufficient runs for inference, but the run count is not separately attested and no second prompt/model version was exercised. | **CONDITIONAL** — likely sufficient for inference; formally incomplete until the run count and multi-config requirement are attested. |
| 3. pass^k consistency | **pass^5 = 0.0434** — the reviewer is wildly inconsistent run-to-run. Against the k=5 floor of 0.25, this is a hard fail. | **FAIL** — Cerberus must not be treated as a blocking auto-reviewer at this consistency. |
| 4. Incident rate | Not yet measurable: Cerberus is not integrated into BB as a live trigger (the BB trigger deployment is pending), so there is no rolling 30-day Canary incident window to score. | **NOT YET APPLICABLE** — pending BB integration. |

### Reading

Cerberus clears Gate 1 cleanly: its mean reward beats null with a CI that
excludes zero, at a cost well under any plausible budget cap, and the reward
is adjudicated rather than self-reported. That is genuine evidence the
agent-config does something real. Gate 2 is likely satisfied in spirit but
not formally attested.

Cerberus fails Gate 3 badly. A pass^5 of 0.0434 means that on the seeded
corpus, almost never do five independent runs agree. A reviewer that is right
on average but disagrees with itself on any given call is the textbook
false-confidence organ: operators learn to ignore it, and an ignored organ is
dead weight regardless of its mean reward. This is the single binding blocker
for trusting Cerberus as a blocking auto-reviewer, and it is exactly the
failure the promotion bar exists to catch before a sibling is minted.

Gate 4 cannot be scored until Cerberus runs live in BB on real PRs and Canary
is observing. This is consistent with the operator note on the ticket: do not
mint snowflakes until Cerberus is properly integrated into BB and
Crucible+Threshold produce evidence.

### Conclusion

Cerberus, measured against the promotion bar it should have been held to,
does not yet clear the consistency floor. It remains a first-class organ by
precedent (it was first), but its verdict must stay advisory, and no sibling
organ (architect, builder, CI, QA, docs) should be promoted until:

1. Cerberus's pass^k clears the k=5 floor (0.25) on a seeded findings corpus.
2. The closed measurement loop (Threshold searching config space, Crucible
   scoring on a cadence, the paired doctrine eval from backlog 020) is
   actually running.
3. Cerberus is integrated into BB as a live trigger and has a rolling 30-day
   Canary incident window below 5%.

The bar is ratcheted at these numbers. When the next organ is proposed, it
faces the same four gates on its own seeded arena — and Cerberus's
post-fix numbers become the floor a sibling must beat, not a waiver.
