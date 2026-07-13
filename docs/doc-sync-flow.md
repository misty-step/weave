# Doc-sync agent flow

Status: draft
Date: 2026-07-01
Work item: Powder card `weave-005`

The doc-sync agent is a BB-triggered maintenance loop that keeps each managed
repo's documentation accurate using the harness-kit document skill with an
affordable model. It is the first instance of the harness-kit-skill →
focused-bespoke-agent pattern.

## Trigger

- **On PR merge** — the `weave.remote_event.v1` merge event reaches BB, which
  dispatches the doc-sync workload for the affected repo.
- **Daily** — a cron-style BB trigger runs the doc-sync workload per managed
  repo to catch drift not caused by a single merge.

The trigger is a BB policy decision; no model call is needed to decide whether
to run.

## Flow

```
merge event ──▶ BB dispatch ──▶ doc-sync agent (harness-kit document skill)
                                     │
                                     │ reads repo, finds doc drift
                                     │
                                     ▼
                              bb.maintenance_result.v1
                                     │
                          ┌──────────┼──────────┐
                          ▼          ▼          ▼
                     open PR   request input  complete card
```

1. BB receives the trigger and dispatches the doc-sync workload with a scoped
   OpenRouter key and budget line.
2. The agent runs the harness-kit document skill (declared via
   `harness.skill_bundle_manifest.v1`) against the repo.
3. The agent identifies doc drift: stale references, missing docs, broken
   links, outdated examples.
4. The agent emits a `bb.maintenance_result.v1` with one of three outcomes:
   - **open_pr** — a PR with doc fixes, proof attached.
   - **request_input** — the agent hit an ambiguity it cannot resolve; the
     Powder card pauses.
   - **complete_card** — no drift found, or drift already fixed; the card
     closes with proof.

## Contracts

| Direction | Schema | Producer | Consumer |
| --- | --- | --- | --- |
| Harness Kit → BB (skill binding) | `harness.skill_bundle_manifest.v1` | harness-kit | BB workload definitions |
| BB → Powder / remote host (result) | `bb.maintenance_result.v1` | bitterblossom | powder, remote host projection |

The skill bundle manifest must declare `schema_version` and a referenced eval
result. BB refuses a skill-bound workload if either is missing.

The maintenance result carries an `outcome` enum (`open_pr`, `request_input`,
`complete_card`). A result missing `outcome` is rejected at the Powder boundary.

## Model choice

Cheap model; frontier tokens are not required. Model choice must be justified
by a Crucible measurement once Crucible runs for real — not vibes.

## Budget and governance

- Per-agent OpenRouter key with a spend cap.
- Budget line declared in the BB workload definition.
- Total cost per run reported in `bb.run_telemetry.v1`.

## Acceptance oracle (from backlog 005)

- [ ] Trigger + agent defined in BB with its own scoped key and budget line.
- [ ] One week of runs: docs drift found and fixed via PRs on ≥2 repos; cost
      within budget.
- [ ] Model choice justified by a Crucible measurement once Crucible runs.
