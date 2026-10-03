#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
TARGET_DIR=${CARGO_TARGET_DIR:-"$ROOT/target"}

cargo build --manifest-path "$ROOT/Cargo.toml" --release \
  --bin seed-core-smoke \
  --bin seed-storage-smoke

report_size() {
  NAME=$1
  BIN="$TARGET_DIR/release/$NAME"

  if [ ! -f "$BIN" ]; then
    echo "size-report: binary not found: $BIN" >&2
    exit 1
  fi

  BYTES=$(wc -c < "$BIN" | tr -d ' ')
  printf '%s release bytes: %s\n' "$NAME" "$BYTES"
  printf '%s release KiB: ' "$NAME"
  awk "BEGIN { printf \"%.1f\\n\", $BYTES / 1024 }"
}

report_size seed-core-smoke
report_size seed-storage-smoke

CORE_BYTES=$(wc -c < "$TARGET_DIR/release/seed-core-smoke" | tr -d ' ')
STORAGE_BYTES=$(wc -c < "$TARGET_DIR/release/seed-storage-smoke" | tr -d ' ')
DELTA=$((STORAGE_BYTES - CORE_BYTES))
printf 'seed-storage-smoke delta bytes: %s\n' "$DELTA"
printf 'seed-storage-smoke delta KiB: '
awk "BEGIN { printf \"%.1f\\n\", $DELTA / 1024 }"
