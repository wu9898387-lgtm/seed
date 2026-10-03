# ADR-0002: Plugin runtime

- Status: Proposed / Spike
- Date: 2026-10-03
- Scope: untrusted plugin execution

## Context

Seed's product model depends on third-party governance and application plugins,
but Core must not grant those plugins ambient access to private keys, arbitrary
chat history, local files, network sockets, or direct state mutation.

The runtime therefore needs:

- a stable language-neutral boundary;
- deterministic resource limits;
- a narrow host-call surface;
- crash/trap containment;
- explicit capability/permission checks;
- acceptable binary size for the Core budget.

## First candidate: WebAssembly interpreter

The first runtime spike uses **wasmi 2.0.0** with default features disabled and
only module validation enabled.

The host intentionally provides:

- no WASI;
- no filesystem imports;
- no network imports;
- no Seed host capabilities yet;
- no WAT parser;
- no plugin-side start function;
- fuel metering enabled.

The embedded test module only exports add_one(i32) -> i32. Its purpose is to
force real module parsing, validation, instantiation, execution, and fuel
accounting into the linked binary.

## Why this candidate

A WebAssembly boundary is language-neutral and makes it possible to expose only
specific host functions. Wasmi is an interpreter rather than a JIT-heavy engine
and supports constrained/no_std use cases.

This does not mean "WASM is accepted". The decisive question for Seed is
whether the full security boundary plus required host ABI fits the size and
maintenance budget.

## Alternatives

### Native in-process dynamic library

Advantages:
- minimal execution overhead;
- mature native toolchains.

Risks:
- weak isolation from Core memory;
- platform-specific ABI/loader behavior;
- a malicious plugin can compromise the process unless an additional sandbox
  exists.

### Native out-of-process plugin

Advantages:
- OS process isolation;
- plugin crashes can be contained;
- runtime engine may not need to be embedded.

Risks:
- IPC and lifecycle complexity;
- mobile/platform restrictions;
- OS sandbox behavior differs by platform;
- higher memory/startup cost.

### Custom VM or scripting runtime

Advantages:
- potentially very small and purpose-built.

Risks:
- Seed would own parser/VM correctness and security;
- ecosystem and tooling burden;
- easy to accidentally create a weak sandbox.

## Security direction

A future Seed WASM ABI should be deny-by-default. Plugins should receive opaque
handles and narrow host calls rather than direct pointers or global stores.

Fuel limits are only one resource-control primitive. Memory/table limits,
host-call quotas, storage quotas, event-loop scheduling, and permission checks
still need separate design.

## Current evidence

The integrated Linux release binary that exercises Seed's Ed25519 + canonical
Event path and the validated/fuel-metered WASM host measured **1,274,376 bytes
(1,245 KiB)**.

Compared with the canonical Event + crypto binary at **355,624 bytes**, the
embedded runtime path adds approximately **918,752 bytes**. The integrated
binary therefore uses about **60.8%** of the current 2 MiB target, leaving
**822,776 bytes** before that reference ceiling.

This is enough evidence to keep testing the WASM path, but not enough to accept
it: memory limits, host-call permissions, plugin storage, and platform behavior
are still unresolved.

## Evidence required before Accepted

- linked release size for the minimal validated/fuel-metered host;
- malformed module rejection test;
- out-of-fuel trap test;
- memory growth limit test;
- first real capability host call;
- scoped plugin storage proof;
- plugin crash/trap cannot mutate Core state;
- comparison against an out-of-process native prototype.

## Revisit conditions

Reopen the candidate if the embedded runtime consumes too much of the 2 MiB
budget, requires a large unsafe surface, cannot provide predictable resource
limits, or complicates mobile support substantially.
