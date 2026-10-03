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
- deterministic signed Event envelope with canonical SEVT wire encoding;
- canonical signed Genesis records;
- plugin package digest pinning inside Genesis;
- append-only in-memory Event Store with duplicate suppression;
- durable append-only FileEventStore with restart replay, record checksums, and
  fail-closed corruption/truncation detection;
- raw FrameTransport boundary and length-prefixed localhost TCP framing spike.

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

Transport boundary:

- TcpFrameTransport moves opaque byte frames only;
- the transport layer has no Owner/Governance semantics;
- raw TCP framing is explicitly **not** an authenticated or encrypted session;
- production Direct/Relay traffic must place an authenticated encrypted session
  above FrameTransport before carrying user content.

Security choices in this spike:

- Ed25519 is provided by ed25519-dalek, not a Seed-specific algorithm;
- weak-key-compatible verification paths are not used;
- temporary secret seed material is securely zeroized;
- private key bytes are not exposed by the core signer API;
- signing keys are for identity/event authenticity only, not transport encryption.

Not implemented yet:

- persistent/OS-backed key storage;
- device revocation and key rotation;
- scalable/indexed production event database;
- automatic repair/salvage of a truncated event-log tail;
- authenticated secure-session handshake and transport encryption;
- NAT traversal / Relay transport;
- plugin sandbox/runtime;
- governance execution;
- Tree Host.
