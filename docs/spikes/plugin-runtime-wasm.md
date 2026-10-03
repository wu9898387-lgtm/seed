# Phase 0 Spike — WASM Plugin Runtime

> Status: measured

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

CI proves that a real WebAssembly parser/interpreter can be linked and invoked
under a deterministic execution budget. It also rejects a malformed module and
prevents a valid module from executing when its fuel budget is zero.

It does not yet prove:

- permission isolation;
- memory quotas;
- secure host calls;
- scoped storage;
- plugin packaging/signatures;
- compatibility across runtime upgrades.

## Measurement

GitHub Actions on Ubuntu 24.04 / Rust 1.99.0 measured:

- canonical Event + crypto: **355,624 bytes (348 KiB)**;
- integrated Seed + WASM host: **1,274,376 bytes (1,245 KiB)**;
- runtime increment over Event + crypto: **918,752 bytes**;
- integrated utilization of the 2 MiB reference target: about **60.8%**;
- remaining reference headroom: **822,776 bytes**.

The integrated executable actively exercises both the Seed Event/signature path
and the WASM module path so dead-code elimination cannot remove either side.

This is a first-order linked-size measurement, not a final installed-size or
resident-memory estimate.
