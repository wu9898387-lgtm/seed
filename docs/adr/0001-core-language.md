# ADR-0001: Core language Phase 0 reference spike

- Status: Proposed / Spike
- Date: 2026-10-03
- Scope: Seed Core reference implementation only

## Context

Seed targets a very small Core Runtime while also requiring strong isolation,
cryptographic correctness, protocol stability, and a plugin boundary. The
implementation roadmap requires Rust, C++, and C to be considered before the
language choice is frozen.

The main risk is optimizing only for empty-binary size and then paying for that
choice with memory-safety defects, dependency duplication, unstable plugin
interfaces, or a much larger networking/runtime stack later.

## Decision

Use Rust as the first Phase 0 reference spike, without freezing Rust as the
permanent Core language.

The first Rust spike must:

- keep seed-core free of third-party dependencies;
- compile the Core library as no_std where practical;
- forbid unsafe code in the initial kernel;
- separate protocol types from social/governance roles;
- measure a stripped release binary on CI;
- add dependencies only when a later spike records their size and security
  cost.

This is an experimental implementation choice, not a protocol requirement.
Wire formats, storage formats, plugin ABI, and transport remain language
neutral.

## Alternatives

### C

Strengths:

- very small runtime assumptions;
- predictable FFI;
- direct control over allocation and binary layout.

Costs/risks:

- memory safety must be enforced by review, testing, sanitizers, and coding
  discipline rather than the type system;
- larger security burden around untrusted network/plugin inputs;
- abstractions for ownership and concurrency are easier to misuse.

### C++

Strengths:

- native performance and broad systems ecosystem;
- RAII and stronger abstraction tools than C;
- mature networking and platform integration options.

Costs/risks:

- language/runtime surface is large;
- ABI compatibility across compilers/platforms is difficult;
- binary-size and template/dependency growth needs close measurement;
- memory safety remains substantially manual.

### Rust

Strengths:

- strong memory-safety model without a tracing GC;
- explicit ownership across identity/event/storage/network boundaries;
- good fit for a small, testable native Core;
- C ABI/FFI remains available when platform adapters require it.

Costs/risks:

- standard-library and dependency choices can inflate binaries;
- async/network stacks can become heavy;
- build/toolchain complexity is higher than plain C;
- a WASM plugin runtime may dominate size regardless of Core language.

## Consequences

The repository gains an executable Rust baseline early, so the 2 MiB target can
be measured instead of debated abstractly. No protocol document may assume
Rust-specific layout or serialization.

C/C++ comparison spikes remain valid and should be run if Rust size or runtime
constraints become problematic.

## Evidence required before Accepted

- CI build/test/lint passes;
- stripped release size is recorded;
- crypto + serialization + storage dependency spike is measured;
- plugin runtime spike is measured separately;
- at least one C or C++ baseline is measured if the Rust path approaches the
  Core size ceiling.

## Revisit conditions

Reopen this ADR if any of the following becomes true:

- the minimal Rust Core binary consumes an unexpectedly large fraction of the
  2 MiB target;
- required crypto/storage/network dependencies make the target structurally
  unrealistic;
- plugin sandbox integration is materially simpler/safer in another runtime
  architecture;
- a supported target platform cannot use the Rust toolchain reliably;
- FFI/ABI requirements force unstable or unsafe boundaries through most Core
  modules.
