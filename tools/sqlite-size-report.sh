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
FILE_BIN="$TARGET_DIR/release/seed-storage-smoke"
CORE_BYTES=$(wc -c < "$CORE_BIN" | tr -d ' ')
FILE_BYTES=$(wc -c < "$FILE_BIN" | tr -d ' ')

printf 'seed-core-smoke release bytes: %s\n' "$CORE_BYTES"
printf 'seed-storage-smoke release bytes: %s\n' "$FILE_BYTES"
printf 'append-file delta bytes: %s\n' "$((FILE_BYTES - CORE_BYTES))"

printf '\n-- SQLite system-link candidate --\n'
if cargo build \
  --manifest-path "$ROOT/Cargo.toml" \
  --release \
  --features sqlite-storage \
  --bin seed-sqlite-storage-smoke
then
  SQLITE_BIN="$TARGET_DIR/release/seed-sqlite-storage-smoke"
  SYSTEM_BYTES=$(wc -c < "$SQLITE_BIN" | tr -d ' ')
  printf 'sqlite-system release bytes: %s\n' "$SYSTEM_BYTES"
  printf 'sqlite-system release KiB: '
  awk "BEGIN { printf \"%.1f\\n\", $SYSTEM_BYTES / 1024 }"
  printf 'sqlite-system delta vs core bytes: %s\n' "$((SYSTEM_BYTES - CORE_BYTES))"
  if command -v ldd >/dev/null 2>&1; then
    printf 'sqlite-system dynamic dependency:\n'
    ldd "$SQLITE_BIN" | grep -E 'sqlite|not found' || true
  fi
else
  printf 'sqlite-system build: unavailable on this runner\n'
fi

printf '\n-- SQLite bundled candidate --\n'
cargo build \
  --manifest-path "$ROOT/Cargo.toml" \
  --release \
  --features sqlite-storage-bundled \
  --bin seed-sqlite-storage-smoke

SQLITE_BIN="$TARGET_DIR/release/seed-sqlite-storage-smoke"
BUNDLED_BYTES=$(wc -c < "$SQLITE_BIN" | tr -d ' ')
printf 'sqlite-bundled release bytes: %s\n' "$BUNDLED_BYTES"
printf 'sqlite-bundled release KiB: '
awk "BEGIN { printf \"%.1f\\n\", $BUNDLED_BYTES / 1024 }"
printf 'sqlite-bundled delta vs core bytes: %s\n' "$((BUNDLED_BYTES - CORE_BYTES))"
printf 'sqlite-bundled delta vs append-file bytes: %s\n' "$((BUNDLED_BYTES - FILE_BYTES))"
