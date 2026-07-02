# CI/quality-gate auditor flow

Status: draft
Date: 2026-07-01
Backlog: `backlog.d/006-ci-auditor-flow.md`

The CI/quality-gate auditor is a BB-triggered maintenance loop that audits
each repo's CI design, quality gates, tests, linter, and build for
correctness, safety, and performance. It proposes enforcement increases,
speedups, and cost cuts via PRs. It never lowers gates (doctrine).

## Trigger

- **On PR merge** — a merge to a repo's default branch triggers an audit of
  the changed CI surface (workflow files, gate scripts, test config).
- **Weekly** — a cron-style BB trigger runs a full audit per managed repo to
  catch drift in CI design that accumulates across merges.

The trigger is a BB policy decision; no model call is needed to decide whether
to run.

## Flow

```
merge / weekly ──▶ BB dispatch ──▶ CI-auditor agent
                                       │
                                       │ reads repo CI, gates, tests, linter
                                       │
                                       ▼
                                bb.maintenance_result.v1
                                       │
                            ┌──────────┼──────────┐
                            ▼          ▼          ▼
                       open PR   request input  complete card
```

1. BB receives the trigger and dispatches the CI-auditor workload with a
   scoped OpenRouter key and budget line.
2. The agent reads the repo's CI configuration: workflow files, gate scripts,
   test runner config, linter rules, build pipeline.
3. The agent evaluates: are gates actually guaranteeing correctness/safety?
   Are they performant? Is there dead config? Are there enforcement gaps
   (missing lint rules, untested paths, flaky tests)?
4. The agent emits a `bb.maintenance_result.v1` with one of three outcomes:
   - **open_pr** — a PR with gate improvements (enforcement increases,
     speedups, cost cuts), proof attached.
   - **request_input** — the agent hit an ambiguity (e.g., a gate that looks
     intentional but might be stale); the Powder card pauses.
   - **complete_card** — no improvements found; the card closes with proof.

## Doctrine

- **Never lower a gate.** The auditor may tighten, add, or speed up gates. It
  must not disable a test, loosen a lint rule, or weaken a threshold to get
  green. Any proposal that lowers enforcement is rejected at the BB boundary.
- **Evidence over vibes.** Every proposal carries before/after evidence: gate
  time, cost delta, or a caught-failure class the old gate missed.
- **Fingerprint-gate alignment.** The auditor checks for the
  tailnet-name/personal-path fingerprint gate pattern from the groom sweep —
  fail CI on secrets, local paths, or tailnet hostnames in tracked files.

## Contracts

| Direction | Schema | Producer | Consumer |
| --- | --- | --- | --- |
| BB → Powder / remote host (result) | `bb.maintenance_result.v1` | bitterblossom | powder, remote host projection |

The maintenance result carries an `outcome` enum (`open_pr`, `request_input`,
`complete_card`). A result missing `outcome` is rejected at the Powder boundary.
A proposal that lowers a gate is rejected at the BB boundary before it reaches
the PR stage.

## Model choice

Cheap model for the read-and-classify pass (scan CI files, categorize gates).
Frontier model only if the improvement proposal itself requires judgment the
cheap model cannot provide. Model choice must be justified by a Crucible
measurement once Crucible runs for real.

## Budget and governance

- Per-agent OpenRouter key with a spend cap.
- Budget line declared in the BB workload definition.
- Total cost per run reported in `bb.run_telemetry.v1`.

## Acceptance oracle (from backlog 006)

- [ ] Trigger + agent defined in BB with scoped key + budget.
- [ ] First month: ≥3 merged gate improvements with before/after evidence
      (time, cost, or caught-failure class).
