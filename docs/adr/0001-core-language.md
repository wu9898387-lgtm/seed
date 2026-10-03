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
