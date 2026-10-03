#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
TARGET_DIR=${CARGO_TARGET_DIR:-"$ROOT/target"}

cargo build \
  --manifest-path "$ROOT/Cargo.toml" \
  --release \
  --bin seed-core-smoke \
  --bin seed-storage-smoke

CORE_BIN="$TARGET_DIR/release/seed-core-smoke"
STORAGE_BIN="$TARGET_DIR/release/seed-storage-smoke"

for BIN in "$CORE_BIN" "$STORAGE_BIN"; do
  if [ ! -f "$BIN" ]; then
    echo "size-report: binary not found: $BIN" >&2
    exit 1
  fi
done

CORE_BYTES=$(wc -c < "$CORE_BIN" | tr -d ' ')
STORAGE_BYTES=$(wc -c < "$STORAGE_BIN" | tr -d ' ')
STORAGE_DELTA=$((STORAGE_BYTES - CORE_BYTES))

printf 'seed-core-smoke release bytes: %s\n' "$CORE_BYTES"
printf 'seed-core-smoke release KiB: '
awk "BEGIN { printf \"%.1f\\n\", $CORE_BYTES / 1024 }"

printf 'seed-storage-smoke release bytes: %s\n' "$STORAGE_BYTES"
printf 'seed-storage-smoke release KiB: '
awk "BEGIN { printf \"%.1f\\n\", $STORAGE_BYTES / 1024 }"

printf 'persistent-storage delta bytes: %s\n' "$STORAGE_DELTA"
printf 'persistent-storage delta KiB: '
awk "BEGIN { printf \"%.1f\\n\", $STORAGE_DELTA / 1024 }"
