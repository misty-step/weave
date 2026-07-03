# Release Event Receiver

Weave owns the public receiver for Landmark release events. The receiver is a
separate scale-to-zero Fly app (`weave-release-events`), not a Bastion service.
It accepts release webhooks from GitHub Actions and stores them in append-only
JSONL on the Fly volume so the Bridge feed can read them later.

## HTTP Contract

- `GET /healthz` is unauthenticated and returns `ok`.
- `POST /v1/events` accepts JSON and requires Landmark's existing HMAC
  signature header: `X-Signature-256: sha256=<hex-hmac>`.
- The HMAC uses SHA-256 over the exact raw request body with the shared
  `webhook-secret`, matching Landmark's `notify-webhook` implementation.
- `GET /v1/events?since=<rfc3339>` requires `Authorization: Bearer <reader token>`.
  When `since` is present, only events received after that server timestamp are
  returned.

The app accepts two event shapes:

- Plain Landmark webhook payloads with non-empty `version`, `release_url`,
  `notes`, and `repository` string fields.
- Landmark release-kit payloads detected by `schema_version:
  "landmark.release-kit.v1"`, `kind: "landmark.release-kit"`, or the release-kit
  object shape (`release` object plus `producer_contracts` array). The receiver
  indexes `release.tag` or `release.version`, `release.release_url`, and
  `release.repository` or `product.repository`.

Each stored line is a JSON object with:

- `received_at` — server timestamp.
- `kind` — `landmark_webhook` or `landmark_release_kit`.
- `repository`, `version`, `release_url` — indexed feed fields.
- `payload` — original JSON payload.

## Runtime

Required secrets:

- `LANDMARK_WEBHOOK_SECRET` — HMAC key for `POST /v1/events`.
- `RELEASE_EVENTS_READER_TOKEN` — bearer token for `GET /v1/events`.

Storage:

- `RELEASE_EVENTS_ROOT` defaults to `/data/events`.
- Events append to `/data/events/events.jsonl`.
- The Fly volume is mounted at `/data`; redeploys must not replace it.

Deploy from the repo root:

```sh
flyctl deploy --config fly.toml --app weave-release-events --remote-only
```
