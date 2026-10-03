# ADR-0005: Raw frame transport boundary

- Status: Provisional
- Date: 2026-10-04

## Context

Seed needs Direct, Relay, and Tree client-host transports without coupling
network plumbing to Identity, Governance, Event semantics, or a specific secure
session protocol.

The Phase-0 implementation plan also calls for a localhost peer/TCP spike so the
binary-size and API cost of networking can be measured before selecting a larger
async or QUIC stack.

## Decision

Define a minimal FrameTransport boundary that only sends and receives opaque byte
frames.

The first implementation is TcpFrameTransport using std::net::TcpStream.

TCP framing is:

1. u32 big-endian payload length;
2. exactly that many opaque payload bytes.

Frames are size-limited and truncated frames are rejected.

## Security boundary

FrameTransport is **not** a secure session.

It does not:

- authenticate the peer;
- negotiate Seed Identity;
- encrypt payloads;
- provide forward secrecy;
- prevent replay;
- grant any Capability;
- assign social authority.

A future secure-session layer must sit above FrameTransport. Only encrypted,
authenticated session payloads should be sent through a Relay path in a real
deployment.

The current loopback smoke sends canonical Event bytes directly only to exercise
the framing and binary-size path on localhost. That is a test/spike behavior, not
a production network security design.

## Why this separation

Keeping raw transport below the secure session allows Seed to reuse the same
upper session/event semantics over:

- direct TCP;
- a future QUIC/UDP transport;
- Relay;
- Tree client-host connections;
- test transports.

It also prevents NAT traversal or Relay implementation details from becoming
implicit sources of Identity or Governance authority.

## Current TCP spike

The implementation provides:

- connect by SocketAddr;
- wrapping an accepted TcpStream;
- peer address inspection;
- TCP_NODELAY configuration;
- bounded length-prefixed send/receive;
- clean-close vs truncated-frame distinction.

Automated tests cover:

- loopback request/reply;
- clean peer close;
- truncated length prefix.

## Size measurement

With the localhost TCP path exercised by the release smoke binary:

- previous Event wire + FileEventStore build: 415,648 bytes (405.9 KiB);
- TCP frame transport build: **426,456 bytes (416.5 KiB)**;
- delta: **10,808 bytes (about 10.6 KiB)**.

No new third-party networking dependency is introduced.

## Not decided yet

This ADR does not freeze:

- secure handshake protocol;
- session key agreement;
- AEAD selection;
- rekey policy;
- connection multiplexing;
- QUIC vs TCP for production Direct traffic;
- NAT traversal;
- Relay discovery/routing;
- offline mailbox protocol.

Those require separate threat modeling and protocol vectors.

## Revisit conditions

Revisit the transport backend after the authenticated secure-session spike and
after measuring whether std TCP is sufficient for the initial desktop MVP.

A QUIC/async runtime should not enter Core merely for convenience; its binary
size and operational benefit must be measured against this baseline.
