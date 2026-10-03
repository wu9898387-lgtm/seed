# seed-core

`seed-core` is the executable protocol-kernel spike for Seed.

Current scope:

- typed Identity / Device / Space / Event / Genesis / Plugin IDs;
- SpaceKind / SpaceDescriptor without social roles;
- capability request/decision primitives with default-deny behavior;
- plugin manifest permission declarations;
- Ed25519 Root Identity and Device Identity keys;
- Root -> Device authorization records;
- strict Ed25519 signature verification;
- domain-separated SHA-256 Identity / Device / Genesis / Space / Event IDs;
- deterministic signed Event envelope;
- canonical signed Genesis records;
- plugin package digest pinning inside Genesis;
- append-only in-memory Event Store with duplicate suppression;
- experimental append-only file Event Store with reopen indexing, EventId integrity checks, and partial-tail recovery.

Genesis rules in the current spike:

- a Genesis draft is mutable only before activation;
- plugin order does not affect canonical bytes or GenesisId;
- duplicate PluginId entries are rejected;
- plugin package bytes are represented by a pinned digest;
- every Genesis includes an explicit 128-bit creation nonce;
- Space identity does not depend on wall-clock uniqueness;
- the activated Genesis record is immutable through the public API;
- SpaceId is derived from GenesisId;
- creator identity is provenance only and does not imply Owner/Admin authority;
- post-activation governance changes must eventually be represented as events,
  rather than rewriting Genesis.

Security choices in this spike:

- Ed25519 is provided by ed25519-dalek, not a Seed-specific algorithm;
- weak-key-compatible verification paths are not used;
- temporary secret seed material is securely zeroized;
- private key bytes are not exposed by the core signer API;
- signing keys are for identity/event authenticity only, not transport encryption.

Not implemented yet:

- persistent/OS-backed key storage;
- device revocation and key rotation;
- materialized views, checkpoints, and large-history storage indexing;
- SQLite backend comparison;
- networking and secure sessions;
- plugin sandbox/runtime;
- governance execution;
- Tree Host.
