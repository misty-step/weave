# Remote event projection

`weave.remote_event.v1` is the host-neutral envelope emitted by host adapters
before BB, Cerberus, Landmark, or Canary see a remote event. GitHub is the v1
source of truth, but consumers pin and test the Weave envelope instead of raw
GitHub webhook JSON.

The adapter keeps host identity and durable links explicit:

- `source.kind` and `source.host` name the forge that emitted the event.
- `source.external_id` and `host_payload.delivery_id` carry the native delivery
  id or closest equivalent.
- `repository` carries the repo identity independently from the event subject.
- `subject` names the thing that changed: PR, push, check run, workflow run,
  deployment, release, or comment.
- `host_payload.links` carries canonical host URLs for audit and replay.
- `host_payload.api_version` may carry the host API version used to interpret
  the payload, for example GitHub REST `2022-11-28`.
- `payload` carries normalized event details consumers may need but should not
  branch on unless the field is documented here.

## GitHub projection table

| GitHub event | Envelope subject | Envelope action | Required host links | Normalized payload |
| --- | --- | --- | --- | --- |
| `pull_request` `opened` | `subject.kind=pull_request`, `subject.id=pull_request.id`, `subject.number=number` | `opened` | PR HTML, REST API, diff, patch | `base_ref`, `head_ref`, `head_sha`, `draft`, `state`, optional `title` |
| `pull_request` `synchronize` | same PR subject | `synchronize` | PR HTML, REST API, commits | `base_ref`, `head_ref`, `head_sha`, `draft`, `state` |
| `push` | `subject.kind=push`, `subject.id=ref`, `subject.ref=ref`, `subject.sha=after` | `push` | compare, commits | `before`, `after`, `ref`, `created`, `deleted`, `forced` |
| `check_run` `completed` | `subject.kind=check_run`, `subject.id=check_run.id`, `subject.sha=head_sha` | `completed` | check HTML, check REST API | `name`, `status`, `conclusion`, `head_sha`, `pull_requests` |
| `workflow_run` `completed` | `subject.kind=workflow_run`, `subject.id=workflow_run.id`, `subject.sha=head_sha` | `completed` | workflow run HTML, REST API, logs | `workflow_name`, `status`, `conclusion`, `head_branch`, `head_sha` |
| `deployment_status` | `subject.kind=deployment`, `subject.id=deployment.id`, `subject.sha=deployment.sha` | deployment status state such as `success` | deployment REST API, status REST API, environment activity | `environment`, `state`, `target_url` |
| `release` `published` | `subject.kind=release`, `subject.id=tag_name` | `published` | release HTML, release REST API | `tag_name`, `target_commitish`, `draft`, `prerelease` |
| `issues` `opened` | `subject.kind=issue`, `subject.id=issue.id`, `subject.number=issue.number` | `opened` | issue HTML, issue REST API | `state`, `title`, `labels` |
| `issue_comment` `created` | `subject.kind=comment`, `subject.id=comment.id`, `subject.number=issue.number` | `created` | comment HTML, comment REST API | `issue_number`, `issue_kind`, bounded `body_excerpt` |

Adapters may add optional payload fields, but consumers must not require fields
outside this table without updating the schema fixtures and conformance check.

## Idempotency

For GitHub, `idempotency_key` starts with the delivery id and includes the
subject, action, and revision when one exists:

```text
github:<delivery-id>:pull_request:<number>:<action>:<head-sha>
github:<delivery-id>:push:<ref>:<after-sha>
github:<delivery-id>:check_run:<check-run-id>:completed:<conclusion>
github:<delivery-id>:release:<tag-name>:published
```

This lets BB and downstream consumers dedupe host retries without treating
later PR synchronizations or new check conclusions as duplicates.

## Merge policy

The glass-902 / bastion-906 primitive rides on the envelope as optional
`policy.merge_policy` metadata:

- `agent-mergeable` means the host event is eligible for automated merge once
  the repo gate and required review policy are satisfied.
- `human-review` means automation may build, test, summarize, or request
  review, but it must not merge without explicit human approval.

The field is optional because push, release, deployment, and some check events
do not always have a merge decision attached. PR and issue-comment adapters
should populate it when branch rules, labels, comment commands, repo rules, or
Powder card policy make the decision known. The deterministic merge decision
belongs to BB or the repo merge controller; the envelope only carries the host
projection and the policy inputs needed for that controller to decide.

## Proof fixtures

- `docs/fixtures/host/github/pull_request.opened.real.json` is the raw GitHub
  webhook-shaped fixture.
- `docs/fixtures/contracts/weave.remote_event.v1.github-pr-opened-from-webhook.json`
  is the normalized envelope derived from it.
- `scripts/remote-event-conformance.cjs` validates every valid remote-event
  fixture and checks that the raw GitHub fixture maps to the normalized one by
  deterministic field projection.
