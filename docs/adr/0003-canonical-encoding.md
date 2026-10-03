# ADR-0003: Narrow canonical binary encoding for signed kernel records

- Status: Provisional
- Date: 2026-10-03

## Context

Seed needs byte-for-byte deterministic encodings for records whose signatures,
hashes, or identifiers depend on serialized content.

The Phase-0 plan listed CBOR, MessagePack, Protobuf, and a custom canonical
binary format as candidates.

General application payloads and plugin-defined data do not need to share the
same encoding as the small set of security-critical kernel records.

## Decision

Use a narrow, purpose-built canonical binary encoding for signed Core records
such as Genesis during the protocol spike.

This is not a general-purpose serialization framework.

Current rules:

1. Integers are fixed-width big-endian.
2. Variable byte strings use a u32 byte-length prefix.
3. Text, when used by a signed kernel schema, is UTF-8 bytes with a byte-length
   prefix.
4. No floating-point values are allowed in signed kernel schemas.
5. No unordered maps are allowed in signed kernel schemas.
6. Set-like collections define an explicit canonical sort key.
7. Genesis plugins are sorted strictly by PluginId.
8. Duplicate PluginId values are invalid.
9. Decoders reject trailing bytes, invalid enum values, unsupported versions,
   oversized records, and non-canonical ordering.
10. Every signed object has explicit schema/wire versioning and domain-separated
    signature/hash namespaces.

Genesis schema/wire v2 additionally carries a 128-bit creation nonce and pins each plugin by:

- PluginId;
- semantic version;
- 32-byte package digest;
- opaque configuration bytes.

## Why not CBOR / MessagePack / Protobuf here

All three remain reasonable choices for other Seed layers.

For the signed kernel, the current format has useful spike properties:

- zero serialization dependencies;
- very small implementation surface;
- no ambiguity about map ordering;
- no float normalization questions;
- no generated-code/runtime dependency;
- straightforward cross-language test vectors.

The cost is that Seed must maintain its own tiny encoder/decoder and version
every schema carefully.

## Scope boundary

This ADR does **not** require plugins, chat payloads, UI state, or RPC APIs to
use this format.

Those layers may later use CBOR, Protobuf, MessagePack, or another format if
their tradeoffs are better.

## Genesis v1 lifecycle

Genesis is modeled as:

Draft -> canonicalize plugin order -> sign by authorized Device -> immutable
GenesisRecord -> derive GenesisId -> derive SpaceId.

The creator field records who signed the initial state. It does not grant an
Owner/Admin role in Core.

The creation nonce is covered by the canonical signed bytes. Two otherwise
identical creations can therefore produce distinct GenesisId / SpaceId values
without depending on clock uniqueness.

After activation, changes to governance or plugins must be expressed through
new authorized events. Replacing the Genesis bytes is not a state transition.

## Revisit conditions

Revisit before protocol freeze if:

- independent implementations find the custom codec error-prone;
- a standard canonical format produces comparable size and simpler auditing;
- schema evolution becomes difficult;
- fuzzing reveals parser complexity disproportionate to the benefit.

Before public Alpha, the canonical decoder must receive fuzz coverage and
cross-language vectors.
