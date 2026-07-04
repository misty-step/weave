#!/usr/bin/env node
'use strict';

const { default: Ajv2020 } = require('ajv/dist/2020');
const { default: addFormats } = require('ajv-formats');
const { readFileSync } = require('fs');
const { join } = require('path');

const root = process.argv[2] || process.cwd();

const SCHEMAS = {
  'weave.work_item_proposal.v1': 'docs/schemas/weave.work_item_proposal.v1.schema.json',
  'powder.card_event.v1': 'docs/schemas/powder.card_event.v1.schema.json',
  'landmark.release-kit.v1': 'docs/schemas/landmark.release-kit.v1.schema.json',
  'weave.release_feed_row.v1': 'docs/schemas/weave.release_feed_row.v1.schema.json',
};

let checks = 0;
let failures = 0;

function readJson(relPath) {
  return JSON.parse(readFileSync(join(root, relPath), 'utf8'));
}

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

function compile(schemaVersion) {
  const schema = readJson(SCHEMAS[schemaVersion]);
  const ajv = new Ajv2020({ strict: false });
  addFormats(ajv);
  return ajv.compile(schema);
}

const validators = Object.fromEntries(
  Object.keys(SCHEMAS).map((schemaVersion) => [schemaVersion, compile(schemaVersion)])
);

function validateFixture(schemaVersion, relPath) {
  const data = readJson(relPath);
  const validate = validators[schemaVersion];
  const valid = validate(data);
  check(`${schemaVersion}: ${relPath}`, valid, valid ? undefined : JSON.stringify(validate.errors));
  return data;
}

function nondecreasing(values) {
  return values.every((value, index) => index === 0 || values[index - 1] <= value);
}

function stableStringify(value) {
  if (Array.isArray(value)) {
    return `[${value.map(stableStringify).join(',')}]`;
  }
  if (value && typeof value === 'object') {
    return `{${Object.keys(value).sort().map((key) =>
      `${JSON.stringify(key)}:${stableStringify(value[key])}`
    ).join(',')}}`;
  }
  return JSON.stringify(value);
}

console.log('==> Replaying weave-906 cross-repo thread fixtures');

const manifest = readJson('docs/fixtures/thread/weave-906/replay.json');
const expected = manifest.expected;

check('Canary incident fixture has an incident id', Boolean(manifest.canary_incident.incident_id));
check(
  'Canary incident timeline includes durable write readback',
  ['remediation_claim.created', 'annotation.added'].every((event) =>
    manifest.canary_incident.timeline.includes(event)
  )
);

const proposal = validateFixture('weave.work_item_proposal.v1', manifest.proposal_fixture);
check('incident id maps to proposal source', proposal.source.external_id === manifest.canary_incident.incident_id);
check('proposal source is Canary', proposal.source.kind === expected.proposal_source_kind);
check('proposal subject is an incident', proposal.subject.kind === expected.proposal_subject_kind);
check('proposal cannot bypass Powder lifecycle', proposal.status === 'proposed');

const cardEvents = manifest.card_event_fixtures.map((fixture) =>
  validateFixture('powder.card_event.v1', fixture)
);
check(
  'Powder event sequence is card-created -> moved-to-ready -> completed',
  JSON.stringify(cardEvents.map((event) => event.event_type)) === JSON.stringify(expected.card_lifecycle)
);
const createdCard = cardEvents[0].card;
check('Powder created card title comes from proposal', createdCard.title === proposal.proposed_card.title);
check('Powder created card body points at proposal description ref', createdCard.body === proposal.proposed_card.description_ref);
check(
  'Powder created card priority comes from proposal',
  createdCard.priority.toLowerCase() === proposal.proposed_card.priority.toLowerCase()
);
check(
  'Powder created card labels include proposal labels',
  proposal.proposed_card.labels.every((label) => createdCard.labels.includes(label))
);
check(
  'Powder lifecycle stays on one card id',
  cardEvents.every((event) => event.card.id === expected.card_id)
);
check(
  'Powder lifecycle timestamps are monotonic',
  nondecreasing(cardEvents.map((event) => event.occurred_at))
);
check(
  'Powder completed event carries proof',
  Boolean(cardEvents.find((event) => event.event_type === 'completed')?.change?.proof)
);

const releaseEvent = validateFixture('landmark.release-kit.v1', manifest.release_event_fixture);
const releaseVersion = releaseEvent.release.tag || releaseEvent.release.version;
check('Landmark release repository matches thread', releaseEvent.release.repository === expected.release_repository);
check('Landmark release version matches thread', releaseVersion === expected.release_version);
check(
  'Landmark release kit includes release-feed receiver contract',
  releaseEvent.producer_contracts.some((contract) => contract.id === 'release-feed-receiver')
);

const feedRow = validateFixture('weave.release_feed_row.v1', manifest.feed_row_fixture);
check('feed row repository indexes release event', feedRow.repository === releaseEvent.release.repository);
check('feed row version indexes release event', feedRow.version === releaseVersion);
check('feed row release URL indexes release event', feedRow.release_url === releaseEvent.release.release_url);
check('feed row stores Landmark release kit payload', feedRow.payload.schema_version === releaseEvent.schema_version);
check(
  'feed row preserves full Landmark release kit payload',
  stableStringify(feedRow.payload) === stableStringify(releaseEvent)
);
check(
  'feed row payload preserves release-feed receiver contract',
  feedRow.payload.producer_contracts.some((contract) => contract.id === 'release-feed-receiver')
);

console.log('\nStubbed live hops:');
for (const stub of manifest.stubs) {
  console.log(`  - ${stub}`);
}

console.log(`\n${checks} checks, ${failures} failures`);
process.exit(failures > 0 ? 1 : 0);
