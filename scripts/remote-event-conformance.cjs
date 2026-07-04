#!/usr/bin/env node
'use strict';

// Remote-event-specific conformance checks for weave.remote_event.v1.
//
// The generic validator proves fixtures accept/reject against JSON Schema.
// This script proves the adapter contract shape the schema cannot express
// cleanly: every valid fixture keeps source host, repository, host payload
// links, and idempotency data explicit, and the raw GitHub webhook fixture maps
// to the normalized envelope fixture by deterministic field projection.

const { default: Ajv2020 } = require('ajv/dist/2020');
const { default: addFormats } = require('ajv-formats');
const { readFileSync, readdirSync } = require('fs');
const { join } = require('path');

const root = process.argv[2];
if (!root) {
  console.error('usage: remote-event-conformance.cjs <repo-root>');
  process.exit(2);
}

const schemaPath = join(root, 'docs', 'schemas', 'weave.remote_event.v1.schema.json');
const fixtureDir = join(root, 'docs', 'fixtures', 'contracts');
const rawGithubFixture = join(root, 'docs', 'fixtures', 'host', 'github', 'pull_request.opened.real.json');
const mappedGithubFixture = join(
  fixtureDir,
  'weave.remote_event.v1.github-pr-opened-from-webhook.json',
);

const INVALID_MARKERS = [
  'missing-schema-version',
  'unknown-major',
  'status-in-progress',
  'missing-host-payload',
];

const REQUIRED_HOST_EVENTS = new Set([
  'pull_request',
  'push',
  'check_run',
  'workflow_run',
  'deployment_status',
  'release',
  'issues',
  'issue_comment',
]);

const REQUIRED_SUBJECT_KINDS = new Set([
  'pull_request',
  'push',
  'check_run',
  'workflow_run',
  'deployment',
  'release',
  'issue',
  'comment',
]);

let failures = 0;
let checks = 0;

function readJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
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

function linkByRel(event, rel) {
  return event.host_payload.links.find(link => link.rel === rel);
}

function actorKind(githubType) {
  if (githubType === 'Bot') return 'bot';
  if (githubType === 'User') return 'user';
  if (githubType === 'App') return 'app';
  if (githubType === 'Organization') return 'system';
  return 'unknown';
}

const schema = readJson(schemaPath);
const ajv = new Ajv2020({ strict: false });
addFormats(ajv);
const validate = ajv.compile(schema);

const fixtureNames = readdirSync(fixtureDir)
  .filter(name => name.startsWith('weave.remote_event.v1.') && name.endsWith('.json'));

const validFixtureNames = fixtureNames
  .filter(name => !INVALID_MARKERS.some(marker => name.includes(marker)));

const validEvents = validFixtureNames.map(name => {
  const event = readJson(join(fixtureDir, name));
  const valid = validate(event);
  check(`schema accepts remote event fixture: ${name}`, valid,
    valid ? undefined : JSON.stringify(validate.errors));
  return { name, event };
});

for (const { name, event } of validEvents) {
  check(`${name}: source host is explicit`, Boolean(event.source && event.source.host));
  check(`${name}: repository full_name is explicit`,
    Boolean(event.repository && event.repository.full_name));
  check(`${name}: subject kind/id are explicit`,
    Boolean(event.subject && event.subject.kind && event.subject.id));
  check(`${name}: actor id/login are explicit`,
    Boolean(event.actor && event.actor.id && event.actor.login));
  check(`${name}: action is explicit`, Boolean(event.action));
  check(`${name}: timestamps are explicit`,
    Boolean(event.produced_at && event.occurred_at));
  check(`${name}: idempotency key is explicit`, Boolean(event.idempotency_key));
  check(`${name}: host payload links are explicit`,
    Boolean(event.host_payload && event.host_payload.links && event.host_payload.links.length > 0));
}

const hostEvents = new Set(validEvents.map(({ event }) => event.host_payload.event_name));
for (const eventName of REQUIRED_HOST_EVENTS) {
  check(`fixture coverage includes GitHub ${eventName}`, hostEvents.has(eventName));
}

const subjectKinds = new Set(validEvents.map(({ event }) => event.subject.kind));
for (const kind of REQUIRED_SUBJECT_KINDS) {
  check(`fixture coverage includes subject kind ${kind}`, subjectKinds.has(kind));
}

const pullRequestActions = new Set(
  validEvents
    .filter(({ event }) => event.host_payload.event_name === 'pull_request')
    .map(({ event }) => event.action),
);
check('pull_request fixtures include opened', pullRequestActions.has('opened'));
check('pull_request fixtures include synchronize', pullRequestActions.has('synchronize'));

const rawGithub = readJson(rawGithubFixture);
const mappedGithub = readJson(mappedGithubFixture);
const rawHeaders = rawGithub.headers;
const rawBody = rawGithub.body;
const rawPullRequest = rawBody.pull_request;
const rawRepository = rawBody.repository;
const rawSender = rawBody.sender;
const rawBase = rawPullRequest.base;
const rawHead = rawPullRequest.head;

const expectedIdempotencyKey = [
  'github',
  rawHeaders['X-GitHub-Delivery'],
  'pull_request',
  String(rawBody.number),
  rawBody.action,
  rawHead.sha,
].join(':');

check('mapped GitHub webhook fixture validates against schema', validate(mappedGithub),
  validate.errors ? JSON.stringify(validate.errors) : undefined);
check('GitHub event header maps to host_payload.event_name',
  mappedGithub.host_payload.event_name === rawHeaders['X-GitHub-Event']);
check('GitHub delivery header maps to source.external_id',
  mappedGithub.source.external_id === rawHeaders['X-GitHub-Delivery']);
check('GitHub delivery header maps to host_payload.delivery_id',
  mappedGithub.host_payload.delivery_id === rawHeaders['X-GitHub-Delivery']);
check('GitHub source kind/host are normalized',
  mappedGithub.source.kind === 'github' && mappedGithub.source.host === 'github.com');
check('GitHub repository maps to repository.full_name',
  mappedGithub.repository.full_name === rawRepository.full_name);
check('GitHub repository id maps as a string',
  mappedGithub.repository.id === String(rawRepository.id));
check('GitHub PR maps to pull_request subject',
  mappedGithub.subject.kind === 'pull_request' && mappedGithub.subject.id === String(rawPullRequest.id));
check('GitHub PR number maps to subject.number',
  mappedGithub.subject.number === rawBody.number);
check('GitHub sender maps to actor',
  mappedGithub.actor.id === String(rawSender.id) &&
    mappedGithub.actor.login === rawSender.login &&
    mappedGithub.actor.kind === actorKind(rawSender.type));
check('GitHub action maps to normalized action',
  mappedGithub.action === rawBody.action);
check('GitHub created_at maps to occurred_at',
  mappedGithub.occurred_at === rawPullRequest.created_at);
check('GitHub PR urls map to host links',
  linkByRel(mappedGithub, 'html')?.href === rawPullRequest.html_url &&
    linkByRel(mappedGithub, 'api')?.href === rawPullRequest.url &&
    linkByRel(mappedGithub, 'diff')?.href === rawPullRequest.diff_url &&
    linkByRel(mappedGithub, 'patch')?.href === rawPullRequest.patch_url);
check('GitHub refs and SHA map into normalized payload',
  mappedGithub.payload.base_ref === rawBase.ref &&
    mappedGithub.payload.head_ref === rawHead.ref &&
    mappedGithub.payload.head_sha === rawHead.sha);
check('GitHub idempotency key includes delivery, subject, action, and head revision',
  mappedGithub.idempotency_key === expectedIdempotencyKey);
check('merge policy, when present, rides as structured policy metadata',
  mappedGithub.policy && mappedGithub.policy.merge_policy === 'agent-mergeable');

console.log(`\n${checks} checks, ${failures} failures`);
process.exit(failures > 0 ? 1 : 0);
