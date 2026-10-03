# ADR-0006: Root Identity 与 Device Keys 分离

- Status: Proposed
- Date: 2026-10-03
- Decision scope: Identity / Crypto / Direct handshake

## Context

Seed 不依赖中心账号数据库，因此身份连续性必须由密码学材料提供。

同时 Root Identity 不能承担所有日常网络操作，否则：

- Root Key 暴露面太大；
- 多设备撤销困难；
- 一台设备失陷会直接等于长期身份失陷。

## Decision

身份至少分两层：

```
Root Identity
├── Device Certificate A
├── Device Certificate B
└── Device Certificate C
```

### Root Identity

职责：

- 长期身份锚；
- 签署 Device Certificate；
- 撤销/轮换关键身份材料；
- 极少参与日常操作。

### Device

设备拥有独立的：

- signing key；
- key-agreement material；
- device id；
- validity/revocation metadata。

日常 Event 由 Device 签名，并通过 Root-signed Device Certificate 验证其归属。

## Proposed primitives

Protocol v0 优先评估：

- Ed25519：Root / Device signing；
- X25519：key agreement；
- SHA-256：protocol object IDs / transcript hash where applicable；
- ChaCha20-Poly1305：transport AEAD。

Secure session 优先采用成熟 Noise Framework pattern，而不是自定义握手。

具体 Noise pattern（例如 XX/IK）要根据“首次见面”和“已知联系人”两类流程分别设计，并在 test vectors 后 Accepted。

## Hard rules

1. Root Private Key 不提供给插件；
2. Root Key 不用于每条消息签名；
3. Device revocation 必须可被验证；
4. 身份资料中的昵称/头像不是密码学身份；
5. IP/网络 endpoint 不是身份；
6. 不自行设计新的密码算法。

## Recovery

Recovery 暂不在本 ADR 冻结。

必须单独设计：

- encrypted backup；
- recovery code/material；
- optional trusted recovery；
- lost-all-keys behavior。

## Validation

- root -> device cert test vectors；
- revoked device event rejected；
- wrong root chain rejected；
- session MITM tests；
- replay tests；
- key separation review；
- malformed key/cert fuzzing。

## Revisit conditions

- 外部密码学审查指出 primitive/profile 不合适；
- 目标平台缺少可靠实现；
- PQ migration 需求进入正式 roadmap。

## References

- https://noiseprotocol.org/
