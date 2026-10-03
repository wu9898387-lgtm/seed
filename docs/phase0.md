# Phase 0 — Engineering Spike

> Status: in progress

Phase 0 converts the current architecture documents into measurable engineering
evidence. The goal is not to build the chat UI yet; it is to prove that the
Core boundaries can stay small, testable, and governance-neutral.

## Current spike

This branch introduces the first executable Core kernel with these constraints:

- Rust reference implementation, not a final language lock-in;
- seed-core uses no_std;
- no third-party dependencies;
- unsafe code is forbidden in the initial kernel;
- Identity, Space, Event, Capability, Plugin Manifest, and Transport boundaries
  are represented as small protocol-facing types;
- Owner/Admin/Moderator roles do not exist in Core;
- sensitive actions flow through a CapabilityPolicy decision boundary;
- release size is measured automatically.

## Measurement contract

The initial reference measurement is:

- GitHub Linux x86_64 runner;
- stable Rust toolchain;
- release build;
- size optimization (opt-level = z);
- LTO enabled;
- one codegen unit;
- panic abort;
- symbols stripped;
- binary measured: seed-phase0-core;
- plugins and platform system libraries excluded.

The 2 MiB value remains a project target, not a security override. A future
dependency that is necessary for cryptographic safety may exceed a sub-budget
and must be evaluated explicitly rather than replaced by custom crypto.

## First measured baseline

GitHub Actions on Ubuntu 24.04 with Rust 1.99.0 produced:

- format: pass;
- tests: pass (1 Core unit test);
- clippy with warnings denied: pass;
- stripped release binary: **283,416 bytes (277 KiB)**;
- current 2 MiB reference target: 2,097,152 bytes;
- baseline utilization: about **13.5%** of that target.

This number is intentionally only a kernel baseline. Crypto, canonical
serialization, persistent storage, networking implementation, and plugin
runtime are not included yet and must be measured as separate increments.

## What this spike does not decide

It intentionally does not freeze:

- signature or AEAD algorithms;
- canonical serialization;
- Event ID hashing;
- clocks or distributed ordering;
- database/storage engine;
- async executor;
- TCP vs QUIC;
- NAT traversal;
- relay protocol;
- WASM/native/custom plugin runtime;
- plugin ABI;
- Genesis binary format.

Those require separate evidence and ADRs.

## Phase 0 next sequence

1. Record the first CI binary-size result.
2. Add ADR-0002 for plugin runtime and measure a hello-plugin host.
3. Add ADR-0003 for canonical serialization and a deterministic Genesis vector.
4. Add ADR-0004 for storage with an append-only baseline and SQLite comparison.
5. Add ADR-0005 for transport and run localhost direct + relay simulations.
6. Add mature cryptographic primitives only after their binary-size/security
   tradeoff is measured.
7. Introduce protocol test vectors before Phase 1 Identity/Event work expands.

## Exit signal for this slice

This first slice is useful when:

- CI compiles and tests it cleanly;
- the release-size number is known;
- Core still contains no social-role bypass;
- the next runtime/serialization/storage experiments can attach to these
  boundaries without rewriting the product model.
