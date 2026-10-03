# ADR-0003: Serialization and canonical signing bytes

- Status: Proposed / Spike
- Date: 2026-10-03
- Scope: protocol serialization

## Context

Genesis, Event IDs, signatures, governance records, and cross-implementation
test vectors need deterministic bytes. Generic serialization formats often
permit more than one valid byte representation for the same data model.

RFC 8949 explicitly distinguishes normal CBOR from deterministic CBOR and
requires a protocol to define the deterministic restrictions it relies on.

Seed also has an aggressive Core size target, so the cost of a general-purpose
codec matters.

## Current spike decision

Do not freeze the repository on CBOR, MessagePack, Protobuf, or a custom
general-purpose format yet.

For the first Event signing path, use a tiny protocol-owned fixed framing:

    "SEED-EV1"                8 bytes domain
    protocol_major           u16 big-endian
    protocol_minor           u16 big-endian
    event_kind               u16 big-endian
    space_id                 32 bytes
    author_identity_id       32 bytes
    device_id                32 bytes
    payload_len              u32 big-endian
    payload                  payload_len bytes

This framing has exactly one representation for the current field set and
requires no allocator or external codec. It is intentionally a signing-preimage
spike, not the final representation for all Seed objects.

EventId and signature are excluded from the preimage because they are derived
from or attest to those bytes.

## Alternatives to measure before Accepted

### RFC 8949 deterministic CBOR

Pros:
- standardized self-describing format;
- compact integer keys are possible;
- explicit deterministic encoding rules exist.

Questions:
- library size and no_std profile;
- whether the chosen encoder can enforce Seed's deterministic subset;
- unknown-field and schema evolution behavior.

### MessagePack

Pros:
- compact and widely implemented.

Questions:
- canonical map/key ordering is not inherently the protocol's default;
- deterministic cross-language constraints would need to be specified.

### Protobuf

Pros:
- mature schema evolution and tooling.

Questions:
- deterministic serialization is not the same as a canonical wire identity
  across every implementation/use;
- generated/runtime code cost for the Core target.

### Protocol-owned binary schema

Pros:
- smallest measured implementation;
- exact control over signing bytes and versioning.

Risks:
- Seed owns all compatibility rules and parser security;
- schema evolution can become brittle;
- duplicating a standards ecosystem may be a long-term maintenance cost.

## Consequences

The Event/signature work can proceed immediately with stable test vectors while
keeping the final general serialization choice open.

Genesis must not automatically reuse this fixed Event framing. Genesis has
plugin/config collections and needs an explicit evolution story before its
canonical form is frozen.

## Current evidence

The fixed Event preimage is now implemented without allocation or third-party
serialization dependencies. CI freezes field offsets, signs the complete
119-byte vector with the Phase 0 Ed25519 key, and rejects drift through an exact
signature assertion.

The linked release binary grew from **354,936 bytes** for the crypto spike to
**355,624 bytes** for canonical Event + crypto, a **688-byte** increment.

This supports keeping the narrow Event framing for the next Phase 0 steps, but
does not resolve the general Genesis/wire-format choice.

## Acceptance evidence still required

- exact Event test vector in CI;
- linked binary-size delta for this codec;
- deterministic CBOR comparison spike;
- Genesis schema experiment;
- malformed/truncated decode strategy;
- at least one second-language vector check before protocol freeze.

## Revisit conditions

Reopen immediately if the fixed framing starts accumulating optional fields,
nested maps, or compatibility exceptions. That is evidence the spike is being
mistaken for a general serialization format.
