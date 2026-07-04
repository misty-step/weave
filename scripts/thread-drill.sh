#!/usr/bin/env bash
# Run the weave-906 incident -> Powder -> Landmark release thread drill.
#
# Required for live Powder hops:
#   POWDER_URL
#   POWDER_API_KEY or POWDER_API_KEY_REF (defaults to op://Agents/POWDER_API_KEY__bridge/credential)
#
# Optional:
#   THREAD_DRILL_WEBHOOK_URL        URL Powder can call for signed webhook capture.
#   RELEASE_FEED_URL                apps/release-events /v1/events endpoint.
#   LANDMARK_WEBHOOK_SECRET         HMAC key for release feed POST.
#   LANDMARK_WEBHOOK_SECRET_REF     1Password ref for LANDMARK_WEBHOOK_SECRET.
#   RELEASE_EVENTS_READER_TOKEN     Bearer for release feed GET.
#   RELEASE_EVENTS_READER_TOKEN_REF 1Password ref for RELEASE_EVENTS_READER_TOKEN.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
POWDER_API_KEY_REF="${POWDER_API_KEY_REF:-op://Agents/POWDER_API_KEY__bridge/credential}"
RELEASE_EVENT_FIXTURE="$ROOT/docs/fixtures/contracts/landmark.release-kit.v1.thread-release.json"
TMPDIR_BASE="${TMPDIR:-/tmp}"
NODE_CACHE_DIR="$TMPDIR_BASE/weave-verify-cache"

PASS_COUNT=0
WARN_COUNT=0
FAIL_COUNT=0

pass() {
  PASS_COUNT=$((PASS_COUNT + 1))
  printf 'PASS  %s\n' "$1"
}

warn() {
  WARN_COUNT=$((WARN_COUNT + 1))
  printf 'WARN  %s\n' "$1"
}

fail() {
  FAIL_COUNT=$((FAIL_COUNT + 1))
  printf 'FAIL  %s\n' "$1"
}

stub() {
  printf 'STUB  %s\n' "$1"
}

read_secret() {
  local value="${1:-}"
  local ref="${2:-}"
  if [ -n "$value" ]; then
    printf '%s' "$value"
    return 0
  fi
  if [ -n "$ref" ] && command -v op >/dev/null 2>&1; then
    op read "$ref" --no-newline 2>/dev/null || true
  fi
}

http_status() {
  local out="$1"
  shift
  local status
  status="$(curl -sS -o "$out" -w '%{http_code}' "$@" 2>/tmp/weave-thread-drill-curl.err || true)"
  if [ -z "$status" ]; then
    status="000"
  fi
  printf '%s' "$status"
}

json_post_powder() {
  local path="$1"
  local body="$2"
  local out="$3"
  http_status "$out" \
    -X POST "$POWDER_URL$path" \
    -H "Authorization: Bearer $POWDER_KEY" \
    -H 'Content-Type: application/json' \
    --data "$body"
}

printf '==> weave-906 live thread drill\n'
printf '    %s\n' "bb dispatch hop is simulated by direct Powder lifecycle calls until powder-008 lands."
printf '    %s\n' "Canary is fixture-derived from misty-step/canary#230; this script does not stand up Canary."
printf '\n'

if [ ! -d "$NODE_CACHE_DIR/node_modules/ajv" ]; then
  mkdir -p "$NODE_CACHE_DIR"
  (cd "$NODE_CACHE_DIR" && npm init -y --silent >/dev/null 2>&1 && npm install --silent ajv@8.17.1 ajv-formats@3.0.1 >/dev/null 2>&1)
fi

if NODE_PATH="${NODE_PATH:-$NODE_CACHE_DIR/node_modules}" node "$ROOT/scripts/thread-replay.cjs" "$ROOT" >/tmp/weave-thread-replay.out 2>&1; then
  pass "fixture replay validates all schemas"
else
  fail "fixture replay failed"
  cat /tmp/weave-thread-replay.out
fi

stub "Canary incident source is replayed from canary#230 conformance receipt"
stub "BB dispatch is simulated by direct Powder card lifecycle calls"

if [ -z "${POWDER_URL:-}" ]; then
  warn "POWDER_URL not set; skipping live Powder hops"
else
  POWDER_KEY="$(read_secret "${POWDER_API_KEY:-}" "$POWDER_API_KEY_REF")"
  if [ -z "$POWDER_KEY" ]; then
    fail "Powder key unavailable; set POWDER_API_KEY or POWDER_API_KEY_REF"
  else
    health_out="$(mktemp)"
    health_status="$(http_status "$health_out" "$POWDER_URL/healthz")"
    if [ "$health_status" = "200" ]; then
      pass "Powder healthz reachable"
    else
      fail "Powder healthz returned HTTP $health_status"
    fi

    events_out="$(mktemp)"
    events_status="$(http_status "$events_out" \
      "$POWDER_URL/api/v1/events/subscriptions" \
      -H "Authorization: Bearer $POWDER_KEY")"
    SUBSCRIPTION_ID=""
    if [ "$events_status" = "200" ]; then
      pass "Powder event subscriptions endpoint reachable"
      if [ -n "${THREAD_DRILL_WEBHOOK_URL:-}" ]; then
        sub_body="$(jq -cn --arg url "$THREAD_DRILL_WEBHOOK_URL" '{url:$url,event_filter:["card-created","moved-to-ready","completed"]}')"
        sub_out="$(mktemp)"
        sub_status="$(json_post_powder "/api/v1/events/subscriptions" "$sub_body" "$sub_out")"
        if [ "$sub_status" = "200" ]; then
          SUBSCRIPTION_ID="$(jq -r '.subscription.id // empty' "$sub_out")"
          pass "Powder webhook subscription created for thread drill"
        else
          warn "Powder webhook subscription create returned HTTP $sub_status"
        fi
      else
        warn "THREAD_DRILL_WEBHOOK_URL not set; signed webhook capture skipped"
      fi
    elif [ "$events_status" = "404" ]; then
      warn "Powder event subscription endpoint returned 404; outbound event face is not live on this instance"
      stub "Powder card_event webhook capture uses pinned producer fixtures until the live instance exposes /api/v1/events/subscriptions"
    else
      warn "Powder event subscription probe returned HTTP $events_status"
    fi

    card_id="weave-thread-fixture-$(date +%Y%m%d%H%M%S)-$$"
    create_body="$(jq -cn --arg id "$card_id" '{id:$id,title:"Weave thread drill fixture",body:"Throwaway card for weave thread drill.",acceptance:["complete direct Powder lifecycle"],status:"backlog",priority:"p2"}')"
    create_out="$(mktemp)"
    create_status="$(json_post_powder "/api/v1/cards" "$create_body" "$create_out")"
    if [ "$create_status" = "200" ]; then
      pass "Powder card created: $card_id"
      ready_out="$(mktemp)"
      ready_status="$(json_post_powder "/api/v1/cards/$card_id/status" '{"status":"ready"}' "$ready_out")"
      if [ "$ready_status" = "200" ]; then
        pass "Powder card moved to ready"
      else
        fail "Powder move-to-ready returned HTTP $ready_status"
      fi
      complete_out="$(mktemp)"
      complete_status="$(json_post_powder "/api/v1/cards/$card_id/complete" '{"proof":"weave thread drill completed"}' "$complete_out")"
      if [ "$complete_status" = "200" ]; then
        pass "Powder card completed"
      else
        fail "Powder complete returned HTTP $complete_status"
      fi
    else
      fail "Powder card create returned HTTP $create_status"
    fi

    if [ -n "$SUBSCRIPTION_ID" ]; then
      sleep 8
      disable_out="$(mktemp)"
      disable_status="$(http_status "$disable_out" \
        -X POST "$POWDER_URL/api/v1/events/subscriptions/$SUBSCRIPTION_ID/disable" \
        -H "Authorization: Bearer $POWDER_KEY")"
      if [ "$disable_status" = "200" ]; then
        pass "Powder webhook subscription disabled"
      else
        warn "Powder webhook subscription disable returned HTTP $disable_status"
      fi
    fi
  fi
fi

if [ -z "${RELEASE_FEED_URL:-}" ]; then
  warn "RELEASE_FEED_URL not set; skipping live release-events POST"
else
  release_repo="$(jq -r '.release.repository' "$RELEASE_EVENT_FIXTURE")"
  release_version="$(jq -r '.release.tag // .release.version' "$RELEASE_EVENT_FIXTURE")"
  release_url="$(jq -r '.release.release_url' "$RELEASE_EVENT_FIXTURE")"
  release_feed_posted=0
  feed_secret="$(read_secret "${LANDMARK_WEBHOOK_SECRET:-}" "${LANDMARK_WEBHOOK_SECRET_REF:-}")"
  if [ -z "$feed_secret" ]; then
    warn "LANDMARK_WEBHOOK_SECRET not available; skipping live release-events POST"
  else
    signature="$(LANDMARK_WEBHOOK_SECRET="$feed_secret" node -e 'const fs=require("fs"); const crypto=require("crypto"); const body=fs.readFileSync(process.argv[1]); const secret=process.env.LANDMARK_WEBHOOK_SECRET; process.stdout.write("sha256="+crypto.createHmac("sha256", secret).update(body).digest("hex"));' "$RELEASE_EVENT_FIXTURE")"
    post_out="$(mktemp)"
    post_status="$(http_status "$post_out" \
      -X POST "$RELEASE_FEED_URL" \
      -H "X-Signature-256: $signature" \
      -H 'Content-Type: application/json' \
      --data-binary "@$RELEASE_EVENT_FIXTURE")"
    if [ "$post_status" = "201" ] || [ "$post_status" = "200" ]; then
      pass "release-events receiver accepted Landmark release kit"
      if jq -e \
        --arg repo "$release_repo" \
        --arg version "$release_version" \
        --arg url "$release_url" \
        '.schema_version == "weave.release_feed_row.v1"
          and .kind == "landmark_release_kit"
          and .repository == $repo
          and .version == $version
          and .release_url == $url
          and .payload.schema_version == "landmark.release-kit.v1"
          and any(.payload.producer_contracts[]?; .id == "release-feed-receiver")' \
        "$post_out" >/dev/null; then
        pass "release-events POST returned a versioned feed row with full release-kit payload"
        release_feed_posted=1
      else
        fail "release-events POST response did not match weave.release_feed_row.v1"
      fi
    else
      fail "release-events receiver returned HTTP $post_status"
    fi
  fi
fi

if [ -n "${RELEASE_FEED_URL:-}" ]; then
  reader_token="$(read_secret "${RELEASE_EVENTS_READER_TOKEN:-}" "${RELEASE_EVENTS_READER_TOKEN_REF:-}")"
  if [ -n "$reader_token" ]; then
    feed_base="${RELEASE_FEED_URL%/v1/events}"
    read_out="$(mktemp)"
    read_status="$(http_status "$read_out" \
      "$feed_base/v1/events" \
      -H "Authorization: Bearer $reader_token")"
    if [ "$read_status" = "200" ]; then
      if [ "${release_feed_posted:-0}" -eq 1 ]; then
        if jq -e \
          --arg repo "$release_repo" \
          --arg version "$release_version" \
          --arg url "$release_url" \
          '.events // [] | any(.[]; .schema_version == "weave.release_feed_row.v1"
            and .kind == "landmark_release_kit"
            and .repository == $repo
            and .version == $version
            and .release_url == $url
            and .payload.schema_version == "landmark.release-kit.v1"
            and any(.payload.producer_contracts[]?; .id == "release-feed-receiver"))' \
          "$read_out" >/dev/null; then
          pass "release-events feed readback includes the posted versioned row"
        else
          fail "release-events feed readback did not include the posted versioned row"
        fi
      else
        warn "release-events feed readback reachable, but no POST was verified in this run"
      fi
    else
      fail "release-events feed readback returned HTTP $read_status"
    fi
  else
    warn "RELEASE_EVENTS_READER_TOKEN not available; skipping release-events readback"
  fi
fi

printf '\nVerdict: %s pass, %s warn, %s fail\n' "$PASS_COUNT" "$WARN_COUNT" "$FAIL_COUNT"
if [ "$FAIL_COUNT" -gt 0 ]; then
  exit 1
fi
