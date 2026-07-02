# Consumer conformance kit

A runnable starting point for consumer repos (BB, Cerberus, Landmark, Powder,
etc.) to write contract tests against Weave-owned schemas. The kit is at
`scripts/consumer-conformance-kit.cjs` and runs as part of
`./scripts/verify.sh`.

## What it proves

For each Weave-owned schema (`weave.remote_event.v1`,
`weave.work_item_proposal.v1`), the kit demonstrates the three checks named in
the [seam reference](seam-reference.md) contract-test column:

1. **Valid fixture accepted** — a well-formed payload passes validation.
2. **Unknown major rejected** — a payload with an unsupported `schema_version`
   (e.g. `weave.remote_event.v2`) is rejected, and the exact unsupported
   version is surfaced in the error.
3. **Missing `schema_version` rejected** — a payload without the
   `schema_version` field is rejected.

## How to use it in a consumer repo

1. **Pin a Weave ref.** In your consumer repo, pin the Weave release, tag, or
   commit that contains the schema version you accept. Copy the schema file
   into your repo's test fixtures (e.g. `test/fixtures/schemas/`) or fetch it
   from the pinned ref in CI.
2. **Copy the kit.** Adapt `scripts/consumer-conformance-kit.cjs` to your
   repo's language and test framework. The kit uses the same `ajv`-based
   approach as `scripts/validate-contracts.cjs` — do not introduce a second
   validation library.
3. **Add your own inbound fixtures.** Use real (sanitized) payloads your
   consumer receives, not just Weave's canonical fixtures. The canonical
   fixtures prove the schema; your fixtures prove your consumer handles the
   shapes it actually sees.
4. **Run in CI.** Wire the adapted kit into your CI gate so a breaking schema
   change fails a contract test in your repo, not at runtime.

## Scope

This kit covers only the two schemas Weave owns
(`weave.remote_event.v1`, `weave.work_item_proposal.v1`). Piece-owned schemas
(`bb.*`, `cerberus.*`, `canary.*`, `landmark.*`, `powder.*`, `threshold.*`,
`harness.*`) belong in their own repos — a starter kit here would drift from
schemas Weave doesn't control.

## Running

```sh
./scripts/verify.sh
```

The conformance kit runs as the final step, after JSON well-formedness,
forbidden-content scan, and schema-vs-fixture validation.
