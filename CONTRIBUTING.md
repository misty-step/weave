# Contributing to the Weave

The Weave repo is the composition spec: contracts, schemas, fixtures, and the
narrative that ties the pieces together. The pieces themselves live in their
own repos (see [VISION.md](VISION.md)). This guide covers how to contribute to
the composition layer.

## Repo layout

```
VISION.md                       — what the Weave is and why
README.md                       — entry point and piece links
docs/
  how-the-loop-runs.md          — onboarding narrative
  composition-contracts.md      — seam map and contract rules
  seam-reference.md             — per-seam reference cards
  remote-and-review-primitives.md
  sdlc-organ-promotion.md       — organ promotion bar
  schemas/                      — JSON Schemas owned by weave
  fixtures/contracts/           — valid and invalid contract fixtures
backlog.d/                      — one file per ticket
```

## What lives here vs. in the piece repos

The Weave owns **composition contracts** and the **host-neutral event envelope**
(`weave.remote_event.v1`). Piece repos own their own schemas and fixtures
(`bb.*`, `cerberus.*`, `crucible.*`, `canary.*`, `landmark.*`, `powder.*`,
`threshold.*`, `harness.*`).

When you add or change a Weave-owned schema, the schema file, fixtures, and
validation live here. When you add or change a piece-owned schema, it lives in
that piece's repo — the Weave only references it in
[composition contracts](docs/composition-contracts.md) and the
[seam reference](docs/seam-reference.md).

## Adding or changing a contract

1. **Pick the right repo.** If the schema is Weave-owned
   (`weave.*`), it lives in `docs/schemas/`. If it is piece-owned, the change
   belongs in the piece repo.
2. **Draft the schema.** Use JSON Schema draft 2020-12. Every payload carries
   `schema_version` in the form `<owner>.<noun>.v<major>`.
3. **Add fixtures.** At minimum one valid fixture under
   `docs/fixtures/contracts/`, named `<schema_version>.<case>.json`. Add
   invalid fixtures using a marker token in the filename
   (`missing-schema-version`, `unknown-major`, `status-in-progress`) — the
   validator treats any fixture whose name contains a marker as must-reject.
   See [composition contracts](docs/composition-contracts.md#schema-registry-shape)
   for the full layout.
4. **Update the seam reference.** If the seam is new or changed, add or update
   the card in [docs/seam-reference.md](docs/seam-reference.md) and the row in
   [docs/composition-contracts.md](docs/composition-contracts.md).
5. **Validate.** Run `./scripts/verify.sh` — it checks JSON well-formedness,
   scans fixtures for forbidden content (secrets, local paths, tailnet
   hostnames), and validates every fixture against its schema (valid fixtures
   must pass; invalid-marker fixtures must be rejected). A breaking change
   (field rename, type change, removed required field) requires a new major
   version.
6. **Open a PR.** One coherent slice per PR. Link the backlog ticket in the PR
   body.

## Contract rules (summary)

See [composition contracts](docs/composition-contracts.md) for the full rules.

- Every inter-piece payload MUST carry `schema_version`.
- Producers own schemas, fixtures, and compatibility policy. Consumers pin a
  release, tag, or commit.
- Additive optional fields are allowed within a major version. Anything else
  (rename, type change, enum change, idempotency semantics) needs a new major.
- Consumers reject unknown major versions and surface the exact version.
- Schemas MUST NOT carry secrets, private instance data, local paths, or
  tailnet-only hostnames.

## Adding a backlog ticket

Create a file in `backlog.d/` named `NNN-short-slug.md` where `NNN` is the next
free number. Include:

```markdown
# Title

Priority: P1|P2|P3 · Status: pending · Estimate: S|M|L

## Goal
One or two sentences on the desired outcome.

## Oracle
- [ ] Concrete, checkable acceptance criteria.

## Notes
Context, constraints, links to related tickets.
```

Update the ticket's `Status` as it moves through the lifecycle. When a ticket
is done, leave it in place with `Status: done` — the backlog is the history.

## Style

- Markdown for prose. JSON Schema (draft 2020-12) for schemas. JSON for
  fixtures.
- Prefer tables for the seam inventory and per-seam summaries. Prefer prose
  for narrative and rules.
- Use `.invalid` domains and placeholder SHAs in fixtures — no real instance
  data, no tailnet hostnames, no secrets.
- Internal links are relative (`docs/how-the-loop-runs.md`, not absolute URLs).

## PR conventions

- One PR per coherent slice. Small, verifiable wins beat big risky ones.
- Squash-merge. The commit subject is `docs: <summary>` or `chore: <summary>`.
- The PR body names the goal, why, and verification. Agent attribution trailers
  go in the commit body when an agent authored the change.

## What does not belong here

- Implementation of piece repos (BB, Powder, Cerberus, etc.) — those have
  their own repos.
- Secrets, instance data, local paths, tailnet hostnames.
- Aesthetic kit design work (operator-gated, lives in `~/Development/aesthetic`).
