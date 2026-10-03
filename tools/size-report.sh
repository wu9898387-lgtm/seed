#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
TARGET_DIR=${CARGO_TARGET_DIR:-"$ROOT/target"}

cargo build --manifest-path "$ROOT/Cargo.toml" --release \
  --bin seed-core-smoke \
  --bin seed-storage-smoke

report_binary() {
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

report_binary seed-core-smoke
report_binary seed-storage-smoke
