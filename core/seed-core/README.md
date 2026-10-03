# seed-core

seed-core is the executable protocol-kernel spike for Seed.

Current scope:

- typed protocol IDs;
- SpaceKind / SpaceDescriptor without social roles;
- capability request/decision primitives with default-deny behavior;
- plugin manifest permission declarations;
- Ed25519 Root Identity and Device Identity keys;
- Root -> Device authorization records;
- strict Ed25519 signature verification;
- SHA-256-derived Identity / Device / Event IDs with domain separation;
- signed event envelope and deterministic Phase-1 signing frame;
- append-only in-memory event store with duplicate suppression.

Security choices in this spike:

- Ed25519 is provided by ed25519-dalek, not a Seed-specific algorithm;
- weak-key-compatible verification paths are not used;
- temporary secret seed material is securely zeroized;
- private key bytes are not exposed by the core signer API;
- signing keys are for identity/event authenticity only, not transport encryption.

Not implemented yet:

- persistent/OS-backed key storage;
- device revocation and key rotation;
- final canonical wire serialization;
- persistent event storage;
- networking and secure sessions;
- plugin sandbox/runtime;
- Genesis state machine.
