# ADR-0001: Core language spike uses Rust

- Status: Provisional
- Date: 2026-10-03

## Context

Seed needs a small, memory-safe, cross-platform core with explicit FFI boundaries
and enough systems/networking support to implement Identity, Event, Transport,
Storage, Capability, and Plugin Runtime primitives.

The implementation plan requires comparing Rust, C++, and C before the language
choice is treated as frozen. Binary size is a hard engineering constraint, but
security primitives must not be weakened to hit the size target.

## Decision

Use Rust for the first Phase-0 core-kernel spike.

This is a **provisional engineering decision**, not a permanent protocol
commitment. The first crate deliberately has no third-party dependencies and
forbids `unsafe` code so that baseline binary size and architecture can be
measured before crypto, storage, networking, or plugin-runtime dependencies are
introduced.

## Initial measurement

GitHub Actions baseline on 2026-10-03:

- host: `x86_64-unknown-linux-gnu`;
- compiler: `rustc 1.99.0`;
- release profile: `opt-level=z`, fat LTO, one codegen unit, `panic=abort`,
  stripped symbols;
- smoke binary: **282,648 bytes (276.0 KiB)**;
- unit tests: **6 passed, 0 failed**.

This number is the dependency-free lower-bound baseline. Persistent storage,
transport, and plugin runtime are still excluded.

## Phase-1 signing measurement

The same CI profile after adding the production signature path measured:

- Ed25519 signing and strict verification through ed25519-dalek 3.0.0;
- SHA-256 Identity / Device / Event IDs;
- OS entropy and secure temporary-secret zeroization;
- Root -> Device authorization and Device-signed Event verification;
- smoke binary: **376,320 bytes (367.5 KiB)**;
- delta from the dependency-free baseline: **93,672 bytes (about 91.5 KiB)**;
- tests: **13 unit tests + 1 protocol-vector integration test passed**;
- format, cargo check, Clippy with warnings denied, and tests all passed.

This is evidence that the signing identity layer is currently compatible with
Seed's 2 MB Core budget. It is not evidence yet for storage, transport, or the
plugin runtime, which remain the largest unknowns.

## Genesis / canonical encoding measurement

After adding the canonical Genesis state machine and forcing its encode, decode,
signature verification, plugin digest pinning, GenesisId derivation, and SpaceId
derivation into the release smoke path:

- smoke binary: **389,816 bytes (380.7 KiB)**;
- delta from the signing/identity measurement: **13,496 bytes (about 13.2 KiB)**;
- total delta from the dependency-free baseline: **107,168 bytes (about 104.7 KiB)**;
- tests: **21 unit tests + 1 protocol-vector integration test passed**;
- format, cargo check, Clippy with warnings denied, vector smoke, and release-size
  gate all passed.

At this point Identity + Device authorization + Event signing + canonical Genesis
still occupy well under one quarter of the 2 MB target. Persistent storage,
transport, and plugin runtime remain the meaningful size-risk areas.

## Consequences

Positive:

- memory-safe default for the protocol kernel;
- strong type system for Identity/Device/Space/Event IDs;
- explicit traits for crypto, storage, transport, and plugin boundaries;
- straightforward unit testing and multi-node simulation later.

Costs / risks:

- Rust standard-library and panic/runtime overhead may challenge the 2 MB target;
- async/network and WASM runtimes can add significant binary size;
- FFI and platform integration still require careful boundary design.

## Revisit conditions

Revisit this ADR after Phase-0 measurements exist for:

1. stripped release baseline;
2. production crypto backend;
3. persistent storage adapter;
4. transport spike;
5. plugin-runtime spike.

If the combined evidence shows Rust cannot meet Seed's size/security/portability
constraints without unreasonable complexity, compare the same vertical slice in
C++ and C before freezing the language.
