# Seed Threat Model v0

> Status: Draft  
> 目标：在 MVP 之前明确“Seed 要防什么、不承诺防什么”。

---

## 1. Security goals

Seed Core 应保护：

- Root / Device private keys；
- 消息机密性与完整性；
- Identity continuity；
- Genesis 完整性；
- Event authenticity；
- Governance/Capability 边界；
- Plugin isolation；
- 本地数据；
- Relay path 上的消息内容。

Seed 不应把以下内容错误宣传为安全保证：

- P2P 可以证明现实身份；
- IP 可以证明某个人是谁；
- 自托管可以阻止所有恶意行为；
- 加密可以阻止终端被攻破后的数据泄露。

---

## 2. Assets

### Critical

- Root Private Key
- Device Private Keys
- session keys
- recovery material

### High value

- message history
- contact graph
- Tree membership
- Governance history
- plugin state
- local profile metadata
- network metadata

---

## 3. Trust boundaries

```
User
 |
Client UI
 |
Core ------------------ Plugin Sandbox
 |                           |
Local Storage             Plugin State
 |
Transport ===== untrusted network ===== Peer/Relay/Tree Host
```

边界：

1. UI -> Core；
2. Plugin -> Host ABI；
3. Core -> Local Storage；
4. Core -> Network；
5. Peer -> Peer；
6. Client -> Relay；
7. Client -> Tree Host；
8. Root Key -> Device authorization。

---

## 4. Adversaries

### A1 Malicious remote peer

能力：

- 任意发包；
- replay；
- malformed payload；
- 伪造身份资料；
- 建立大量连接；
- 尝试触发 parser/state bugs。

### A2 Malicious plugin

能力：

- 合法安装后执行任意 Wasm 逻辑；
- 尝试读其他插件数据；
- 尝试读取私钥；
- infinite loop；
- memory exhaustion；
- 发送大量 capability request。

### A3 Malicious Relay

能力：

- drop；
- delay；
- reorder；
- replay encrypted envelopes；
- 观察连接元数据。

不应能够：

- 解密 E2EE message；
- 合法伪造 Device Event。

### A4 Malicious Tree Host

能力：

- 拒绝服务；
- 隐藏事件；
- 延迟事件；
- 提供历史子集；
- 尝试注入自己制造的 event。

不应仅凭 Host 身份获得 Governance capability。

### A5 Compromised device

如果 Device Private Key 已被攻击者取得，Seed 无法神奇阻止其在撤销生效前冒充该 Device。

系统应：

- 支持撤销；
- 限制 Root Key 暴露；
- 支持安全恢复；
- 减少损失范围。

### A6 Local malware / OS compromise

如果整个 OS 已被高权限恶意软件控制，Seed 不能保证消息和按键仍然私密。

这是明确的 out-of-scope boundary，但应尽可能使用 OS secure storage 减少普通文件泄露风险。

---

## 5. Threats and mitigations

### T-01 Identity forgery

Mitigation：

- Root Identity signature；
- Device Certificate；
- signed Events；
- fingerprint verification。

### T-02 MITM on Direct

Mitigation：

- authenticated handshake；
- known Identity binding；
- key change warning；
- forward secrecy target。

### T-03 Replay

Mitigation：

- Event ID / nonce；
- session replay protection；
- duplicate store check。

### T-04 Malicious Event

Mitigation：

- schema；
- signature；
- Device cert；
- Governance/Capability；
- state transition validation。

### T-05 Host privilege escalation

Mitigation：

- Host identity 与 Governance authority 分离；
- 客户端重新验证 event；
- 无 `host_is_admin` shortcut。

### T-06 Plugin key theft

Mitigation：

- Root Key 永不暴露给 plugin ABI；
- Device signing 通过受限 Host service；
- sensitive signing 需要明确 capability/purpose。

### T-07 Plugin filesystem/network escape

Mitigation：

- 无默认完整 WASI；
- explicit capability grants；
- scoped handles；
- runtime sandbox tests。

### T-08 Plugin DoS

Mitigation：

- memory limits；
- fuel/instruction/time budget（runtime 支持方式待定）；
- event/capability rate limits；
- trap isolation。

### T-09 Local database theft

Mitigation target：

- OS secure key store；
- local encryption strategy；
- attachment separation；
- minimal plaintext metadata。

具体 at-rest encryption 在独立 ADR 处理。

### T-10 Metadata leakage

P2P 可能向对方暴露 endpoint。

Mitigation：

- Relay/privacy path；
- endpoint 不进入普通 profile；
- Group 不默认 full mesh；
- plugin 不默认获得 network endpoint。

### T-11 Supply-chain plugin replacement

Mitigation：

- PluginRef package_hash；
- version；
- source/signature/reputation future；
- upgrade must be governance event。

### T-12 Genesis rewrite

Mitigation：

- SpaceId derives from GenesisBody；
- immutable genesis store；
- all later changes use Events。

---

## 6. Abuse vs security

Seed 必须区分：

### Protocol security

Core 负责：

- crypto；
- authentication；
- sandbox；
- block/disconnect；
- rate-limit primitive；
- data integrity。

### Community policy

插件决定：

- spam definition；
- moderation；
- ban policy；
- reports；
- elections；
- roles。

P2P/加密不会从技术上消除诈骗或违法行为，因此产品文案不能把网络结构描述成“自动消除犯罪”。

---

## 7. High-risk implementation surfaces

优先 fuzz/review：

1. CBOR decoder；
2. handshake parser；
3. Event validation；
4. Device Certificate；
5. Plugin Host ABI；
6. package loader；
7. Relay envelope；
8. Tree sync；
9. attachment parser；
10. backup/recovery。

---

## 8. Security gates

### Before MVP

- internal threat model reviewed；
- protocol test vectors；
- parser fuzz baseline；
- plugin escape tests；
- Host != Authority E2E；
- replay tests；
- dependency audit。

### Before public Alpha

- external/independent security review scope；
- plugin package threat review；
- identity recovery review；
- update mechanism review；
- secure build/release pipeline。

---

## 9. Explicit non-goals for v0

v0 不承诺：

- anonymity network；
- traffic-analysis resistance；
- compromised OS protection；
- global Sybil resistance；
- real-world identity verification；
- censorship resistance against a Host that simply shuts down；
- post-quantum security。

这些可以作为未来独立目标，但不能混进 MVP 的安全承诺。
