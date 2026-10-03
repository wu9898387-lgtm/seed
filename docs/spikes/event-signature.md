# Phase 0 Spike — Event Signature

> Status: running

## Purpose

Measure the size and API cost of adding a mature event-signature primitive
without yet defining the full Identity, Event canonical encoding, or session
handshake.

## Candidate

This spike uses Ed25519 through ed25519-dalek 3.0.0 with default features
disabled and zeroize enabled.

The wrapper intentionally exposes only:

- construction from externally managed 32-byte secret material;
- public verifying-key bytes;
- detached signatures;
- strict signature verification.

It does not yet expose random key generation, private-key export, PKCS#8,
batch verification, prehash mode, or a general cryptographic toolbox.

## Security boundary

The spike does not invent a new signature algorithm. Key generation and secure
key storage remain separate adapters because desktop/mobile platforms may use
OS keystores or hardware-backed keys.

A fixed test secret exists only to generate a deterministic protocol vector. It
must never be reused as a real identity key.

## Measurement

CI builds both:

- seed-phase0-core — kernel baseline;
- seed-phase0-crypto — the same workspace with the Ed25519 path exercised.

The binary delta is useful as a rough linked-size signal, not as an exact
per-crate accounting method.

## Next evidence

After CI passes:

1. record the public key and signature vector;
2. record release binary size and delta;
3. freeze the vector as a regression test;
4. only then connect signatures to canonical Event bytes.
