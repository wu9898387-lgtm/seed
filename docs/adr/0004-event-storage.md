# ADR-0004: Append-only local Event log

- Status: Provisional
- Date: 2026-10-04

## Context

Seed is Local-first. A node must be able to persist signed Events, exit, restart,
replay local history, and independently re-verify the cryptographic authorization
chain without depending on an online service.

The first storage spike should also expose corruption and crash-boundary behavior
before a larger database dependency is selected.

## Decision

Use a small append-only file log as the first persistent Event Store reference
implementation.

The file starts with:

- ASCII magic: SLOG
- u16 log version: 1

Each record is:

1. u32 big-endian canonical Event byte length;
2. canonical SEVT Event bytes;
3. 32-byte SHA-256 checksum using the domain
   seed:event-log-record:v1 followed by a NUL byte.

The Event itself remains the authenticated protocol object. The record checksum
is only an early corruption detector; it is not an authorization or authenticity
mechanism.

## Append semantics

FileEventStore:

- rejects oversize records;
- treats an already-known EventId as an idempotent duplicate;
- does not append duplicate bytes to disk;
- writes length, Event bytes, and checksum;
- flushes and calls sync_data before exposing the Event as committed in memory.

This per-record durability policy is intentionally conservative for the spike.
Later implementations may batch durability barriers if benchmarks show that is
necessary.

## Replay semantics

On open, the store:

1. validates the log header/version;
2. replays every record in order;
3. validates record length limits;
4. validates the record checksum;
5. parses the Event through the canonical Event decoder;
6. rejects duplicate EventIds already present in the same log;
7. reconstructs the in-memory Event index.

The current implementation fails closed on:

- truncated records;
- checksum mismatch;
- invalid Event encoding;
- duplicate records;
- unsupported log versions.

It does not silently skip or automatically truncate corrupt data.

## Cryptographic boundary

The storage layer performs structural/canonical decoding but does not decide
whether an Event is socially or cryptographically authorized.

After replay, callers verify the Event with the appropriate IdentityDocument and
DeviceAuthorization.

This keeps local persistence separate from Identity, Governance, and Capability
policy.

## Why not SQLite yet

SQLite remains a strong candidate for later indexed/local state.

The append-only spike is useful first because it:

- adds no new dependency;
- makes the authoritative Event history explicit;
- has simple crash/corruption semantics;
- is easy to test and inspect;
- gives a clean size baseline before introducing a database runtime.

A future storage adapter may use SQLite or another embedded database without
changing the Event wire format.

## Current limitations

The spike currently:

- rebuilds an in-memory index on every open;
- has no compaction;
- has no secondary indexes;
- has no automatic truncated-tail recovery;
- syncs each inserted record individually;
- stores Genesis separately from this Event log abstraction.

These are acceptable for architecture validation, not production scale.

## Revisit conditions

Revisit the backend after measuring:

- startup/replay time at realistic history sizes;
- append latency;
- disk amplification;
- memory used by rebuilt indexes;
- crash behavior under injected write failures;
- SQLite or embedded-KV binary-size cost.

Before public Alpha, recovery policy and backup semantics must be explicit.
