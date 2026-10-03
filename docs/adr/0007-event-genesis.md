# ADR-0007: Space 以 Genesis 为历史根，所有状态变化通过 Validated Event

- Status: Proposed
- Date: 2026-10-03
- Decision scope: Space / Event / Genesis / Governance

## Context

Seed 必须同时满足：

- 创建前可以自由选择创世插件；
- 激活后创建者没有隐藏 bypass；
- Genesis 不可静默改写；
- 治理变化可验证；
- Host 不能凭机器控制权伪造合法治理结果。

## Decision

每个 Space 由一个 Genesis Body 建立历史根。

概念：

```text
GenesisBody {
    protocol_version
    kind
    nonce
    creator_identity
    initial_members
    initial_plugins
    initial_plugin_config
    host_descriptor?   // Tree only
    created_at
}
```

```text
Genesis {
    body
    creator_signature
}
```

### Space ID

Proposed：

```
SpaceId = SHA-256(
  domain("seed:v0:space-id") ||
  deterministic_cbor(GenesisBody)
)
```

GenesisBody 必须包含随机 nonce，因此同一创建者使用同一配置创建两个 Space 时仍得到不同 SpaceId。

签名验证 GenesisBody，但 SpaceId 不依赖签名字节，从而避免不同签名封装影响标识。

### Event

所有激活后的状态变化通过 Event 表达。

```text
EventBody {
    protocol_version
    space_id
    event_nonce_or_id_material
    author_identity
    author_device
    kind
    payload
    causal/context metadata
}
```

```text
SignedEvent {
    body
    device_signature
}
```

事件进入状态前必须：

1. schema valid；
2. canonical encoding valid；
3. signature valid；
4. device certificate valid；
5. not revoked for applicable policy；
6. Capability/Governance valid；
7. replay/duplicate check；
8. state transition valid。

## Governance rule

关键原则：

> 数据包能被 Host 接收，不等于该 Event 被 Seed 状态机接受。

因此恶意 Tree Host 即使注入一个“把自己设为管理员”的事件，只要当前 Governance 不授权，该事件就不能成为合法 materialized state 的一部分。

Host 仍可拒绝服务或隐藏数据；这是基础设施控制问题，需要后续通过备份、迁移、多 Host 等机制缓解。

## Ordering

Protocol v0 不强制所有 Space 都拥有全球统一 wall-clock total order。

MVP 可以按不同场景定义足够的 event ordering / causal context。

禁止把未经验证的客户端时间戳当作授权依据。

## Validation

必须建立固定 vectors：

- Genesis canonical bytes；
- SpaceId；
- valid/invalid Genesis signature；
- valid Event；
- duplicate Event；
- unauthorized Event；
- Host-forged governance Event；
- rebuild state determinism。

## Revisit conditions

- 多设备离线并发需要更强 causal model；
- Tree federation 引入多 writer consensus；
- 某些 Space 类型需要专门 CRDT。
