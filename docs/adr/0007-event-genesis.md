# ADR-0007: Genesis 是 Space 历史根，激活后的状态变化通过 Validated Event

- Status: Proposed
- Date: 2026-10-03
- Decision scope: Space / Event / Genesis / Governance

## Context

Seed 必须同时满足：

- 创建前可以自由选择创世插件；
- 激活后创建者没有隐藏 bypass；
- Genesis 不可静默改写；
- 插件包内容能够被历史固定；
- 治理变化可验证；
- Tree Host 不能凭机器控制权伪造合法治理结果。

`core-kernel-spike` 已经实现第一版 Genesis 状态机与 deterministic test vector。

## Current implementation

当前 Spike 的生命周期：

```
GenesisDraft
  -> canonicalize plugin order
  -> authorized Device signs
  -> immutable GenesisRecord
  -> GenesisId
  -> SpaceId
```

Genesis 当前包含：

- SpaceKind；
- creator IdentityId；
- creator DeviceId；
- created_at_ms；
- sorted Genesis plugins。

每个 Genesis plugin 固定：

- PluginId；
- semantic version；
- 32-byte package digest；
- opaque config bytes。

创建者字段只表示 provenance，不产生 Owner/Admin 权限。

## Current ID derivation

当前 Spike：

```
GenesisId = SHA-256(
  domain(seed:genesis-id:v1) ||
  canonical_unsigned_genesis ||
  device_signature
)
```

```
SpaceId = SHA-256(
  domain(seed:space-id:v1) ||
  GenesisId
)
```

这与现有 protocol vector 保持一致。

## Required change before protocol freeze: explicit creation nonce

当前 Genesis 没有独立随机 nonce。

虽然 `created_at_ms` 通常会使两次创建不同，但 Space 唯一性不应依赖：

- 时钟精度；
- 时钟正确性；
- 调用方“碰巧”传入不同时间。

因此在协议冻结前必须评估并优先加入：

```
creation_nonce: 128-bit or 256-bit random value
```

它进入 canonical unsigned Genesis，因此：

- 同一身份；
- 同一设备；
- 同一毫秒；
- 完全相同插件配置

仍然可以创建两个不同 Space。

加入该字段属于 wire-breaking change，必须更新 protocol vector 和 schema/wire version，而不是静默改变现有 v1 bytes。

## Post-activation rule

Genesis 激活以后不得被“编辑”。

后续变化必须表达为新 Event，例如：

- plugin install；
- plugin remove；
- plugin configure；
- membership changes；
- governance changes。

事件进入 authoritative state 前必须经过：

1. schema/framing validation；
2. identity/device authorization validation；
3. signature validation；
4. duplicate/replay validation；
5. Capability/Governance authorization；
6. state transition validation；
7. accepted Event append；
8. materialization。

## Host / Authority rule

关键原则：

> Host 能接收或存储一个数据包，不等于该 Event 是合法状态变化。

恶意 Tree Host 即使注入“把自己设为管理员”的 Event，只要当前 Governance 不授权，客户端和其他验证节点就必须拒绝该状态变化。

Host 仍然可能：

- 拒绝服务；
- 丢弃数据；
- 延迟数据；
- 选择性返回历史。

这些属于基础设施控制问题，需要通过备份、迁移、多 Host 等后续机制缓解，而不是把 Host 直接赋予社会权限。

## Ordering

MVP 不要求所有 Space 使用单一全局 wall-clock total order。

时间戳主要用于展示和辅助处理，不能单独作为授权依据。

具体 causal/order 模型在多设备与 Group sync Spike 后冻结。

## Validation

必须覆盖：

- Genesis canonical bytes；
- GenesisId / SpaceId；
- valid/invalid signature；
- plugin insertion order independence；
- duplicate PluginId rejection；
- package digest pinning；
- activated Genesis immutability；
- explicit nonce uniqueness（加入 nonce 后）；
- duplicate Event；
- unauthorized Event；
- Host-forged governance Event；
- state rebuild determinism。

## Revisit conditions

- 多设备离线并发需要更强 causal model；
- Tree federation 引入多 writer consensus；
- 特定 Space 类型需要专门 CRDT。
