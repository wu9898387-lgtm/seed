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

当前 `main` 已经有 append-only in-memory Event Store 与 duplicate suppression，可用于证明基本接口，但还不是持久化方案。

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

持久化 backend 尚未 Accepted。

Phase 0 比较：

- SQLite adapter；
- 简单 append-only file + index。

优先正确性、crash recovery、跨平台与体积，不为了节省少量体积自行实现一个脆弱数据库。

### 当前 append-only file spike

`phase0/persistent-event-store-spike-20261003` 增加了最小 `FileEventStore`，用于先验证文件日志这一候选方向的底层语义，而不是提前决定最终 backend。

当前 spike 明确实现：

- 独立文件头与版本化 record framing；
- EventId 幂等去重；
- append 后 `sync_data`；
- 重启后扫描恢复；
- 恢复时重新计算 content-derived EventId；
- 末尾长度前缀半写 / record 半写时截断到最后完整记录；
- 完整但损坏的 record fail-closed，不静默跳过；
- 单 record 16 MiB 上限，避免损坏长度导致无界分配。

该 on-disk framing 是 **storage implementation detail**，不是 Seed network wire format，也不冻结 Event 协议编码。

仍未完成：

- SQLite 对照实现；
- 10k / 100k Event benchmark；
- crash/kill 注入矩阵；
- fsync 策略与批量提交权衡；
- index / recent-history query；
- materialized view checkpoint；
- 插件 state namespace；
- hostile local disk tampering 后的全链签名重验证流程。

因此 ADR 状态保持 **Proposed**。

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
