# seed-core

`seed-core` is the first executable protocol-kernel spike for Seed.

Current scope is intentionally narrow:

- typed protocol IDs;
- `SpaceKind` / `SpaceDescriptor` without social roles;
- capability request/decision primitives with default-deny behavior;
- plugin manifest permission declarations;
- event envelope + deterministic Phase-0 signing frame;
- crypto adapter traits that do not expose private key bytes;
- append-only in-memory event store with duplicate suppression.

Not implemented yet:

- production cryptography;
- key storage;
- final canonical wire serialization;
- persistent storage;
- networking;
- plugin sandbox/runtime;
- Genesis state machine.

The absence of production crypto is deliberate: the project must use a mature
cryptographic implementation rather than inventing a temporary algorithm just
to make the spike appear complete.
