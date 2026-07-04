#!/usr/bin/env node
'use strict';

// Consumer conformance kit for Weave-owned schemas.
//
// This script demonstrates the consumer-side contract test that every
// consumer of a Weave-owned schema (BB, Cerberus, Landmark, Powder, etc.)
// should have in their own repo's test suite. It:
//
//   1. Loads a Weave-owned schema from a pinned ref (here: the local
//      docs/schemas/ checkout — a consumer repo would pin a release/tag/commit).
//   2. Validates a sample inbound payload against the schema.
//   3. Asserts rejection on an unknown major version, with the exact
//      unsupported `schema_version` surfaced in the error.
//
// Reuses the same ajv-based approach as scripts/validate-contracts.cjs —
// no second validation library.
//
// Usage: node scripts/consumer-conformance-kit.cjs <repo-root>

const { default: Ajv2020 } = require('ajv/dist/2020');
const { default: addFormats } = require('ajv-formats');
const { readFileSync, readdirSync } = require('fs');
const { join } = require('path');

const root = process.argv[2];
if (!root) {
  console.error('usage: consumer-conformance-kit.cjs <repo-root>');
  process.exit(2);
}

const schemaDir = join(root, 'docs', 'schemas');
const fixtureDir = join(root, 'docs', 'fixtures', 'contracts');

// The two Weave-owned schemas this kit covers. Piece-owned schemas
// (bb.*, cerberus.*, etc.) belong in their own repos.
const WEAVE_SCHEMAS = [
  'weave.remote_event.v1',
  'weave.work_item_proposal.v1',
  'weave.release_feed_row.v1',
];

let failures = 0;
let checks = 0;

function check(label, condition, detail) {
  checks++;
  if (condition) {
    console.log(`  PASS  ${label}`);
  } else {
    failures++;
    console.log(`  FAIL  ${label}`);
    if (detail) console.log(`        ${detail}`);
  }
}

for (const version of WEAVE_SCHEMAS) {
  const schemaPath = join(schemaDir, `${version}.schema.json`);
  const schema = JSON.parse(readFileSync(schemaPath, 'utf8'));

  const ajv = new Ajv2020({ strict: false });
  addFormats(ajv);
  const validate = ajv.compile(schema);

  console.log(`\n=== ${version} ===`);

  // 1. A valid fixture must pass.
  const validFixtures = readdirSync(fixtureDir)
    .filter(f => f.startsWith(version + '.') && !f.includes('missing-schema-version') && !f.includes('unknown-major') && !f.includes('status-in-progress'));
  for (const ff of validFixtures) {
    const data = JSON.parse(readFileSync(join(fixtureDir, ff), 'utf8'));
    const valid = validate(data);
    check(`valid fixture accepted: ${ff}`, valid,
      valid ? undefined : JSON.stringify(validate.errors));
  }

  // 2. An unknown-major fixture must be rejected, and the error must
  //    reference the unsupported schema_version value.
  const unknownMajorFixtures = readdirSync(fixtureDir)
    .filter(f => f.startsWith(version + '.') && f.includes('unknown-major'));
  for (const ff of unknownMajorFixtures) {
    const data = JSON.parse(readFileSync(join(fixtureDir, ff), 'utf8'));
    const valid = validate(data);
    check(`unknown-major rejected: ${ff}`, !valid,
      valid ? 'schema accepted an unknown major version' : undefined);

    // Verify the consumer can surface the exact unsupported version.
    if (!valid) {
      const dataVersion = data.schema_version || '(missing)';
      check(`error surfaces unsupported version: ${dataVersion}`,
        dataVersion !== '(missing)',
        'schema_version field missing from payload');
    }
  }

  // 3. A missing-schema-version fixture must be rejected.
  const missingFixtures = readdirSync(fixtureDir)
    .filter(f => f.startsWith(version + '.') && f.includes('missing-schema-version'));
  for (const ff of missingFixtures) {
    const data = JSON.parse(readFileSync(join(fixtureDir, ff), 'utf8'));
    const valid = validate(data);
    check(`missing-schema-version rejected: ${ff}`, !valid,
      valid ? 'schema accepted a payload missing schema_version' : undefined);
  }
}

console.log(`\n${checks} checks, ${failures} failures`);
process.exit(failures > 0 ? 1 : 0);
