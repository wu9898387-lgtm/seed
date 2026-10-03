# Phase 0 Spike — WASM Plugin Runtime

> Status: running

## Configuration

Candidate runtime: wasmi 2.0.0.

For this spike:

- default crate features are disabled;
- module validation remains enabled;
- WAT parsing is excluded;
- WASI is excluded;
- fuel metering is enabled;
- start functions are disallowed;
- there are no host imports.

This is deliberately more restrictive than a real plugin ABI.

## Embedded module

The test module is a raw WebAssembly binary with one function:

    add_one(i32) -> i32

The test loads and validates the module, instantiates it, calls add_one(41),
requires the result 42, and verifies that execution consumed fuel.

## What this proves

If CI passes, it proves that a real WebAssembly parser/interpreter can be linked
and invoked under a deterministic execution budget.

It does not yet prove:

- permission isolation;
- memory quotas;
- secure host calls;
- scoped storage;
- plugin packaging/signatures;
- compatibility across runtime upgrades.

## Measurement

CI compares the stripped Linux release binary against the existing Phase 0
kernel baseline. This gives a first-order answer to whether an embedded WASM
interpreter is compatible with the 2 MiB ambition.
