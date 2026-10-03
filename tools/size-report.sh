#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
TARGET_DIR=${CARGO_TARGET_DIR:-"$ROOT/target"}

cargo build --manifest-path "$ROOT/Cargo.toml" --release --bin seed-core-smoke
BIN="$TARGET_DIR/release/seed-core-smoke"

if [ ! -f "$BIN" ]; then
  echo "size-report: binary not found: $BIN" >&2
  exit 1
fi

BYTES=$(wc -c < "$BIN" | tr -d ' ')
printf 'seed-core-smoke release bytes: %s\n' "$BYTES"
printf 'seed-core-smoke release KiB: '
awk "BEGIN { printf \"%.1f\\n\", $BYTES / 1024 }"
