# Remote and review primitives for an agent-majority fleet

Status: recommendation pending operator decision
Date: 2026-07-01
Backlog: `backlog.d/002-remote-and-review-primitive-research.md`

## Recommendation

Keep GitHub as the public source of truth for the next Weave slice, but stop
letting GitHub's pull request model be the architecture. Treat GitHub PRs as
the first projection of a host-neutral review event contract. Add two small
experiments immediately:

1. Adopt stacked changes as the default agent review discipline on GitHub.
2. Pilot `jj` as a local agent workspace primitive while preserving Git refs
   and GitHub PRs at publication time.

In parallel, mirror all Factory repos to a self-hosted Forgejo or Gitea
instance as a cold-read fallback and webhook/API laboratory. Promote a self-
hosted forge to source-of-truth only if GitHub outage/rate-limit incidents block
the fleet often enough to justify operating the forge. Do not migrate to Gerrit
as the default remote unless the review model itself becomes the product center;
Gerrit is review-native, but it would force the largest workflow and ecosystem
migration.

This is an intentionally boring first answer: GitHub remains the cheapest
coordination surface because Bitterblossom, Cerberus, Landmark, existing public
repos, and operator muscle memory already orbit it. The engineering move is to
extract the real primitive now: a versioned remote/review event envelope that
can be produced by GitHub today and by GitLab, Forgejo/Gitea, Gerrit, or a patch
queue later.

## Fleet requirements

The remote/review layer must support:

- event-triggered Bitterblossom workloads for PR open/update, push, checks,
  deployments, releases, and incidents;
- Cerberus advisory reviews with durable artifacts and host projection;
- Landmark release intelligence on merge/release;
- agent-heavy write volume without secondary rate-limit storms;
- outage tolerance when the public forge is degraded;
- public-able repos with no private instance data in tracked files;
- migration to a different host without changing Powder, BB, Cerberus, Canary,
  Crucible, Threshold, Landmark, or Harness Kit domain contracts.

## Remote comparison

| Remote | Fit | Strengths | Risks | Migration cost |
| --- | --- | --- | --- | --- |
| GitHub.com | Keep as v1 source of truth | Best existing fit. Public org already lives there. Rich webhook events for pull requests, pushes, checks, deployments, workflow jobs/runs, and releases. REST API and Actions are already the ecosystem default. GitHub publishes current and historical service status. | SaaS outage dependency. REST primary rate limit is 5,000 authenticated requests/hour, 15,000/hour for some Enterprise Cloud contexts, with secondary limits such as 100 concurrent requests, 80 content-generating requests/minute, and 500 content-generating requests/hour. PRs are host-native, not Git-native. | Low: 0.5-1 week to add a host-neutral event envelope and backpressure discipline. |
| GitLab.com or GitLab self-managed | Viable v2 candidate | Merge requests, webhooks, CI, API, and self-managed control. Self-managed administrators can tune many limits. GitLab documents project/group webhook limits and instance limits. | Heavier product and ops surface than Gitea/Forgejo. GitLab.com remains SaaS unless self-managed. Existing Misty Step automations would need GitLab adapters. | Medium to high: 3-5 weeks for source migration, CI parity, secrets, BB/Cerberus/Landmark adapters, and operator retraining. |
| Gitea or Forgejo self-hosted | Best standby/lab candidate | Lightweight self-hosted Git hosting with PRs, webhooks, API docs, Actions runners, and configurable webhook/action behavior. Forgejo exposes operational knobs for webhook queues, delivery timeout, allowed hosts, Actions timeouts, API paging, and federation flags. Good outage-control story. | Smaller ecosystem. Actions are compatible in spirit but still need parity testing. The operator owns backups, upgrades, security, runner capacity, and webhook delivery reliability. | Medium: 2-4 weeks for a serious source-of-truth move; about 1 week for read mirror plus event lab. |
| Gerrit | Strong review primitive, weak default remote fit | Changes, patch sets, labels, reviewers, rebasing, comments, submit requirements, and REST APIs are review-native. Submit requirements model merge gates directly. | Biggest culture and tooling migration. Existing GitHub PR-centric tools, release Actions, and public contribution expectations would need replacement or projection. | High: 4-8+ weeks, plus ongoing operator retraining. Best reserved for a bounded review-lab proof. |

## Review and merge primitive comparison

| Primitive | What it buys | What it costs | Recommendation |
| --- | --- | --- | --- |
| Host pull/merge requests | Ubiquitous review object with comments, checks, merge buttons, webhooks, and API projection. Works across GitHub, GitLab, Gitea, and Forgejo with platform-specific details. | Not Git-native. Encourages large branches unless disciplined. Host outage/rate limits can stall review and merge. | Keep as the public v1 merge projection. Do not let it be the only internal contract. |
| Stacked diffs / stacked PRs | Smaller dependent changes, faster review, cleaner agent decomposition, and better conflict isolation. Can run on top of GitHub through tools such as Graphite. | Requires agents and reviewers to understand stack order, restacking, and merge sequencing. Tooling adds another dependency. | Adopt as a workflow discipline immediately, with Graphite-style behavior as reference. |
| `jj` changes and bookmarks | Local agent work becomes change-oriented instead of branch-oriented. Change IDs, revsets, and bookmarks improve recovery, rebasing, and tracking of many concurrent agent edits while preserving Git interoperability. | `jj` does not replace the remote review object by itself. Agents must learn bookmarks, push behavior, and divergence recovery. Force-push semantics still need policy. | Pilot locally for agent workspaces. Publish via Git refs and PRs until the remote layer changes. |
| Gerrit changes / patch sets / submit requirements | Review object is the core primitive. Patch sets are first-class. Submit requirements express gates before submit. REST APIs support listing, reviewing, revising, rebasing, and submitting changes. | High migration burden and lower alignment with current GitHub-triggered Factory tools. | Keep as the north-star review-native alternative; run a bounded proof only after the event envelope exists. |
| Git email patches / patch queues / `request-pull` | Git-native and host-neutral. `format-patch`, `send-email`, and `am` preserve commit metadata through mailbox patches; `request-pull` asks an upstream to pull from a published ref. | Weak operator UX, weak default webhook surface, SMTP/list operations, and more custom indexing needed before agents can observe/review/claim work durably. | Useful fallback and archival primitive. Not the default fleet coordination layer unless wrapped by a product. |

## Decision details

### Why not migrate immediately?

The current pain is real but the Factory has not yet separated the host event
contract from the host. Moving the source of truth first would force every
piece to relearn the forge while the data model is still implicit. The first
durable move is `weave.remote_event.v1`: a normalized event envelope with
`schema_version`, source host, repository, subject, actor, action, timestamps,
idempotency key, and links back to the host payload. BB, Cerberus, Landmark, and
Canary consume that envelope, not raw GitHub payloads.

### Why stacked diffs now?

Agent-majority work wants small reviewable units. Stacked changes let an agent
split a large feature into a dependency chain without waiting for every lower
PR to merge. This preserves GitHub's social/review surface while reducing PR
size and making Cerberus review packets smaller and more focused.

The operational discipline is now
[stacked-diff discipline](stacked-diff-discipline.md): use native GitHub branch
bases, explicit stack metadata in PR bodies, restack with ordinary rebase, and
merge bottom-up with a default-branch remote-sync check.

### Why `jj` as a local primitive?

The fleet problem is not only remote review. It is also local recovery across
many agents editing many repos. `jj` gives stable change IDs, revsets, and a
workflow where a local agent can reorganize work before choosing which bookmark
to push. The remote stays Git-compatible. The pilot should measure whether this
reduces conflict recovery and branch bookkeeping without breaking existing
gates.

### Why Forgejo/Gitea as standby?

Forgejo/Gitea is the cheapest path to an owned forge laboratory. It can mirror
GitHub, exercise webhooks/actions under our control, and prove whether a self-
hosted forge really reduces operational pain before the fleet pays migration
cost. It also keeps the alternative close to GitHub's PR mental model.

### Why not Gerrit first?

Gerrit is the cleanest answer if the review primitive is all that matters. It
is not the cleanest answer for an ecosystem that already depends on GitHub org
visibility, Actions, GitHub Apps, release workflows, and public contribution
defaults. Use Gerrit as an adversarial design reference for submit requirements
and patch-set modeling, not as the first migration.

## Migration plan

| Phase | Work | Exit evidence |
| --- | --- | --- |
| 0 | Keep GitHub source-of-truth. Add `weave.remote_event.v1` fixtures for PR open/update, push, check run, workflow run, deployment, release, and issue/comment events. | BB/Cerberus/Landmark test against normalized fixtures, not raw GitHub payloads. |
| 1 | Stack discipline on GitHub. Add review guidance and branch naming for stacked PRs. Pilot `jj` locally in one low-risk repo. | At least three stacked PR sequences land; no gate loss; agent receipt compares Git-only vs `jj` conflict/recovery cost. |
| 2 | Forgejo/Gitea mirror. Mirror repos, configure webhooks/actions, replay normalized events into a staging BB queue. | Same `weave.remote_event.v1` consumers accept GitHub and Forgejo/Gitea fixtures. |
| 3 | Source-of-truth decision. Compare incident rate, operator time, API limits, CI parity, and release workflow fit. | Operator either keeps GitHub or promotes self-hosted forge with a dated decision record. |
| Later | Gerrit proof if review gates become the binding constraint. | One repo review flow proves Cerberus, BB, Landmark, and contract tests through Gerrit changes. |

## Source notes

- GitHub REST rate limits: https://docs.github.com/rest/using-the-rest-api/rate-limits-for-the-rest-api
- GitHub webhook events: https://docs.github.com/en/webhooks/webhook-events-and-payloads
- GitHub status: https://www.githubstatus.com/
- GitLab instance limits: https://docs.gitlab.com/administration/instance_limits/
- GitLab webhooks: https://docs.gitlab.com/user/project/integrations/webhooks/
- Gitea webhooks: https://docs.gitea.com/usage/webhooks
- Gitea pull requests: https://docs.gitea.com/usage/pull-request
- Gitea Actions design: https://docs.gitea.com/usage/actions/design
- Forgejo configuration: https://forgejo.org/docs/v15.0/admin/config-cheat-sheet/
- Jujutsu GitHub workflow: https://jj-vcs.github.io/jj/latest/github/
- Jujutsu revsets: https://jj-vcs.github.io/jj/latest/revsets/
- Gerrit changes API: https://gerrit-review.googlesource.com/Documentation/rest-api-changes.html
- Gerrit submit requirements: https://gerrit-review.googlesource.com/Documentation/config-submit-requirements.html
- Graphite stacked diffs: https://graphite.dev/guides/stacked-diffs
- Git patch primitives: https://git-scm.com/docs/git-format-patch, https://git-scm.com/docs/git-send-email, https://git-scm.com/docs/git-am, https://git-scm.com/docs/git-request-pull
