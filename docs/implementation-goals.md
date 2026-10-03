# Seed 实现目标与开发路线

> 文档状态：Draft v0.1  
> 目标：把 Seed 从架构概念推进到可验证 MVP，再推进到可供第三方插件开发的 Alpha。

---

## 1. 实现总目标

第一阶段实现不是“做一个漂亮聊天客户端”，而是证明以下命题：

1. 无中心账号服务也能建立稳定密码学身份；
2. Direct 可以优先 P2P 并支持 Relay fallback；
3. Group 可以在完全没有群主/管理员的情况下工作；
4. 同一个 Group Core 可以通过插件变成传统群主制；
5. Tree 可以由用户 Host，但 Host 不自动成为社会 Owner；
6. Genesis 可以可靠定义初始制度；
7. Governance Plugin 可以控制 Capability；
8. 插件有真实安全边界；
9. Core 仍然保持小体积和低依赖；
10. 整套系统能够通过自动化多节点测试验证。

---

## 2. 实现原则

### I-01：先做协议骨架，再做丰富 UI

先证明：

- identity；
- event；
- space；
- capability；
- plugin；
- transport。

UI 只覆盖必要路径。

### I-02：每个阶段必须可运行

避免“六个月后第一次集成”。

每个里程碑应产生一个可运行的纵向切片。

### I-03：先做单平台参考实现

MVP 优先一个桌面平台 + headless Tree Host。

协议与模块边界保持跨平台，但不在 MVP 同时承担所有平台适配。

### I-04：依赖引入需要预算

每个重量级依赖进入 Core 前需要说明：

- 必要性；
- 替代方案；
- 二进制体积；
- 安全维护成本。

### I-05：安全功能不因体积预算删除

2 MB 是目标，不是绕过安全的理由。

---

## 3. 建议仓库结构

初步建议：

```
seed/
├── README.md
├── docs/
│   ├── PRD.md
│   ├── architecture.md
│   ├── design-goals.md
│   ├── implementation-goals.md
│   └── adr/
├── core/
│   ├── identity/
│   ├── crypto/
│   ├── event/
│   ├── space/
│   ├── capability/
│   ├── plugin/
│   ├── storage/
│   ├── sync/
│   └── transport/
├── host/
│   └── tree/
├── client/
│   └── desktop/
├── plugins/
│   ├── governance-owner/
│   ├── governance-community/
│   └── voting/
├── tests/
│   ├── protocol/
│   ├── multi-node/
│   └── e2e/
└── tools/
    └── size-report/
```

这是目标形态，不要求立即创建所有空目录。

---

## 4. Phase 0：技术决策与 Spike

### 目标

在正式写大量业务代码前解决高风险技术问题。

### 当前 ADR 序列

- ADR-0001：Core language / Rust spike；
- ADR-0002：Root and Device signing keys；
- ADR-0003：Narrow canonical binary encoding；
- ADR-0004：Plugin Runtime / Wasm Host ABI；
- ADR-0005：Validated Event Log / Storage；
- ADR-0006：Direct-first Transport；
- ADR-0007：Genesis root / Validated Event。

其中 ADR-0001 ~ 0003 已经随 PR #1 合并到 `main` 并有真实实现与测量，其余仍需后续 Spike。

### Exit Criteria

Phase 0 完成必须有：

- 一个空 Core release binary；
- crypto + serialization + storage 最小组合 size 数据；
- plugin hello-world size 数据；
- transport hello-peer 可运行；
- 5 份左右 ADR；
- 2 MB 目标是否现实的第一次证据。

---

## 5. Phase 1：Identity + Event Kernel

### 目标

建立最小可信内核。

### 实现

#### Identity

- Root Identity；
- Device Identity；
- device authorization；
- fingerprint；
- local key storage adapter。

#### Crypto

- signatures；
- authenticated encryption primitives；
- key derivation；
- secure random。

#### Event

- Event ID；
- author/device；
- payload；
- signature；
- validation；
- deduplication。

#### Storage

- append event；
- query by space；
- materialized state 基础。

### Demo

两个本地进程：

1. 各自创建 Identity；
2. 交换 public identity；
3. A 签事件；
4. B 验证；
5. B 存储并重启后仍可读取。

### Exit Criteria

- 无网络服务器依赖；
- 私钥不出 Identity/Crypto 边界；
- malformed event fuzz/basic tests；
- deterministic serialization test；
- release size 自动记录。

---

## 6. Phase 2：Direct Messaging

### 目标

完成第一个真实纵向产品路径。

### 实现

- contact relation；
- authenticated handshake；
- secure session；
- text message event；
- local history；
- send status；
- direct transport；
- relay fallback；
- block/disconnect。

### 第一版 Relay

Relay 只需要证明架构：

- 接收密文 envelope；
- 按目标标识暂存/转发；
- 不拥有 Direct Space；
- 无明文消息处理。

不追求生产级全球 Relay 网络。

### Demo

两台机器或两个隔离网络进程完成：

```
Identity exchange
-> connect
-> authenticated session
-> encrypted message
-> disconnect
-> reconnect
-> history available
```

### Exit Criteria

- Wireshark/测试代理无法看到消息明文；
- Relay 路径与 Direct 路径使用同一上层消息语义；
- 更换错误身份密钥时握手失败；
- Block 后不继续接受正常会话流量。

---

## 7. Phase 3：Space + Group + Genesis

### 目标

验证“创建者不是天然 Owner”。

### 实现

#### Space

- Space ID；
- kind；
- membership；
- events；
- plugin attachments。

#### Genesis

- Draft；
- initial plugins；
- canonical record；
- signature；
- hash；
- immutable storage。

#### Group

- create；
- join；
- member list；
- group message。

### 必做 Demo A：Empty Group

Alice 创建 Group：

- 不安装 Governance；
- Bob、Carol 加入；
- 三人聊天；
- Alice 不具有 member.remove。

### 必做 Demo B：Owner Group

Alice 在 Genesis 前安装 Owner Governance：

- Alice 成为插件定义 Owner；
- Alice 可以 remove Bob；
- Carol 不可以；
- Core Group 代码与 Empty Group 使用同一实现。

### Exit Criteria

- Core 搜索不到业务角色判断，例如 `if role == owner`；
- creator bypass 不存在；
- Genesis hash 可跨进程一致；
- 激活后不能 update Genesis file 替代治理事件。

---

## 8. Phase 4：Capability + Plugin Runtime

### 目标

把治理逻辑真正移出 Core，而不是“名义插件化”。

### 实现

#### Capability Engine

至少：

- request；
- evaluate；
- allow；
- deny；
- pending；
- execute；
- audit event。

#### Plugin Runtime

- manifest load；
- permission check；
- scoped storage；
- event subscription；
- capability hook；
- lifecycle；
- crash isolation 基础。

#### Basic Owner Governance

插件负责：

- owner state；
- member.remove authorization；
- plugin management authorization。

### 安全测试

恶意插件尝试：

- 读取 Root Private Key；
- 访问其他插件 storage；
- 直接修改 Group membership；
- 调用未声明 Capability。

都必须失败。

### Exit Criteria

- 移除 Governance Plugin 后，Core 本身不再提供 Owner 权限；
- 所有 member.remove 流程经过 Capability；
- 未声明权限有自动化拒绝测试；
- 插件 panic/crash 不导致 Core 状态损坏。

---

## 9. Phase 5：Tree Host

### 目标

验证自托管社区模型。

### 实现

#### Tree Host

- headless process；
- Tree Genesis；
- durable storage；
- branch/channel；
- membership；
- event routing；
- plugin host runtime。

#### Client

- add Tree endpoint/invite；
- inspect governance summary；
- join；
- browse branches；
- text channel。

### Host / Authority Test

必须建立一个专门测试：

1. Host 进程由账户 X 控制；
2. Governance 中 Primary Administrator 是 Y；
3. X 尝试产生管理员-only event；
4. 客户端验证后拒绝；
5. Y 的合法事件通过。

这不能阻止恶意 Host 拒绝服务，但能证明协议没有把 Host 身份等价于管理员身份。

### Exit Criteria

- Tree 可在另一台机器自托管；
- 客户端不依赖 Seed 官方 Tree 服务；
- Host restart 后 Tree 恢复；
- governance summary 可在加入前读取；
- Host ≠ Authority 测试通过。

---

## 10. Phase 6：Community Governance + Voting

### 目标

验证 Pending Capability 与 Meta-Governance。

### 实现

- proposal；
- vote；
- threshold；
- deadline；
- execution；
- recall Primary Administrator；
- plugin.install proposal。

### Demo

Community Tree：

1. Alice 初始为 Primary Administrator；
2. Bob 发起 recall；
3. 成员投票；
4. 达到阈值；
5. Capability 执行；
6. Alice 权限消失；
7. 新主管理人产生或进入预设继任流程。

另一套 Organization Governance：

- member vote 不能触发 recall；
- Root/organization authority 才能更换。

### Exit Criteria

- 两种制度不需要修改 Tree Core；
- 投票是 Plugin state，不是 Core 特判；
- plugin.install 本身可以由 Governance 控制。

---

## 11. Phase 7：MVP Client Polish

### 目标

让非开发者能够实际完成核心流程。

### UI 最小范围

- onboarding；
- identity card/fingerprint；
- Direct list；
- Group list；
- Tree list；
- message view；
- member view；
- plugin view；
- create space wizard；
- Genesis Bundle selection；
- governance summary；
- plugin permission dialog；
- Tree host setup 基础指引。

### 明确不做

- 复杂主题系统；
- 动画；
- 社交 feed；
- 语音视频；
- sticker marketplace；
- 大型插件商店。

### Exit Criteria

可用性测试用户可以在无口头解释下完成：

- 加联系人；
- 发 Direct；
- 创建普通 Group；
- 创建 Owner Group；
- 加入 Tree；
- 看懂“谁可以修改规则”的摘要。

---

## 12. Phase 8：Alpha Plugin SDK

### 目标

允许非 Core 开发者写出第一个第三方插件。

### 输出

- Plugin SDK；
- manifest schema；
- API reference；
- example plugin；
- testing harness；
- compatibility policy；
- packaging format；
- signing/reputation 的初步方案。

### Alpha Gate

第三方开发者应能在不修改 Seed 仓库的情况下实现一个插件，例如：

- Poll；
- Welcome message；
- Simple moderation；
- Game command。

---

## 13. 测试目标

### Unit Tests

覆盖：

- identity；
- crypto wrapper；
- serialization；
- capability；
- plugin permission；
- state reducers。

### Protocol Tests

固定 test vectors：

- Identity；
- Genesis；
- Event；
- Signature；
- Capability request。

### Multi-node Simulation

至少模拟：

- 2-node Direct；
- 3-node Group；
- node offline/rejoin；
- duplicate event；
- malicious unauthorized event；
- Relay path。

### E2E

运行真正的：

- client A；
- client B；
- Tree Host；
- plugin runtime。

---

## 14. 安全实现目标

MVP 前必须完成至少一次内部 threat model。

覆盖：

- key theft；
- malicious peer；
- malicious plugin；
- malicious Tree Host；
- replay；
- event forgery；
- downgrade；
- plugin supply chain；
- local database theft；
- IP/network metadata exposure。

进入公开 Alpha 前应安排独立安全审查或外部审计范围。

---

## 15. 性能与资源目标

第一阶段先建立可测基线，再逐步收紧。

建议持续记录：

- binary size；
- idle memory；
- startup time；
- message encode/decode time；
- handshake latency；
- local DB open time；
- plugin cold-start；
- Tree event throughput。

不要在没有基线的情况下提前做大量微优化。

---

## 16. 2 MB Size Budget

建议把 2 MB 拆成预算，而不是只在最后看总数。

示例目标（非最终承诺）：

```
identity + crypto wrappers     <= 250 KB
event + serialization          <= 200 KB
space + capability             <= 250 KB
storage adapter                <= 250 KB
transport core                 <= 350 KB
plugin runtime interface       <= 350 KB
misc / glue / error handling   <= 350 KB
---------------------------------------
target                         <= 2 MB
```

实际数据以工具测量为准。

如果某依赖单独增加数 MB，需要重新讨论：

- 是否系统提供；
- 是否动态可选；
- 是否移动到扩展包；
- 是否值得突破目标。

---

## 17. CI / 工程质量目标

每次主分支提交至少执行：

- formatting；
- lint；
- unit tests；
- protocol vectors；
- multi-node smoke test；
- dependency audit；
- release size report。

后续增加：

- fuzz；
- sanitizers；
- cross-platform builds；
- plugin compatibility suite。

---

## 18. Definition of Done

一个 Core feature 完成必须同时满足：

1. 有接口定义；
2. 有单元测试；
3. 有错误路径；
4. 不绕过 Capability / Permission 边界；
5. 文档更新；
6. release size 变化已知；
7. 如果影响协议，有 test vector；
8. 如果影响安全边界，有对应威胁分析。

仅“UI 上能点”不算完成。

---

## 19. MVP Release Gate

MVP 发布候选必须满足：

### Identity
- 可创建；
- 可备份基础材料；
- 设备身份可验证。

### Direct
- P2P；
- Relay fallback；
- E2EE；
- 本地历史。

### Group
- Empty Governance；
- Owner Governance；
- Genesis。

### Tree
- self-hosted；
- branch/channel；
- Community Governance；
- governance summary。

### Plugin
- sandbox；
- manifest；
- capability；
- scoped storage。

### Quality
- automated multi-node test；
- threat model；
- size report；
- crash recovery 基础；
- protocol docs。

---

## 20. Alpha 之后的候选方向

只有 MVP 骨架稳定后再评估：

- mobile；
- multi-device full sync；
- voice/video plugin；
- richer file transfer；
- Tree migration；
- federation；
- plugin registry；
- plugin reputation/signing；
- advanced moderation；
- bots；
- AI plugins；
- games；
- payment-related plugins（如未来涉及真实金融能力，需单独处理其安全与合规边界）。

---

## 21. 当前实现进度与下一步

截至当前 `main`：

- Rust baseline 已完成；
- Identity / DeviceAuthorization 已完成第一版；
- signed Event 与 deterministic IDs 已完成；
- Genesis canonical state machine 已完成；
- plugin package digest pinning 已完成；
- Capability DefaultDeny 已存在；
- deterministic protocol vector 已存在；
- release size CI 已存在；
- Genesis + 128-bit creation nonce 后 smoke binary 实测约 380.9 KiB；
- PR #1 已 squash 合并，Core Kernel 已进入主分支。

下一批优先实现：

1. persistent Event Store（SQLite vs append-file）；
2. Transport abstraction + Loopback；
3. TCP Direct + Relay fallback；
4. Plugin Runtime / Wasm engine size + sandbox comparison；
5. Governance Allow / Deny / Pending 纵向切片；
6. Device revocation / key lifecycle；
7. Multi-node smoke framework；
8. Tree Host。

项目现在已经从“产品理念”进入“可执行工程 Spike”，但尚未进入面向用户的 MVP 功能堆叠阶段。
