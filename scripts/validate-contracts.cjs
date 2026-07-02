#!/usr/bin/env node
'use strict';

const { default: Ajv2020 } = require('ajv/dist/2020');
const { default: addFormats } = require('ajv-formats');
const { readFileSync, readdirSync } = require('fs');
const { join, basename } = require('path');

const root = process.argv[2];
if (!root) {
  console.error('usage: validate-contracts.cjs <repo-root>');
  process.exit(2);
}

const schemaDir = join(root, 'docs', 'schemas');
const fixtureDir = join(root, 'docs', 'fixtures', 'contracts');

const schemas = readdirSync(schemaDir).filter(f => f.endsWith('.schema.json'));
const allFixtures = readdirSync(fixtureDir).filter(f => f.endsWith('.json'));

// A fixture is "valid" unless its name contains an invalid-marker token.
const INVALID_MARKERS = ['missing-schema-version', 'unknown-major'];

const isInvalid = (name) => INVALID_MARKERS.some(m => name.includes(m));

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

for (const sf of schemas) {
  const schemaPath = join(schemaDir, sf);
  const schema = JSON.parse(readFileSync(schemaPath, 'utf8'));
  const ajv = new Ajv2020({ strict: false });
  addFormats(ajv);
  const validate = ajv.compile(schema);

  // Derive the schema_version prefix from the $id or filename.
  const version =
    (schema.$id && schema.$id.replace(/.*\/(.+)\.schema\.json$/, '$1')) ||
    sf.replace(/\.schema\.json$/, '');

  const matching = allFixtures.filter(f => f.startsWith(version + '.'));
  if (matching.length === 0) {
    check(`${sf}: has at least one fixture`, false, 'no matching fixtures found');
    continue;
  }

  for (const ff of matching) {
    const fixturePath = join(fixtureDir, ff);
    const data = JSON.parse(readFileSync(fixturePath, 'utf8'));
    const valid = validate(data);
    const expected = isInvalid(ff) ? 'reject' : 'accept';
    const actual = valid ? 'accept' : 'reject';
    const label = `${ff} — expected ${expected}`;
    if (expected === 'reject') {
      check(label, !valid,
        !valid ? undefined : 'schema accepted a fixture that should be rejected');
    } else {
      check(label, valid,
        valid ? undefined : 'schema rejected a valid fixture: ' + JSON.stringify(validate.errors));
    }
  }
}

console.log(`\n${checks} checks, ${failures} failures`);
process.exit(failures > 0 ? 1 : 0);
