#!/usr/bin/env bash
set -euo pipefail

budget_bytes="${SEED_SIZE_BUDGET_BYTES:-2097152}"
binary="target/release/seed-size-spike"

cargo build --release -p seed-size-spike

if [[ ! -f "$binary" ]]; then
  echo "size-report: binary not found: $binary" >&2
  exit 1
fi

bytes="$(wc -c < "$binary" | tr -d '[:space:]')"

echo "Seed Phase 0 size report"
echo "binary=$binary"
echo "bytes=$bytes"
echo "budget_bytes=$budget_bytes"

if command -v size >/dev/null 2>&1; then
  echo
  echo "sections:"
  size "$binary" || true
fi

if command -v ldd >/dev/null 2>&1; then
  echo
  echo "dynamic_dependencies:"
  ldd "$binary" || true
fi

if (( bytes > budget_bytes )); then
  message="baseline exceeds current 2 MiB target: $bytes > $budget_bytes"
  if [[ "${GITHUB_ACTIONS:-}" == "true" ]]; then
    echo "::warning::$message"
  else
    echo "WARNING: $message" >&2
  fi

  if [[ "${SEED_SIZE_ENFORCE:-0}" == "1" ]]; then
    exit 2
  fi
fi
