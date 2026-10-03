# ADR-0002: Root and Device signing keys

- Status: Provisional
- Date: 2026-10-03

## Context

Seed needs cryptographic identity continuity without making network addresses,
phone numbers, or real-world identity documents part of the protocol identity.

The MVP also requires a distinction between a long-lived Root Identity and
individual Device identities. A compromised or replaced device must not require
the social identity itself to be replaced.

## Decision

For the Phase-1 spike:

1. A Root Identity is represented by an Ed25519 signing key.
2. IdentityId is SHA-256(domain || root_public_key).
3. Each device has an independent Ed25519 signing key.
4. DeviceId is SHA-256(domain || device_public_key).
5. The Root key signs a DeviceAuthorization record binding:
   - IdentityId
   - DeviceId
   - device public key
   - authorization sequence
   - issuance timestamp
6. Events are signed by the authorized Device key, not by the Root key.
7. EventId is derived from canonical event content plus the device signature.
8. All hash namespaces use explicit domain separation.
9. Strict Ed25519 verification is required.
10. Ed25519 keys are not reused for transport encryption.

## Why

The Root key stays out of routine message signing and can later be placed behind
stronger local storage, hardware-backed storage, or an offline recovery path.

Independent Device keys also make later device revocation, multi-device sync,
and key rotation possible without inventing social roles in Core.

## Current implementation

The spike uses:

- ed25519-dalek 3.0.0;
- SHA-256 from sha2 0.11.0;
- OS randomness through getrandom 0.4.3;
- zeroize 1.9.0 for temporary secret material.

The Ed25519 dependency is built without its default speed table feature in this
spike so the size cost can be measured separately from throughput optimization.

## Not decided yet

This ADR does not freeze:

- device revocation event format;
- Root key rotation/recovery;
- multi-device authorization policy;
- persistent secret-key storage;
- transport/session key agreement;
- final wire serialization.

These require separate ADRs and threat-model tests.
