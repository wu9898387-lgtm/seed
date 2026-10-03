#!/usr/bin/env bash
set -euo pipefail

cargo build --release -p seed-phase0-core -p seed-phase0-crypto -p seed-phase0-event -p seed-phase0-plugin-wasm -p seed-phase0-storage-append -p seed-phase0-storage-sqlite

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
event_binary="target/release/seed-phase0-event"
plugin_binary="target/release/seed-phase0-plugin-wasm"
append_binary="target/release/seed-phase0-storage-append"
sqlite_binary="target/release/seed-phase0-storage-sqlite"

core_bytes="$(wc -c < "${core_binary}" | tr -d '[:space:]')"
crypto_bytes="$(wc -c < "${crypto_binary}" | tr -d '[:space:]')"
event_bytes="$(wc -c < "${event_binary}" | tr -d '[:space:]')"
plugin_bytes="$(wc -c < "${plugin_binary}" | tr -d '[:space:]')"
append_bytes="$(wc -c < "${append_binary}" | tr -d '[:space:]')"
sqlite_bytes="$(wc -c < "${sqlite_binary}" | tr -d '[:space:]')"
delta="$((crypto_bytes - core_bytes))"
event_delta="$((event_bytes - crypto_bytes))"
plugin_vs_event_delta="$((plugin_bytes - event_bytes))"
plugin_vs_kernel_delta="$((plugin_bytes - core_bytes))"
append_vs_kernel_delta="$((append_bytes - core_bytes))"
sqlite_vs_kernel_delta="$((sqlite_bytes - core_bytes))"

measure "${core_binary}"
measure "${crypto_binary}"
measure "${event_binary}"
measure "${plugin_binary}"
measure "${append_binary}"
measure "${sqlite_binary}"
printf 'crypto-vs-kernel delta: %s bytes\n' "${delta}"
printf 'event-vs-crypto delta: %s bytes\n' "${event_delta}"
printf 'plugin-runtime-vs-event delta: %s bytes\n' "${plugin_vs_event_delta}"
printf 'integrated-vs-kernel delta: %s bytes\n' "${plugin_vs_kernel_delta}"
printf 'append-storage-vs-kernel delta: %s bytes\n' "${append_vs_kernel_delta}"
printf 'system-sqlite-vs-kernel delta: %s bytes\n' "${sqlite_vs_kernel_delta}"
printf 'phase0 reference budget: %s bytes (2 MiB)\n' "${budget}"

if (( plugin_bytes > budget )); then
  printf 'warning: plugin runtime spike exceeds the 2 MiB Core target\n' >&2
else
  printf 'status: plugin runtime spike remains within the current 2 MiB reference target\n'
fi
