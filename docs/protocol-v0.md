# Seed Protocol v0 草案

> Status: Draft / experimental  
> 本文档用于 Phase 0/1 test vectors。字段与编码在相关 ADR Accepted 前都不视为稳定公开协议。

---

## 1. 目标

Protocol v0 只解决最小内核问题：

- Identity；
- Device authorization；
- Genesis；
- Space ID；
- Signed Event；
- Capability request/result；
- transport envelope 的最小抽象。

不在 v0 同时解决：

- 完整 federation；
- 全功能 CRDT；
- voice/video；
- payment；
- plugin marketplace；
- 大规模群拓扑。

---

## 2. 编码

协议签名对象使用 Deterministic CBOR。

v0 规则：

- 不允许浮点字段；
- map keys 使用 unsigned small integer；
- definite length only；
- integer 使用最短编码；
- UTF-8 text；
- hash/signature 输入使用 canonical bytes；
- object type 使用 domain separation。

任何实现都必须提供同一组 test vectors。

---

## 3. 标识长度

v0 暂定：

- IdentityId: 32 bytes；
- DeviceId: 32 bytes；
- SpaceId: 32 bytes；
- EventId: 32 bytes；
- PluginId: UTF-8 namespace string（进入稳定版前再次评估）。

二进制协议中不得默认使用 hex 字符串保存上述 ID；hex/base32 主要用于 UI 和调试。

---

## 4. Root Identity

概念对象：

```text
RootIdentityPublic {
  1: protocol_version
  2: root_signing_public_key
  3: created_at
}
```

Proposed：

```
IdentityId = SHA-256(
  "seed:v0:identity-id" ||
  deterministic_cbor(RootIdentityPublic)
)
```

昵称、头像、简介不进入 Root Identity ID。

它们属于可变 Profile 数据。

---

## 5. Device Certificate

```text
DeviceCertificateBody {
  1: protocol_version
  2: identity_id
  3: device_id
  4: device_signing_public_key
  5: device_kex_public_key
  6: issued_at
  7: expires_at? 
}
```

```text
DeviceCertificate {
  1: body
  2: root_signature
}
```

Root Signature 覆盖：

```
domain("seed:v0:device-cert") ||
deterministic_cbor(DeviceCertificateBody)
```

DeviceId 的最终计算方式待 identity spike 固定。

---

## 6. Device Revocation

v0 需要可表达：

```text
DeviceRevocationBody {
  1: protocol_version
  2: identity_id
  3: device_id
  4: revoked_at
  5: reason_code?
}
```

由 Root Identity 签名。

客户端必须区分：

- certificate cryptographically valid；
- certificate currently accepted under known revocation state。

离线节点可能不知道最新撤销，因此 UI/协议不能声称“永远实时知道设备是否失效”。

---

## 7. Genesis

```text
GenesisBody {
  1: protocol_version
  2: space_kind
  3: nonce
  4: creator_identity
  5: initial_members
  6: initial_plugins
  7: initial_plugin_config
  8: host_descriptor?
  9: created_at
}
```

约束：

- `nonce` 至少提供足够随机熵；
- `host_descriptor` 只用于需要长期 Host 的 Space；
- role/owner/admin 不属于固定 Genesis Core 字段；
- 如果需要创建者成为 Owner，由 plugin config 表达。

Proposed：

```
SpaceId = SHA-256(
  "seed:v0:space-id" ||
  deterministic_cbor(GenesisBody)
)
```

```text
Genesis {
  1: body
  2: creator_device_certificate
  3: creator_signature
}
```

Genesis Signature：

```
domain("seed:v0:genesis") ||
deterministic_cbor(GenesisBody)
```

---

## 8. Space Kind

v0 Core 识别最小结构类型：

```
0 = Direct
1 = Group
2 = Tree
3 = Branch
4 = Channel
```

这里的 kind 只表达结构/网络语义。

它不表达：

- 社会身份；
- 社区类型；
- 公司/民主；
- 管理模式。

---

## 9. Event

```text
EventBody {
  1: protocol_version
  2: space_id
  3: author_identity
  4: author_device
  5: nonce
  6: kind
  7: payload
  8: parents_or_context
  9: created_at
}
```

```text
SignedEvent {
  1: body
  2: signature
}
```

Proposed：

```
EventId = SHA-256(
  "seed:v0:event-id" ||
  deterministic_cbor(EventBody)
)
```

签名：

```
domain("seed:v0:event") ||
deterministic_cbor(EventBody)
```

`created_at` 用于展示/辅助排序，不能单独作为授权依据。

---

## 10. Event Validation Pipeline

收到 Event 后必须按明确 pipeline 验证：

```
decode
 -> deterministic/schema validation
 -> Space exists
 -> Device certificate validation
 -> signature validation
 -> duplicate/replay check
 -> Capability/Governance authorization
 -> state-transition validation
 -> append accepted event
 -> materialize state
```

未通过的 Event：

- 不进入 authoritative accepted log；
- 不改变 materialized state；
- 可以进入有限、安全的 diagnostics/quarantine，但不能与合法历史混淆。

---

## 11. Event Kind

Core v0 只保留最小事件 namespace，例如：

```
seed.message.text
seed.member.invite
seed.member.join
seed.member.leave
seed.capability.intent
seed.capability.resolution
seed.plugin.install
seed.plugin.remove
seed.plugin.configure
seed.space.child.create
```

“管理员任命”等不应作为 Core 必需 event kind。

治理插件可拥有自己的 namespaced events。

---

## 12. Capability Request

```text
CapabilityRequest {
  1: request_id
  2: space_id
  3: actor_identity
  4: actor_device
  5: capability
  6: resource
  7: parameters
  8: context
}
```

Evaluation：

```
Allow
Deny
Pending(reference)
```

Pending 可以映射到：

- vote；
- multisig；
- approval；
- delay/timelock。

Core 不内置“投票”概念。

---

## 13. Plugin Attachment

Genesis 或运行中事件引用插件时至少要固定：

```text
PluginRef {
  1: plugin_id
  2: version
  3: package_hash
  4: api_version
}
```

这样治理历史不会因为“同名插件后来内容变化”而失去可验证性。

插件升级必须是新事件，不允许静默替换 package bytes。

---

## 14. Transport Envelope

Transport framing 与 Event schema 分开。

最小 envelope 概念：

```text
Envelope {
  protocol_version
  session
  type
  payload_ciphertext
}
```

Direct 与 Relay 路径不改变内部已认证/加密的 Seed session message 语义。

Relay 不需要解析聊天 Event payload。

---

## 15. Tree Host

客户端从 Tree Host 收到 Event 时仍执行正常验证。

不能出现：

```
if source == tree_host:
    trust_without_governance_check()
```

Host 可以：

- 路由；
- 存储；
- 提供 durable history；
- 拒绝服务。

Host 不能仅因为是 Host 就自动制造合法 Governance Event。

---

## 16. Error Philosophy

协议错误应可分类：

- malformed；
- unsupported-version；
- invalid-signature；
- unknown-identity；
- revoked-device；
- duplicate-event；
- unauthorized；
- invalid-transition；
- plugin-unavailable；
- plugin-incompatible。

网络失败与治理拒绝必须是不同错误类型。

---

## 17. Test Vectors

正式进入 Phase 1 前建立：

```
tests/vectors/
  identity/
  device-cert/
  genesis/
  event/
  capability/
```

每个 vector 至少有：

- human-readable source；
- canonical CBOR hex；
- expected hash/id；
- signature；
- expected validation result。

---

## 18. Protocol v0 Exit Criteria

v0 可以进入实现冻结的条件：

1. Deterministic CBOR test vector 跨至少两个独立 encoder 路径一致；
2. Identity/Device test vectors 完成；
3. Genesis/SpaceId vectors 完成；
4. Event validation pipeline 有自动化测试；
5. Host-forged governance Event 被拒绝；
6. creator bypass 不存在；
7. PluginRef package_hash 进入历史；
8. version negotiation 最小方案明确。
