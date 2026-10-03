# Phase 0 Spike — Transport

> Status: running

## Goal

Prove the transport abstraction with the smallest possible real network path
before choosing QUIC, NAT traversal, discovery, or relay infrastructure.

## Direct path

- bind loopback TCP on an ephemeral port;
- send a u32-length-framed 119-byte canonical Event;
- receiver reconstructs the exact frame.

## Relay path

A minimal in-memory mailbox runs on loopback TCP.

1. sender connects and PUTs an opaque frame for target 7;
2. relay stores only the frame bytes;
3. receiver connects and GETs target 7;
4. receiver obtains byte-for-byte identical content.

The relay has no governance semantics.

## Important limitation

The current Event bytes are signed/canonical protocol bytes, not an encrypted
session envelope. Therefore this spike demonstrates routing equivalence, not
confidentiality.

The next transport security spike must place authenticated encryption/session
semantics above this route selection layer.

## Measurement

CI measures the stripped transport executable independently so the cost of
standard-library TCP framing can be compared with the kernel and other Phase 0
components.
