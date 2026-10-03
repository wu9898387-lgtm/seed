#!/usr/bin/env bash
set -euo pipefail

cargo build --release -p seed-phase0-core -p seed-phase0-crypto

budget="$((2 * 1024 * 1024))"

measure() {
  local binary="$1"
  local bytes
  local kib
  bytes="$(wc -c < "${binary}" | tr -d '[:space:]')"
  kib="$(( (bytes + 1023) / 1024 ))"
  printf '%s: %s bytes (%s KiB)\n' "${binary}" "${bytes}" "${kib}"
}

core_binary="target/release/seed-phase0-core"
crypto_binary="target/release/seed-phase0-crypto"

core_bytes="$(wc -c < "${core_binary}" | tr -d '[:space:]')"
crypto_bytes="$(wc -c < "${crypto_binary}" | tr -d '[:space:]')"
delta="$((crypto_bytes - core_bytes))"

measure "${core_binary}"
measure "${crypto_binary}"
printf 'crypto-vs-kernel delta: %s bytes\n' "${delta}"
printf 'phase0 reference budget: %s bytes (2 MiB)\n' "${budget}"

if (( crypto_bytes > budget )); then
  printf 'warning: crypto spike exceeds the 2 MiB Core target\n' >&2
else
  printf 'status: crypto spike remains within the current 2 MiB reference target\n'
fi
