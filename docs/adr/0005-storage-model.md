# ADR-0005: Validated Event Log 为权威历史，Materialized View 为可重建状态

- Status: Proposed
- Date: 2026-10-03
- Decision scope: Storage / Sync / Tree Host

## Context

Seed 需要同时支持：

- 本地历史；
- 断线追赶；
- Genesis 不可静默改写；
- 治理事件审计；
- 多节点验证；
- Tree Host 持久化；
- 插件状态。

当前 `main` 有 append-only in-memory Event Store；`storage-spike` 进一步加入了 append-only file baseline 与 partial-tail recovery。

如果只同步“当前数据库表”，很难证明状态变化过程，也难以正确验证治理历史。

## Decision

Seed 的状态模型采用：

> **Validated Event Log = authoritative accepted history**

> **Materialized View = rebuildable state projection**

原则：

1. Event 先经过签名、身份、Capability/Governance 和状态转换验证；
2. 只有合法 Event 才进入 accepted history；
3. reducer/materializer 将 accepted Event 投影为当前 Space 状态；
4. UI 读取 materialized state；
5. materialized state 损坏时应能由 Genesis + accepted history 重建；
6. Genesis 是每个 Space 的历史根；
7. 数据库产品不进入 wire protocol。

Core Storage interface 至少需要表达：

```text
append_accepted_event()
has_event()
scan_space_events()
load_genesis()
put_plugin_state()
get_plugin_state()
checkpoint()
```

## MVP backend

持久化 backend 尚未 Accepted，但 append-only file baseline 已完成第一轮 Spike。

### Append-only file baseline

当前实现：

- 不新增第三方依赖；
- versioned log header；
- length-prefixed canonical Event record；
- persisted EventId integrity check；
- reopen 时重建 in-memory index；
- duplicate suppression；
- interrupted final record 自动截断到最后完整记录；
- completed-record corruption 返回错误，不静默跳过；
- 每次 append 当前调用 flush + sync_data，优先验证 durability 语义。

同一 CI 构建下：

- seed-core-smoke：389,976 B / 380.8 KiB；
- seed-storage-smoke：398,904 B / 389.6 KiB；
- persistent file storage path 增量：8,928 B / ~8.7 KiB。

这说明文件日志在二进制体积上非常便宜，但仍未证明它在大历史、索引和查询方面优于 SQLite。

### 仍需比较

- SQLite adapter；
- 10k / 100k / 1M Event reopen/rebuild；
- batch durability；
- random recent-history query；
- concurrent reader / writer needs；
- crash fault injection。

优先正确性、crash recovery、跨平台与体积，不为了节省少量体积自行实现一个脆弱数据库。

## Blob / Attachment

大附件不直接塞入 Event Log。

Event 只保存受验证的引用信息，例如：

- content hash；
- size；
- media metadata；
- transfer locator/metadata（按后续协议定义）。

## Alternatives

### Mutable tables as source of truth

优点：

- CRUD 简单。

缺点：

- 治理历史难验证；
- 状态篡改更难发现；
- 断线与离线同步语义更复杂。

### 全量 CRDT-first

优点：

- 能处理复杂离线并发。

缺点：

- MVP 复杂度过高；
- Group / Tree / Governance 的需求并不全部等同于协作文档。

## Validation

- crash / kill recovery；
- duplicate event idempotency；
- invalid event 不进入 accepted history/materialized state；
- rebuild 与在线 materialization 结果一致；
- 10k / 100k event 基线；
- storage binary-size contribution；
- corrupted tail / partial write tests。

## Revisit conditions

- 多设备并发需求证明当前 Event 模型不足；
- 特定 Space 类型需要独立 CRDT；
- backend 在移动端或 Tree Host 上无法满足可靠性/体积目标。
