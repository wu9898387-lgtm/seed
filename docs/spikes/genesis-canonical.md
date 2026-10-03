# Phase 0 Spike — Canonical Genesis Bytes

> Status: running

## Goal

Create one deterministic creator-signed Genesis representation while keeping
the general Seed serialization decision open.

## Framing

The Phase 0 signing preimage is:

    "SEED-GN1"              8 bytes domain
    protocol_major         u16 big-endian
    protocol_minor         u16 big-endian
    space_kind             u16 big-endian
    space_id               32 bytes
    creator_identity_id    32 bytes
    created_at_ms          u64 big-endian
    plugin_count           u16 big-endian
    plugins...

Each plugin entry is:

    plugin_id              16 bytes
    version_major          u16 big-endian
    version_minor          u16 big-endian
    version_patch          u16 big-endian
    config_len             u32 big-endian
    config                 config_len bytes

## Canonical plugin ordering

Plugins must be strictly sorted by raw PluginId bytes.

This means:

- duplicate PluginIds are rejected;
- the same plugin set cannot produce multiple Genesis byte sequences merely
  because two implementations iterate a map differently.

Plugin config bytes remain opaque to Core. A plugin that places structured data
inside config is responsible for defining its own deterministic encoding.

## Fixed vector

The CI vector contains two sorted plugins and encodes to **151 bytes**. It is
signed with the same deterministic Ed25519 test key used by the Event spike.

The exact signature is frozen in the Core unit test.

## Still open

- Genesis hash algorithm;
- whether SpaceId is independent or derived from Genesis;
- how plugin config schemas are versioned;
- whether a standard deterministic format such as deterministic CBOR should
  replace this fixed framing before protocol freeze.
