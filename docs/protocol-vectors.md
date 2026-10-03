# Seed protocol vectors

Status: Phase-1 test vector, protocol not yet frozen.

This document provides deterministic values that another implementation can use
to validate the current Identity -> DeviceAuthorization -> Event signing path.

## Vector v1 inputs

- Root Ed25519 secret seed: 32 bytes, all 0x01
- Device Ed25519 secret seed: 32 bytes, all 0x02
- Device authorization sequence: 1
- Device authorization issued_at_ms: 1700000000000
- SpaceId: 32 bytes, all 0x03
- Event protocol version: 1
- Event sequence: 7
- Event timestamp_ms: 1700000000123
- Event schema: seed.message.text/v1
- Event payload UTF-8 bytes: hello

## Vector v1 expected outputs

Root public key:
8a88e3dd7409f195fd52db2d3cba5d72ca6709bf1d94121bf3748801b40f6f5c

IdentityId:
00524173a9a0f74fe50a3ffde32c2c8cf013c734a39ae7847fb56fd98ddefdb9

Device public key:
8139770ea87d175f56a35466c34c7ecccb8d8a91b4ee37a25df60f5b8fc9b394

DeviceId:
73c192ef5ee4e3f2e1815f48f09cf8f3d8d958d6e347aa07b456389e03554c4a

DeviceAuthorization Root signature:
5c762762b103c4fbbd382a08f900e5d4e2ffa0a4be1c7d0f6bce65ce1a1d8b4b5f80b9d7d4ac736afaba7a546263aeab13d4ec84814579a63ff3d260e6f7570f

EventId:
c620f326a53b852d0a0b5a1b128b24da0a4c750a9473a135d5ef6467b5e71ff5

Event Device signature:
f68705f312deb509321dc190f7a8fa8840c60547d8b8b7284b073ed3be1bb3356a3589f50902de1705256caf253c2bfc0a643fb5d8a0f22e06ffb97c2087ed0c

## Domain separation

Current Phase-1 namespaces:

- seed:identity-id:v1 followed by a NUL byte
- seed:device-id:v1 followed by a NUL byte
- seed:device-authorization:v1 followed by a NUL byte
- seed:event-signature:v1 followed by a NUL byte
- seed:event-id:v1 followed by a NUL byte

The canonical event framing is still considered provisional until the
serialization ADR is completed. If that framing changes, this vector must
version-bump rather than silently changing expected values.
