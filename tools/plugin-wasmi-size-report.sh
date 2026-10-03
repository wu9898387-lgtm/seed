#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
TARGET_DIR=${CARGO_TARGET_DIR:-"$ROOT/target"}
BUDGET=$((2 * 1024 * 1024))

cargo build \
  --manifest-path "$ROOT/Cargo.toml" \
  --release \
  --bin seed-core-smoke

CORE_BIN="$TARGET_DIR/release/seed-core-smoke"
CORE_BYTES=$(wc -c < "$CORE_BIN" | tr -d ' ')

cargo build \
  --manifest-path "$ROOT/Cargo.toml" \
  --release \
  --features plugin-wasmi \
  --bin seed-plugin-wasmi-smoke

PLUGIN_BIN="$TARGET_DIR/release/seed-plugin-wasmi-smoke"
PLUGIN_BYTES=$(wc -c < "$PLUGIN_BIN" | tr -d ' ')
DELTA=$((PLUGIN_BYTES - CORE_BYTES))
HEADROOM=$((BUDGET - PLUGIN_BYTES))

printf 'seed-core-smoke release bytes: %s\n' "$CORE_BYTES"
printf 'seed-core-smoke release KiB: '
awk "BEGIN { printf \"%.1f\\n\", $CORE_BYTES / 1024 }"

printf 'seed-plugin-wasmi-smoke release bytes: %s\n' "$PLUGIN_BYTES"
printf 'seed-plugin-wasmi-smoke release KiB: '
awk "BEGIN { printf \"%.1f\\n\", $PLUGIN_BYTES / 1024 }"

printf 'wasmi-runtime delta vs core bytes: %s\n' "$DELTA"
printf 'wasmi-runtime delta vs core KiB: '
awk "BEGIN { printf \"%.1f\\n\", $DELTA / 1024 }"

printf '2 MiB reference headroom bytes: %s\n' "$HEADROOM"

if [ "$PLUGIN_BYTES" -gt "$BUDGET" ]; then
  echo "warning: integrated Wasmi plugin smoke exceeds 2 MiB reference target" >&2
else
  echo "status: integrated Wasmi plugin smoke remains within 2 MiB reference target"
fi
