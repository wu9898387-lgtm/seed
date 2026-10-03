#!/usr/bin/env bash
set -euo pipefail

cargo build --release -p seed-phase0-core

binary="target/release/seed-phase0-core"
if [[ "${OS:-}" == "Windows_NT" ]]; then
  binary="${binary}.exe"
fi

bytes="$(wc -c < "${binary}" | tr -d '[:space:]')"
kib="$(( (bytes + 1023) / 1024 ))"
budget="$((2 * 1024 * 1024))"

printf 'seed-phase0-core: %s bytes (%s KiB)\n' "${bytes}" "${kib}"
printf 'phase0 reference budget: %s bytes (2 MiB)\n' "${budget}"

if (( bytes > budget )); then
  printf 'warning: phase0 spike exceeds the 2 MiB Core target\n' >&2
else
  printf 'status: within the current 2 MiB reference target\n'
fi
