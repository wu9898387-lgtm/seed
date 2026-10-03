# Phase 0 Spike — Storage

> Status: running

## Goal

Measure the smallest useful durable store and compare it with a mature database
adapter without allowing the database choice to leak into Core protocol types.

## Core interface

The Phase 0 RecordStore boundary supports:

- append(record);
- scan(visitor).

Records are opaque bytes to the storage adapter.

## Append-only baseline

Implementation:

- Rust std::fs only;
- u32 big-endian record length;
- opaque record payload;
- explicit sync_data;
- reopen and ordered replay test.

The first stored record is the 119-byte canonical Event vector.

## System SQLite comparison

Implementation:

- rusqlite 0.40.2;
- default features disabled;
- system SQLite linkage;
- WAL mode;
- events table with an autoincrement sequence and BLOB record;
- close/reopen/count/query test.

This result must not be confused with a bundled SQLite measurement: the SQLite
engine itself is outside the Seed executable in this configuration.

## Questions this spike should answer

1. How much binary size does the minimal append-only path cost?
2. How much Seed-side code does system SQLite add?
3. Is system SQLite attractive enough to justify platform-specific adapters?
4. How much of the remaining 2 MiB headroom should storage be allowed to use?

The next iteration must add truncation/corruption behavior before either path is
considered production-safe.
