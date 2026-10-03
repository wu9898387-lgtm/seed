# Seed protocol vectors

Status: Phase-1/Genesis vector v2, protocol not yet frozen.

This document provides deterministic values that another implementation can use
to validate the current Identity -> DeviceAuthorization -> Genesis -> Event path.

Vector v1 was superseded when Genesis gained an explicit creation nonce. The
wire/schema version was bumped rather than silently redefining the old bytes.

## Vector v2 inputs

Identity and device:

- Root Ed25519 secret seed: 32 bytes, all 0x01
- Device Ed25519 secret seed: 32 bytes, all 0x02
- Device authorization sequence: 1
- Device authorization issued_at_ms: 1700000000000

Genesis:

- Genesis schema version: 2
- Genesis wire version: 2
- SpaceKind: Group
- Genesis created_at_ms: 1700000000100
- creation nonce: 16 bytes, all 0x03
- PluginId: 32 bytes, all 0x05
- Plugin version: 1.2.3
- Plugin package digest: 32 bytes, all 0x06
- Plugin config UTF-8 bytes: owner=creator

Event:

- Event protocol version: 1
- Event sequence: 7
- Event timestamp_ms: 1700000000123
- Event schema: seed.message.text/v1
- Event payload UTF-8 bytes: hello
- Event SpaceId: derived from Genesis

## Vector v2 expected outputs

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

GenesisId:

064056bb758914219b527afc13bd39b705b6af7844a304f15e0786bad60021bc

SpaceId:

ccebbe4515777718143f6fc51b37c4412366b08c4c73757a92668fb276b86806

Genesis Device signature:

198a14442860990599bf124a56c638f571b71a8cfb42f2f74bc5d04f541dc4d59ae684ac4ba0927be41b22956151bd5b64b1df6bee10439f6b6064812ab1130b

Canonical Genesis bytes:

5347454e0002000000b6000200010200524173a9a0f74fe50a3ffde32c2c8cf013c734a39ae7847fb56fd98ddefdb973c192ef5ee4e3f2e1815f48f09cf8f3d8d958d6e347aa07b456389e03554c4a0000018bcfe56864030303030303030303030303030303030001050505050505050505050505050505050505050505050505050505050505050500010002000306060606060606060606060606060606060606060606060606060606060606060000000d6f776e65723d63726561746f72198a14442860990599bf124a56c638f571b71a8cfb42f2f74bc5d04f541dc4d59ae684ac4ba0927be41b22956151bd5b64b1df6bee10439f6b6064812ab1130b

EventId:

5670cf7b9d76f1814d0ae6bf6b75e5c715c289e50e10754e77eecfbbda656af8

Event Device signature:

ffe70b44a079531fa750e4c94004cab15ce4385cf01c5641b5d4eabeb75620ac15e2f9c98bccd5c4d149a9909156436e2730a9b2dd40cb4245d8fd70ff13df0d

## Domain separation

Current namespaces remain:

- seed:identity-id:v1 followed by a NUL byte
- seed:device-id:v1 followed by a NUL byte
- seed:device-authorization:v1 followed by a NUL byte
- seed:genesis-signature:v1 followed by a NUL byte
- seed:genesis-id:v1 followed by a NUL byte
- seed:space-id:v1 followed by a NUL byte
- seed:event-signature:v1 followed by a NUL byte
- seed:event-id:v1 followed by a NUL byte

Genesis schema/wire versioning is independent from these object-domain labels.

The canonical format remains provisional until the protocol is frozen. Any
future incompatible framing change must version-bump rather than silently
changing the expected values.
