# Phase 0 Spike — Canonical Event Bytes

> Status: measured

## Goal

Define one deterministic byte sequence for the first Event signature path
without prematurely selecting the serialization format for every Seed object.

## Invariants

- no allocation is required by the Core encoder;
- integer byte order is explicit;
- field order is fixed;
- payload length is explicit;
- EventId and signature are not part of the signing preimage;
- social roles do not appear in the Event header;
- changing any covered field changes the signed bytes.

## Fixed vector

Input:

- protocol: 0.1;
- kind: message (1);
- space: 0x22 repeated 32 bytes;
- author: 0x33 repeated 32 bytes;
- device: 0x44 repeated 32 bytes;
- payload: UTF-8 bytes for "hello".

Expected canonical length: **119 bytes**.

The Core unit test freezes every field at an exact byte offset, and a second
test freezes an Ed25519 signature over the complete 119-byte output.

## Measurement

GitHub Actions on Ubuntu 24.04 / Rust 1.99.0 measured:

- kernel baseline: **283,416 bytes (277 KiB)**;
- Ed25519 spike: **354,936 bytes (347 KiB)**;
- canonical Event + Ed25519 spike: **355,624 bytes (348 KiB)**;
- canonical Event increment over crypto: **688 bytes**;
- all 5 Core tests, format, and clippy passed.

This is a linked-binary comparison, not exact symbol-level attribution, but it
shows the fixed canonical Event framing itself is currently negligible relative
to the cryptographic dependency.

## Why not call this the final serialization format?

A fixed Event preimage is simple. Genesis is not: it contains plugin lists,
configs, policy metadata, and future extensibility. The project still needs to
compare this approach with RFC 8949 deterministic CBOR and other candidates
before a general wire-format decision is accepted.
