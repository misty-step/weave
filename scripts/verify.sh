#!/usr/bin/env bash
# Verify Weave contract schemas and fixtures.
#
# Validates that every fixture under docs/fixtures/contracts/ is accepted or
# rejected by its schema as expected. Fixtures whose names contain an
# invalid-marker token (missing-schema-version, unknown-major) must be rejected;
# all others must pass.
#
# Also checks that no fixture carries secrets, local paths, or tailnet
# hostnames.
#
# Usage: ./scripts/verify.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TMPDIR_BASE="${TMPDIR:-/tmp}"
CACHE_DIR="$TMPDIR_BASE/weave-verify-cache"

# --- JSON well-formedness -----------------------------------------------------
echo "==> Checking JSON well-formedness"
while IFS= read -r f; do
  if ! jq empty "$f" 2>/dev/null; then
    echo "  FAIL  invalid JSON: $f"
    exit 1
  fi
done < <(find "$ROOT/docs/schemas" "$ROOT/docs/fixtures" -type f -name '*.json' | sort)
echo "  OK    all JSON files are well-formed"

# --- Secret / instance-data scan ---------------------------------------------
echo "==> Scanning fixtures for forbidden content"
FORBIDDEN_PATTERNS=(
  'tailnet'
  'ts\.net'
  '/Users/'
  '/home/'
  'password'
  'api_key'
  'secret'
  'Bearer '
)

FOUND=0
while IFS= read -r f; do
  for pattern in "${FORBIDDEN_PATTERNS[@]}"; do
    if grep -qiE "$pattern" "$f"; then
      echo "  FAIL  $f matches forbidden pattern '$pattern'"
      FOUND=1
    fi
  done
done < <(find "$ROOT/docs/fixtures" -type f -name '*.json' | sort)
if [ "$FOUND" -ne 0 ]; then
  exit 1
fi
echo "  OK    no forbidden content found"

# --- Schema validation -------------------------------------------------------
echo "==> Validating fixtures against schemas"

mkdir -p "$CACHE_DIR"
if [ ! -d "$CACHE_DIR/node_modules/ajv" ]; then
  echo "  Installing ajv (cached at $CACHE_DIR)..."
  (cd "$CACHE_DIR" && npm init -y --silent >/dev/null 2>&1 && npm install --silent ajv@8.17.1 ajv-formats@3.0.1 >/dev/null 2>&1)
fi

NODE_PATH="$CACHE_DIR/node_modules" node "$ROOT/scripts/validate-contracts.cjs" "$ROOT"

# --- Consumer conformance kit ------------------------------------------------
echo "==> Running consumer conformance kit"
NODE_PATH="$CACHE_DIR/node_modules" node "$ROOT/scripts/consumer-conformance-kit.cjs" "$ROOT"

# --- Cross-repo thread replay -------------------------------------------------
echo "==> Replaying weave-906 cross-repo thread"
NODE_PATH="$CACHE_DIR/node_modules" node "$ROOT/scripts/thread-replay.cjs" "$ROOT"

# --- Rust app checks ----------------------------------------------------------
if [ -f "$ROOT/Cargo.toml" ]; then
  echo "==> Checking Rust workspace"
  (cd "$ROOT" && cargo fmt --all -- --check)
  (cd "$ROOT" && cargo clippy --workspace --all-targets -- -D warnings)
  (cd "$ROOT" && cargo test --workspace)
fi
