# ADR-0005: Transport

- Status: Proposed / Spike
- Date: 2026-10-04
- Scope: Direct, Relay, and Tree transport adapters

## Context

Seed needs one upper-layer envelope/event model while allowing multiple delivery
paths:

- direct peer connection;
- relay/mailbox fallback;
- Tree client-host connection;
- later NAT traversal and mobile-friendly transports.

Transport selection must not grant social authority. A relay or Tree Host can
carry/store bytes without becoming an Owner/Admin in protocol semantics.

## Phase 0 TCP baseline

The first spike uses only the Rust standard library and loopback TCP.

It proves:

1. a length-framed canonical Event can travel directly over TCP;
2. the exact same envelope can be PUT into a relay mailbox;
3. a receiver can GET that unchanged envelope later;
4. Direct and Relay do not require different upper-layer message types.

The relay protocol used here is deliberately tiny and not production-ready:

    PUT: 'P' | target:u8 | frame
    GET: 'G' | target:u8
    frame: len:u32be | bytes

## What this does not decide

- authenticated handshake;
- E2EE/session keys;
- forward secrecy;
- replay windows;
- NAT traversal;
- QUIC vs TCP;
- multiplexing;
- congestion behavior;
- offline retention limits;
- relay authentication/abuse controls;
- discovery;
- Tree protocol.

## Current direction

Keep Transport as an adapter boundary. Higher layers submit opaque protocol
frames and should not need to know whether the selected route is Direct, Relay,
or Tree.

A later secure-session layer must encrypt/authenticate before an untrusted Relay
is allowed to handle message content.

## Evidence required before Accepted

- Linux stripped size of the TCP/relay baseline;
- authenticated peer handshake;
- encrypted Direct payload;
- identical encrypted envelope through Relay fallback;
- wrong-identity handshake failure;
- replay rejection;
- disconnect/block behavior;
- QUIC comparison or an explicit reason to keep TCP;
- NAT traversal strategy;
- bounded frame sizes and malformed-frame tests.

## Revisit conditions

Reopen if a concrete transport requires social roles, leaks raw transport
objects into plugin APIs, or forces different Event semantics between Direct
and Relay paths.
