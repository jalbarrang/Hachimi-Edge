#!/usr/bin/env bash
# Verify plugin API version and vtable slot count stay in sync.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ABI_VERSION_FILE="$ROOT/crates/hachimi-plugin-abi/src/version.rs"
ABI_TEST_FILE="$ROOT/crates/hachimi-plugin-abi/tests/abi_layout.rs"

API_VERSION="$(grep -E 'pub const API_VERSION' "$ABI_VERSION_FILE" | grep -oE '[0-9]+')"
SLOT_COUNT="$(grep -E 'pub const VTABLE_SLOT_COUNT' "$ABI_VERSION_FILE" | grep -oE '[0-9]+')"
TEST_SLOTS="$(grep -E 'VTABLE_SLOT_COUNT, [0-9]+' "$ABI_TEST_FILE" | grep -oE '[0-9]+$' || true)"

if [[ -z "$API_VERSION" || -z "$SLOT_COUNT" ]]; then
  echo "check-plugin-api: could not read API_VERSION or VTABLE_SLOT_COUNT from version.rs" >&2
  exit 1
fi

echo "API_VERSION=$API_VERSION VTABLE_SLOT_COUNT=$SLOT_COUNT"

cargo test -p hachimi-plugin-abi --quiet

# Host builds the vtable in api.rs — field count must match abi constant.
HOST_FIELDS="$(grep -E '^\s+[a-z_]+,' "$ROOT/src/core/plugin/api.rs" | grep -c ',' || true)"
# Count only inside build_host_vtable struct literal (between fn build_host_vtable and closing brace)
HOST_FIELDS="$(awk '/fn build_host_vtable/,/^}$/ { if ($0 ~ /^        [a-z_]+,/) c++ } END { print c+0 }' "$ROOT/src/core/plugin/api.rs")"

if [[ "$HOST_FIELDS" != "$SLOT_COUNT" ]]; then
  echo "check-plugin-api: host build_host_vtable has $HOST_FIELDS fields, abi expects $SLOT_COUNT" >&2
  exit 1
fi

if [[ -n "$TEST_SLOTS" && "$TEST_SLOTS" != "$SLOT_COUNT" ]]; then
  echo "check-plugin-api: abi_layout test documents $TEST_SLOTS slots, version.rs has $SLOT_COUNT" >&2
  exit 1
fi

echo "check-plugin-api: OK"
