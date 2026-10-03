# ADR-0004: Storage

- Status: Proposed / Spike
- Date: 2026-10-04
- Scope: local durable event/state storage

## Context

Seed needs durable local history, Genesis records, governance events, plugin
state, and eventually indexes/materialized views. The storage choice affects:

- Core binary size;
- crash recovery;
- schema migration;
- write amplification;
- local encryption strategy;
- corruption handling;
- query/index complexity;
- portability across desktop/mobile/Tree Host targets.

The protocol must not depend on one database engine. Storage remains an adapter
behind a narrow Core boundary.

## Core boundary

Phase 0 introduces a minimal RecordStore interface:

- append one opaque protocol record;
- scan records in durable order.

Core does not expose SQLite types, filesystem paths, SQL, or database handles.

This is deliberately smaller than the eventual query/index interface.

## Candidate A: append-only framed log

The first baseline uses only the Rust standard library:

    record_len   u32 big-endian
    record       record_len bytes

The spike proves:

- append;
- explicit sync_data;
- close;
- reopen;
- ordered replay.

Advantages:

- very small implementation;
- transparent on-disk framing;
- deterministic recovery model;
- no database dependency.

Current limitations:

- no checksum;
- no index;
- no transaction grouping;
- no compaction;
- no concurrent writers;
- truncated-tail semantics are not yet fully specified;
- application must build materialized indexes separately.

## Candidate B: system SQLite

The comparison uses rusqlite 0.40.2 without the bundled SQLite feature.

This intentionally tests the architectural option of using a platform/system
SQLite library while keeping SQLite itself outside the Seed binary size
measurement.

The spike proves:

- file database creation;
- WAL mode;
- BLOB event insert;
- close/reopen;
- ordered query.

Advantages:

- mature transaction and recovery semantics;
- indexing/query capabilities;
- widely deployed storage engine;
- potentially small Seed binary cost when a trusted system SQLite is available.

Risks:

- system SQLite availability/version differs by platform;
- mobile/desktop packaging behavior is platform-specific;
- relying on external SQLite changes the reproducibility and security-update
  model;
- bundled SQLite would have a materially different size result and must be
  measured separately if needed.

## Decision for now

Do not freeze the storage engine.

Keep the protocol-facing storage boundary engine-neutral and measure both
approaches. Append-only remains the minimum reference implementation; SQLite is
the mature comparison.

## Evidence required before Accepted

- Linux stripped release sizes for both spikes;
- truncated append-log recovery policy;
- corruption detection/checksum strategy;
- fsync/durability policy;
- SQLite bundled-vs-system packaging comparison on targets that lack a trusted
  system SQLite;
- local encryption-at-rest integration;
- indexed event lookup;
- crash-interruption tests;
- migration/versioning strategy.

## Revisit conditions

Reopen whenever a storage-specific assumption leaks into Event/Genesis wire
formats or plugin APIs.
