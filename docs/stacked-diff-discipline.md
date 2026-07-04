# Stacked-Diff Discipline

Status: active discipline
Backlog: `weave-018`

Stacked diffs are the default shape for agent-majority features that are too
large for one reviewable PR but can be split into dependent slices. The Weave
uses native GitHub branches and PR bases for this discipline. Graphite-style
tooling is a reference behavior, not a required platform dependency.

## When to Stack

Use a stack when all of these are true:

- The feature is one coherent outcome, not unrelated work.
- A lower slice can be reviewed and merged while upper slices are still open.
- Each slice has its own proof command, diff summary, and rollback boundary.
- The dependency order is real: PR B needs PR A, not just chronological order.

Do not stack to hide churn, bypass review, or merge speculative scaffolding.
If a slice cannot stand as a reviewable unit with its own oracle, split it
differently.

## Branches

Name branches as a chain:

```text
codex/<card>-01-<slug>
codex/<card>-02-<slug>
codex/<card>-03-<slug>
```

Create the first branch from the default branch. Create every later branch from
the previous branch:

```sh
git switch master
git pull --ff-only
git switch -c codex/weave-018-01-discipline

git switch -c codex/weave-018-02-lane-briefs
```

Publish each PR with an explicit base:

```sh
gh pr create --base master --head codex/weave-018-01-discipline
gh pr create --base codex/weave-018-01-discipline --head codex/weave-018-02-lane-briefs
```

## PR Body Contract

Every stacked PR body names the stack. Use this block near the top:

```markdown
Stack:
- [ ] 1. <this PR or lower PR> — <scope>
- [ ] 2. <upper PR> — <scope>

Base: `<base-branch>`
Depends on: <PR URL or "none">
Merge order: bottom-up; after each merge, rebase/restack the remaining PRs and
verify the final default branch with `git rev-list --left-right --count`.
```

The PR summary still describes only that PR's diff. Do not make reviewers infer
which files belong to another slice.

## Restacking

Restack upper branches whenever a lower branch changes:

```sh
git switch codex/weave-018-02-lane-briefs
git fetch origin
git rebase origin/codex/weave-018-01-discipline
git push --force-with-lease
```

Use `--force-with-lease`, never blind force-push. If a collaborator pushed to
the same branch, stop and inspect before overwriting.

## Review

Reviewers and Cerberus evaluate only the diff introduced by the PR under
review, but they must understand the stack context:

- Verify the PR base is the previous stack branch or the default branch for the
  bottom PR.
- Distinguish defects introduced by this PR from defects already present in a
  lower PR.
- Treat missing lower-PR context as residual risk, not as evidence against the
  upper PR.
- Prefer comments that say "belongs in lower PR" when the issue is about stack
  partitioning.

## Merge Sequence

Merge bottom-up:

1. Merge the lowest PR into the default branch.
2. Update local default branch:

   ```sh
   git switch master
   git pull --ff-only
   ```

3. Rebase the next branch onto the updated default branch.
4. Change that PR's base to the default branch when it no longer depends on an
   unmerged lower PR.
5. Wait for checks again.
6. Merge the next PR.
7. Repeat until the stack is empty.

The known failure mode is merging an upper PR into its lower branch and calling
the stack done while `master` never received the final work. The closeout check
is mandatory:

```sh
git rev-list --left-right --count master...origin/master
```

For a shipped stack this must be `0 0` after the final merge and local
fast-forward.

## Receipts

The lane report records:

- stack PR URLs and base branches;
- merge order actually used;
- restack commands or GitHub base edits;
- checks read for every PR after its last restack;
- final default-branch `rev-list` result.
