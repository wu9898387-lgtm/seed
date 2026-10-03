# Seed protocol vectors

Status: Phase-1/Genesis test vector, protocol not yet frozen.

This document provides deterministic values that another implementation can use
to validate the current Identity -> DeviceAuthorization -> Genesis -> Event path.

## Vector v1 inputs

Identity and device:

- Root Ed25519 secret seed: 32 bytes, all 0x01
- Device Ed25519 secret seed: 32 bytes, all 0x02
- Device authorization sequence: 1
- Device authorization issued_at_ms: 1700000000000

Genesis:

- SpaceKind: Group
- Genesis created_at_ms: 1700000000100
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

GenesisId:

27f4890bfbd715f8f49052baeaf99835b3f742c5244b40ba53ef5722c89bc823

SpaceId:

37d08795ca6c64054be999fd42e7d8fc797a097db1ff84bf5c06bb08fec6ebe8

Genesis Device signature:

50ec0caea3c14516945a8870b631e22caa3a4406efdd21b204a74415e289cb50afa4b9b3bf4a552d0c65697e82c39f78d9394065e5afda113e4f45fa0f39e308

Canonical Genesis bytes:

5347454e0001000000a6000100010200524173a9a0f74fe50a3ffde32c2c8cf013c734a39ae7847fb56fd98ddefdb973c192ef5ee4e3f2e1815f48f09cf8f3d8d958d6e347aa07b456389e03554c4a0000018bcfe568640001050505050505050505050505050505050505050505050505050505050505050500010002000306060606060606060606060606060606060606060606060606060606060606060000000d6f776e65723d63726561746f7250ec0caea3c14516945a8870b631e22caa3a4406efdd21b204a74415e289cb50afa4b9b3bf4a552d0c65697e82c39f78d9394065e5afda113e4f45fa0f39e308

EventId:

d20549bd5b6e2bae07ea7ea15fd76150e67c4e20bb3699a46eb37f66245134bc

Event Device signature:

65f8d1f5be10a11a16acedfa3251f903b37d910483a1d4c1bb221687eeba8bae89e11a025a285e1be2e726f5281ae25cf2ebb266824b5b98177848c51ec9950e

## Domain separation

Current namespaces:

- seed:identity-id:v1 followed by a NUL byte
- seed:device-id:v1 followed by a NUL byte
- seed:device-authorization:v1 followed by a NUL byte
- seed:genesis-signature:v1 followed by a NUL byte
- seed:genesis-id:v1 followed by a NUL byte
- seed:space-id:v1 followed by a NUL byte
- seed:event-signature:v1 followed by a NUL byte
- seed:event-id:v1 followed by a NUL byte

The canonical format remains provisional until the protocol is frozen. Any
incompatible framing change must version-bump rather than silently changing the
expected values.
