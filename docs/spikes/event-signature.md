# Phase 0 Spike — Event Signature

> Status: measured

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

GitHub Actions on Ubuntu 24.04 / Rust 1.99.0 measured:

- seed-phase0-core: **283,416 bytes (277 KiB)**;
- seed-phase0-crypto: **354,936 bytes (347 KiB)**;
- linked binary delta: **71,520 bytes**;
- crypto spike utilization of the 2 MiB target: about **16.9%**.

The binary delta is a rough linked-size signal, not exact per-crate accounting.

## Deterministic vector

Test secret bytes:

    11 repeated 32 times

Message:

    seed:phase0:event-signature:v1

Public key:

    d04ab232742bb4ab3a1368bd4615e4e6d0224ab71a016baf8520a332c9778737

Signature:

    0b0e491a61de67f3909383608bb4b1f7edc216e951b3a36427a7276e6d833e9b
    92bedc7ea5b68fe82f25ba2c930692490091f1937ad46d84e8da803cae6cfb00

The Core unit test freezes this public key and signature exactly and also
verifies that a one-byte message change is rejected.

## Next evidence

The next protocol step is canonical Event bytes. Only after that encoding is
stable enough for a spike should this signature wrapper sign Event payloads
rather than an isolated vector string.
