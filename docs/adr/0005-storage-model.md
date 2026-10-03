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

当前 Core 已经有 append-only in-memory Event Store 与 duplicate suppression。
Phase 0 现在进一步加入了一个 append-only file 候选，用于验证持久化、恢复与体积；
它仍然不是最终 backend 决策。

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

当前最小 Rust interface 只覆盖 Event append / dedup / scan / checkpoint；
Genesis 与 plugin state persistence 仍待后续扩展。

## Phase-0 append-file candidate

当前候选文件格式：

```text
"SEEDLOG1"
repeat {
  event_len: u32 big-endian
  checksum: 32-byte SHA-256(
    "seed:event-store-frame:v1\0" || canonical_event_bytes
  )
  canonical_event_bytes
}
```

其中 `canonical_event_bytes` 是 Event 自身带 wire version 的 deterministic
record；文件 backend 不序列化 Rust struct 内存布局。

当前实现约束：

- duplicate EventId 不重复写入；
- 启动时顺序扫描并重建 in-memory index；
- frame length 在分配 payload buffer 前验证；
- Event wire decode 也有 schema/payload/body 上限；
- checksum mismatch 是 hard failure；
- 非 canonical / invalid Event wire 是 hard failure；
- 已完整写入的重复 Event frame 是 hard failure；
- 默认不自动修剪任何损坏；
- 显式 recovery API 只允许删除**不完整的最终 frame**；
- append 成功前执行 `sync_data`；
- file checksum 只用于 corruption detection，不替代 Device signature authenticity。

这组规则的目标是避免“为了恢复而悄悄吞掉中间损坏”。

## Phase-0 SQLite comparison adapter

PR #8 增加了一个**可选** SQLite adapter，用来和 append-file 在同一 canonical
Event wire 语义下比较，而不是引入第二套协议格式。

当前实现：

- `sqlite-storage` feature 使用系统 SQLite；
- `sqlite-storage-bundled` 仅用于自包含构建/体积对照；
- Event 仍保存 canonical Event wire bytes；
- `event_id` 与 `space_id` 有独立索引列，并在 reopen 时与 Event wire 交叉校验；
- duplicate EventId 保持 idempotent；
- WAL + `synchronous=FULL`；
- 提供按 Space 的 recent-history index/query；
- 为兼容当前 `EventStore` trait，open 时仍加载完整 Event cache，因此这还不是最终 database API。

同一 Linux x86_64 stripped release CI：

- core：390,328 B / 381.2 KiB；
- append-file + Space index：420,880 B / 411.0 KiB，较 core +30,552 B / 29.8 KiB；
- SQLite system-linked：430,656 B / 420.6 KiB，较 core +40,328 B / 39.4 KiB；
- SQLite bundled：1,488,048 B / 1,453.2 KiB，较 core +1,097,720 B / 1,072.0 KiB。

因此 bundled SQLite 不适合作为 Seed 极小默认 Core 的基线；system-linked SQLite
仍然是有竞争力的可选 backend，需要继续用 scale/query/crash/platform 数据比较。

为保证 query 对照可解释，append-file 同样维护按 Space 的 in-memory index，
并提供 bounded recent-history lookup。comparison harness 对两个 backend 使用同一预生成
signed Event corpus，分别记录 durable append、reopen/rebuild、recent-history query 与
persisted bytes。共享 GitHub Actions 仅运行小规模 correctness smoke；用于 ADR 决策的
10k / 100k / 1M 数据必须来自固定 runner / filesystem 的重复测量。

## MVP backend

持久化 backend 尚未 Accepted。

Phase 0 仍需比较：

- SQLite adapter；
- append-only file + index。

append-file 候选的价值是提供一个**零新增依赖**、可测量的下界；
它不能仅凭体积更小就自动胜出。

最终选择优先：

- 正确性；
- crash recovery；
- 跨平台；
- 查询需求；
- 维护风险；
- 体积。

不要为了节省少量体积自行实现一个脆弱数据库。

## Blob / Attachment

大附件不直接塞入 Event Log。

当前 Event payload 有明确上限；大对象应只保存受验证的引用信息，例如：

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

### Append-only file 直接 Accepted

当前拒绝提前做这个决定。

原因：

- 还没有 10k / 100k rebuild 数据；
- recent-history 已有 Space index，但规模数据尚未在固定环境重复测量；
- compaction / index lifecycle 尚未设计；
- SQLite 对 crash consistency、query/index、跨平台工具链可能更有优势；
- Tree Host 与桌面客户端可能最终需要不同 adapter。

## Validation

append-file 候选已经覆盖/正在覆盖：

- duplicate event idempotency；
- restart 后 index rebuild；
- canonical Event decode/re-encode；
- oversized frame rejection before allocation；
- corrupted checksum rejection；
- truncated tail strict rejection；
- explicit final-tail recovery；
- release binary size delta：`seed-core-smoke` 390,328 B / 381.2 KiB，
  `seed-storage-smoke` 420,880 B / 411.0 KiB，delta 30,552 B / 29.8 KiB；
- CI full gate（fmt/check/clippy/tests/protocol smoke/storage smoke/size）通过。

仍需：

- process kill / power-loss style recovery；
- invalid event 不进入 accepted history/materialized state 的完整 acceptance pipeline；
- rebuild 与在线 materialization 结果一致；
- 10k / 100k event rebuild；
- append throughput；
- recent-history query；
- file size；
- SQLite 初始 adapter / reopen / duplicate / indexed recent-history / size 对照 — **DONE**；
- append-file 与 SQLite 的 10k / 100k scale、query、crash/power-loss 同条件对照；
- desktop / Tree Host platform checks。

## Revisit conditions

- 多设备并发需求证明当前 Event 模型不足；
- 特定 Space 类型需要独立 CRDT；
- backend 在移动端或 Tree Host 上无法满足可靠性/体积目标；
- append-file index/rebuild 成本随着真实历史规模失控；
- SQLite 或其他系统 backend 在体积预算内显著降低正确性与维护风险。
