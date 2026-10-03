# ADR-0004: Event Log 为权威历史，Materialized View 为缓存

- Status: Proposed
- Date: 2026-10-03
- Decision scope: Storage / Sync / Tree Host

## Context

Seed 需要同时支持：

- 本地历史；
- 断线追赶；
- Genesis 不可改写；
- 治理事件审计；
- 多节点验证；
- Tree Host 持久化；
- 插件状态。

如果只同步“当前数据库表”，很难证明状态变化过程，也难以正确处理治理历史。

## Decision

Seed 的状态模型采用：

> **Validated Event Log = authoritative history**
>
> **Materialized View = rebuildable cache/state projection**

也就是说：

1. 合法 Event 先进入 append-oriented history；
2. reducer/materializer 将 Event 投影为当前 Space 状态；
3. UI 读取 materialized state；
4. materialized state 损坏时应能从可用历史重建；
5. Genesis 是每个 Space 的历史根。

数据库产品不进入 wire protocol。

Core 需要 Storage interface，例如：

```text
append_event()
has_event()
scan_space_events()
load_genesis()
put_plugin_state()
get_plugin_state()
checkpoint()
```

## MVP backend

MVP backend 暂不 Accepted。

Phase 0 比较：

- SQLite adapter；
- 简单 append-only file + index。

优先正确性、crash recovery 和体积，不为了几十 KB 自己实现一个脆弱数据库。

## Blob

大附件不直接塞进 Event Log。

Event 只引用：

- content hash；
- size；
- media metadata；
- locator/transfer metadata（按协议定义）。

## Alternatives

### Mutable tables as source of truth

优点：

- CRUD 简单。

缺点：

- 治理历史难验证；
- 状态篡改更难发现；
- 离线同步语义变复杂。

### 全量 CRDT-first

优点：

- 强大的离线并发语义。

缺点：

- MVP 复杂度过高；
- Group / Tree / Governance 的需求并不全部等同于协作文档。

## Validation

- kill -9 / crash recovery；
- duplicate event idempotency；
- invalid event 不进入 materialized state；
- rebuild 与在线 materialization 结果一致；
- 10k/100k event 基线；
- storage binary-size contribution。

## Revisit conditions

- 多设备并发需求证明当前事件模型不足；
- 特定 Space 类型需要独立 CRDT 数据结构；
- backend 在移动端/Tree Host 上无法满足可靠性或体积目标。
