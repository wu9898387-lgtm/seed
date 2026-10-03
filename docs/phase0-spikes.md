# Seed Phase 0 Spike Plan

> 目标：用可重复实验决定技术选型，而不是凭感觉冻结依赖。

---

## 1. 输出物

Phase 0 结束时必须得到：

- Core Language ADR -> Accepted / Rejected；
- Plugin Runtime ADR -> engine selected；
- Serialization ADR -> Accepted；
- Storage backend recommendation；
- Transport reference adapter；
- Identity/Crypto profile test vectors；
- baseline binary-size report；
- baseline startup/memory report。

---

## 2. Spike A — Rust Core Size

### A0 Empty

功能：

- CLI starts；
- version string；
- no third-party deps。

记录：

- unstripped；
- stripped；
- compressed artifact（仅参考，不算 Core size）；
- startup；
- RSS。

### A1 Serialization

加入：

- deterministic CBOR encoder/decoder；
- one Genesis vector。

### A2 Crypto

加入：

- SHA-256；
- Ed25519 verify/sign；
- X25519 primitive or selected handshake library；
- AEAD primitive。

### A3 Storage

分别构建：

- SQLite variant；
- append-file variant。

### Acceptance

最终 size 以 A3 + minimal transport + plugin interface 的整合构建判断，而不是 Empty binary。

---

## 3. Spike B — Wasm Runtime

候选最少两个。

每个 candidate 构建：

### B0 Hello

Plugin:

```
on_load -> return 0
```

### B1 Host call

循环调用 scoped state read / capability request stub。

### B2 Trap

- panic/trap；
- infinite loop；
- memory growth。

### B3 Permission escape

尝试：

- filesystem；
- network；
- Root Key；
- foreign plugin state。

### Measure

- Host binary size delta；
- plugin package size；
- instantiate latency；
- call throughput；
- peak RSS；
- limit enforcement；
- platform support。

### Gate

任何 runtime 只要无法可靠限制插件资源，就不能因为“更小”而被接受。

---

## 4. Spike C — Canonical CBOR

建立至少 10 个 vector：

- empty/minimal identity；
- unicode profile-adjacent data；
- Genesis；
- plugin refs；
- integer boundaries；
- map ordering；
- malformed indefinite form；
- duplicate keys；
- unknown extension；
- oversized input。

要求：

- bytes deterministic；
- invalid/noncanonical forms policy 明确；
- fuzz decoder。

---

## 5. Spike D — Identity / Crypto

场景：

1. Create Root；
2. Create Device；
3. Root signs Device Certificate；
4. Device signs Event；
5. Peer verifies chain；
6. Revoke Device；
7. same Device event rejected under updated revocation state。

Handshake：

- first-contact profile；
- known-contact profile；
- MITM negative test；
- replay negative test。

---

## 6. Spike E — Storage

数据集：

- 10k events；
- 100k events；
- 1M events（如果本地实验成本合理）。

测量：

- append；
- cold rebuild；
- query recent messages；
- plugin state lookup；
- crash recovery；
- binary size delta；
- DB/file size。

重点不是追求极限 benchmark，而是排除明显不适合 MVP 的方案。

---

## 7. Spike F — Transport

实现统一 trait/interface 后，写两个 adapter：

```
LoopbackTransport
TcpTransport
```

再实现：

```
RelayTransport
```

测试：

- direct；
- forced direct failure；
- relay fallback；
- reconnect；
- duplicate frame；
- truncated frame；
- malicious length；
- message > limit。

上层 Direct session 不允许知道具体 adapter 类型。

---

## 8. Spike G — Governance Kernel

不需要 UI。

在测试中：

1. 创建 Empty Group；
2. Alice 请求 `seed.member.remove`；
3. Deny；
4. 创建 Owner Plugin Group；
5. Alice -> Allow；
6. Bob -> Deny；
7. 创建 Voting mock；
8. request -> Pending；
9. resolution event -> Allow。

如果为了实现这些场景必须在 Core 写 `role == owner`，Spike 失败。

---

## 9. Size Measurement Contract

Phase 0 必须先固定测试口径。

建议 Linux x86_64 baseline：

- release；
- LTO；
- panic abort；
- stripped；
- dynamic/system libraries单独报告；
- debug symbols 不计入；
- plugins 不计入 Core binary；
- optional Tree Host binary 单独报告。

同时报告：

```
core executable
tree host executable
relay executable
sample plugin wasm
total fresh-install footprint
```

不要用压缩包大小替代 executable size。

---

## 10. Phase 0 Completion Checklist

- [ ] ADR-0001 有真实 size 数据
- [ ] ADR-0002 runtime 已比较
- [ ] ADR-0003 vectors 通过
- [ ] ADR-0004 storage comparison 完成
- [ ] ADR-0005 transport adapters 通过
- [ ] ADR-0006 crypto vectors 通过 review
- [ ] ADR-0007 Genesis/Event vectors 完成
- [ ] Threat Model review
- [ ] CI size report
- [ ] Multi-node smoke test framework
- [ ] 下一阶段 backlog 已从实验结果更新
