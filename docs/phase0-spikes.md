# Seed Phase 0 Spike Plan

> Status: Active  
> 目标：用可重复实验收敛技术选型，而不是凭感觉冻结依赖。

---

## 1. 已有证据

PR #1 已 squash 合并到 `main`；PR #3 的 std-only persistent FileEventStore baseline 也已进入 `main`。当前分支是在该主线基线上继续强化 Event wire、corruption detection、显式 tail recovery 与可重复 CI 测量。

当前已验证：

- Rust reference Core 可构建；
- `#![forbid(unsafe_code)]`；
- typed Identity / Device / Space / Event / Plugin IDs；
- Ed25519 Root Identity；
- independent Device signing key；
- Root -> DeviceAuthorization；
- strict signature verification；
- SHA-256 domain-separated IDs；
- signed Event；
- Capability primitives + DefaultDeny；
- Genesis Draft -> immutable record；
- Genesis plugin digest pinning；
- canonical plugin ordering；
- deterministic protocol vector；
- canonical Event wire encode/decode；
- append-only in-memory Event Store；
- PR #3 已合并的 std-only append-only FileEventStore baseline；
- 当前强化候选的 bounded Event wire、restart index rebuild、frame checksum corruption detection 与 explicit tail recovery；
- release-size CI。

### 当前 size baseline

Linux x86_64 release / stripped：

| Slice | Size |
|---|---:|
| dependency-free baseline | 282,648 B / 276.0 KiB |
| identity + signing/event | 376,320 B / 367.5 KiB |
| + Genesis canonical state machine | 389,816 B / 380.7 KiB |
| + explicit 128-bit creation nonce | 389,992 B / 380.9 KiB |
| PR #3 persistent file-store baseline | 398,904 B / 389.6 KiB |
| current core smoke（含 bounded Event wire） | 390,328 B / 381.2 KiB |
| current hardened storage smoke + Space index | 420,880 B / 411.0 KiB |

同一 CI commit 下，`seed-storage-smoke` 相对 `seed-core-smoke` 的链接后增量为 **30,552 B / 29.8 KiB**。
这不是最终数据库占用或历史数据文件大小，只是当前 persistent storage code path 的 release binary delta。

Identity + signing + Event + Genesis + 第一版 append-file storage 仍远低于 2 MiB。
当前真正的 size risk 更集中在：

- SQLite 等替代 storage backend；
- transport；
- plugin runtime。

---

## 2. CI 状态

当前 `core-ci` 检查：

- rustfmt；
- cargo check；
- Clippy with `-D warnings`；
- tests；
- protocol vector smoke；
- persistent storage smoke；
- release size（core + storage candidate delta）。

最新完整检查为绿色：31 个 unit tests + 1 个 protocol-vector integration test 通过，且 `seed-storage-smoke` 已实际执行 append -> checkpoint -> reopen/index recovery。

任何 Phase 0 代码都不应通过关闭 warning gate 来“修 CI”。

---

## 3. Spike A — Core language / size

### A0 Empty Rust baseline

**DONE**

- 282,648 bytes / 276.0 KiB。

### A1 Identity / signing

**DONE**

- 376,320 bytes / 367.5 KiB。
- signing layer delta ~91.5 KiB。

### A2 Genesis canonical state machine

**DONE**

- 389,816 bytes / 380.7 KiB。
- Genesis delta ~13.2 KiB。

### A3 Persistent storage

**IN PROGRESS — append-file baseline merged; SQLite comparison adapter CI green**

append-only file + index 候选已经进入实现：

- zero new dependency；
- canonical Event wire records；
- EventId duplicate suppression；
- restart 顺序 scan + in-memory index rebuild；
- u32 frame length 在分配前做上限检查；
- domain-separated SHA-256 frame checksum；
- checksum mismatch hard fail；
- truncated tail 默认 hard fail；
- 显式 recovery 只修复 incomplete final frame；
- persistent storage smoke binary，并在 CI 中实际执行；
- 独立 size delta report：最新同一 build 增量 **30,552 B / 29.8 KiB**。

已完成第一轮 SQLite 对照：

- system-linked：430,688 B / 420.6 KiB（较 core +40,360 B / 39.4 KiB）；
- bundled：1,488,112 B / 1,453.2 KiB（较 core +1,097,784 B / 1,072.1 KiB）；
- reopen / duplicate suppression / corrupted row rejection / Space recent-history index smoke 通过；
- SQLite 保持 optional feature，不进入默认 Core。
- forced process kill 集成测试已通过：worker 在 completed durable appends 后保持 store 打开，父测试直接终止进程；append-file 与 SQLite WAL reopen 后都保留全部已完成 append。该测试不等价于 mid-write / 掉电模拟。

当前 storage comparison harness 已进入主线；本轮进一步统一 query 语义：

- append-file 维护 `SpaceId -> event positions` in-memory index；
- append-file / SQLite recent-history 都使用 bounded indexed lookup（默认 limit 64）；
- `seed-storage-compare` 预生成同一 signed Event corpus，避免把签名成本计入 backend append；
- 输出 durable append、reopen/rebuild、recent-history query 与 persisted bytes；
- PR CI 只跑 256 events correctness smoke，共享 runner wall clock 不作为 backend 性能结论；
- 10k / 100k / 1M 决策数据必须在固定 runner / filesystem 上重复执行，并记录环境与多次样本。

PR #9 的共享 runner 曾完成一次 **10k pre-index sanity run**：Event 生成 55,394 ms（单独计时）；append-file append 3,436 ms / reopen 24 ms / 2,850,008 B；SQLite append 2,345 ms / reopen 12 ms / 4,378,624 B。该结果只证明 10k harness 可运行，不用于 backend 选择；query 当时尚未统一为 indexed recent-history。

仍需记录：

- binary delta — **DONE: +30,552 B / 29.8 KiB**（Linux x86_64 stripped release）；
- append throughput；
- forced process kill after completed durable appends — **DONE (append-file + SQLite)**；
- mid-write / torn-write / real power-loss fault injection；
- file/db size；
- recent-history query；
- 10k / 100k event rebuild；
- append-file / SQLite 10k、100k 同条件 scale/query 数据；
- append-file / SQLite mid-write / torn-write / power-loss fault-injection 对照。

在这些数据完成前，不把 append-file 标记为 Accepted。

---

## 4. Spike B — Canonical encoding hardening

当前采用窄的 custom canonical binary encoding。

### 已完成

- fixed-width big-endian primitives；
- length-prefixed bytes；
- Genesis canonical ordering；
- duplicate plugin rejection；
- canonical Genesis decode/re-encode check；
- canonical Event decode/re-encode check；
- Event schema/payload/body size bounds；
- Event wire deterministic vector；
- deterministic vector。

### TODO

- malformed/oversized corpus；
- fuzz decoder；
- independent implementation；
- cross-language vector；
- schema evolution experiment。

### Gate

在独立实现与 fuzz 之前，ADR-0003 保持 Provisional。

---

## 5. Spike C — Identity / Crypto

### 已完成

1. Create Root；
2. Create Device；
3. Root signs DeviceAuthorization；
4. Device signs Event；
5. peer verifies chain；
6. deterministic vector。

### TODO

- Device revocation；
- Root key recovery/rotation；
- persistent secret store adapter；
- transport KEX；
- MITM tests；
- replay tests；
- key lifecycle threat review。

---

## 6. Spike D — Plugin Runtime

**TODO / major size risk**

候选至少两个 runtime。

每个 candidate：

### D0 Hello

```
on_load -> success
```

### D1 Host ABI

实现：

- scoped state get/put；
- event subscription stub；
- capability request stub。

### D2 Resource isolation

测试：

- trap；
- infinite loop；
- memory growth；
- OOM；
- excessive host calls。

### D3 Permission escape

插件尝试：

- filesystem；
- network；
- Root key；
- foreign plugin state；
- undeclared capability。

### Measure

- Host stripped size delta；
- plugin package size；
- instantiate latency；
- call throughput；
- peak RSS；
- resource limit enforcement。

---

## 7. Spike E — Transport

**IN PROGRESS — Loopback abstraction merged; TCP / Relay pending**

先建立统一 abstraction，再实现：

```
LoopbackTransport
TcpTransport
RelayTransport
```

测试：

- direct；
- direct failure；
- relay fallback；
- reconnect；
- duplicate frame；
- truncated frame；
- malicious length；
- oversized message。

上层 Direct session 不允许依赖具体 adapter 类型。

---

## 8. Spike F — Governance Kernel

### 基础能力

**PARTIAL**

已有：

- typed Capability；
- DefaultDeny；
- no implicit creator authority test。

### TODO

测试完整链：

1. Empty Group；
2. Alice 请求 MemberRemove -> Deny；
3. Owner Governance plugin；
4. Alice -> Allow；
5. Bob -> Deny；
6. Voting mock；
7. request -> Pending；
8. resolution -> Allow；
9. install/remove plugin 本身也经过 governance。

如果必须在 Core 写 `role == owner`，Spike 失败。

---

## 9. Spike G — Genesis uniqueness / lifecycle

### 已完成

- Draft；
- plugin attach；
- canonical sort；
- duplicate reject；
- Device signature；
- immutable record；
- GenesisId；
- SpaceId；
- package digest pin；
- creator provenance only。

### Explicit creation nonce

**DONE**

- Genesis schema/wire 已升级到 v2；
- 加入 128-bit creation nonce；
- 完全相同 config + 同一 timestamp + 不同 nonce -> 不同 SpaceId；
- nonce 被 canonical bytes 与签名覆盖；
- protocol vector 已升级到 v2；
- smoke binary 仅增加约 176 B。

---

## 10. Size Measurement Contract

继续使用可重复的 Linux x86_64 release baseline：

- `opt-level=z`；
- fat LTO；
- one codegen unit；
- `panic=abort`；
- stripped symbols；
- system/dynamic dependencies单独报告。

分别报告：

```
seed-core smoke
seed-storage smoke
tree host
relay
plugin runtime host
sample plugin wasm
fresh-install footprint
```

不要用压缩包大小替代 executable size。

---

## 11. Phase 0 Completion Checklist

- [x] Rust baseline
- [x] Identity / Device signing path
- [x] Event signature + deterministic ID
- [x] Genesis state machine
- [x] Genesis package digest pinning
- [x] protocol vector
- [x] release size CI
- [x] DefaultDeny capability baseline
- [x] Genesis explicit nonce
- [ ] Device revocation
- [ ] canonical decoder fuzzing
- [ ] independent/cross-language vector
- [x] append-file persistent Event Store candidate
- [ ] persistent storage scale/crash comparison
- [x] Loopback Transport abstraction
- [ ] TCP / Relay Transport adapters
- [ ] Relay fallback
- [ ] Plugin Runtime comparison
- [ ] Plugin sandbox escape tests
- [ ] Governance Allow/Deny/Pending vertical slice
- [ ] Multi-node smoke framework
- [ ] Threat Model review against implementation

---

## 12. 下一批实现优先级

当前最合理的顺序：

```
1. persistent storage 10k / 100k / 1M reopen/rebuild benchmark + SQLite 同条件对照
2. Transport abstraction + Loopback — **DONE**
3. TCP Direct + Relay fallback
4. Plugin Runtime comparison
5. Governance vertical slice
6. Device revocation / key lifecycle
7. Multi-node smoke framework
8. Tree Host
```

不要先做复杂 UI、语音视频或插件市场。
