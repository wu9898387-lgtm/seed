# Seed Protocol Kernel Draft

> Status: Experimental / not frozen  
> 当前实现分支：`core-kernel-spike`  
> 当前 Core protocol constant：v1

本文档描述“目前正在实现和验证的协议内核”，不是公开稳定协议承诺。

---

## 1. 当前目标

最小协议内核只覆盖：

- Root Identity；
- Device Authorization；
- Genesis；
- Space identity；
- signed Event；
- Capability primitives；
- Plugin references；
- accepted Event history 的基础接口。

暂不在同一阶段解决：

- 完整 P2P NAT traversal；
- persistent storage 最终选型；
- plugin sandbox engine；
- multi-device full sync；
- Tree federation；
- voice/video；
- payment；
- plugin registry。

---

## 2. 当前 canonical encoding

安全关键的签名对象当前采用一个**窄的、schema-specific canonical binary encoding**。

当前规则：

- fixed-width integers 使用 big-endian；
- variable bytes 使用 u32 length prefix；
- signed kernel schema 不使用浮点；
- signed kernel schema 不使用 unordered map；
- set-like collection 必须有明确 canonical sort；
- Genesis plugins 按 PluginId 排序；
- duplicate PluginId 非法；
- decoder 拒绝 trailing bytes、unsupported version、invalid enum、oversized input 和 non-canonical order；
- hash/signature 使用 domain separation。

这是 Phase-0/1 的实现选择，不是对所有插件 payload、UI state 或 RPC 的格式要求。

在公开协议冻结前仍需：

- fuzz；
- 独立第二实现；
- cross-language vector；
- schema evolution review。

---

## 3. Root Identity

当前 Root Identity 使用 Ed25519 signing key。

```
IdentityId = SHA-256(
  "seed:identity-id:v1\0" ||
  root_public_key
)
```

Profile 数据，例如：

- nickname；
- avatar；
- description；

不进入 IdentityId。

它们是可变的用户资料，不是密码学身份本身。

---

## 4. Device Identity / Authorization

每个 Device 使用独立 Ed25519 signing key。

```
DeviceId = SHA-256(
  "seed:device-id:v1\0" ||
  device_public_key
)
```

Root Identity 签署 `DeviceAuthorization`，绑定：

- IdentityId；
- DeviceId；
- Device public key；
- authorization sequence；
- issued_at timestamp。

日常 Event 使用 Device key 签名，而不是 Root key。

Root key 不复用于 transport encryption。

### 尚未冻结

- Device revocation event；
- Root recovery/rotation；
- persistent secret storage；
- transport key agreement；
- multi-device policy。

---

## 5. SpaceKind

当前 Core 结构类型：

```
Direct
Group
Tree
Branch
Channel
```

这些类型只表达结构/拓扑语义，不表达：

- Owner；
- Admin；
- Moderator；
- 公司；
- 民主；
- 投票；
- 社会等级。

---

## 6. Genesis

当前 `GenesisDraft` 包含：

- SpaceKind；
- creator IdentityId；
- creator DeviceId；
- created_at_ms；
- Genesis plugins。

每个 Genesis plugin 包含：

- PluginId；
- semantic version；
- package digest；
- opaque config bytes。

插件列表在签名前按 PluginId canonical sort，重复 PluginId 被拒绝。

创建者只是 provenance。

```
creator != implicit owner
```

如果创建者希望成为 Owner，必须由创世插件配置表达。

---

## 7. Genesis lifecycle

当前实现：

```
Draft
  -> attach/configure Genesis plugins
  -> canonicalize
  -> authorized Device signs
  -> immutable GenesisRecord
  -> GenesisId
  -> SpaceId
```

当前：

```
GenesisId = SHA-256(
  "seed:genesis-id:v1\0" ||
  canonical_unsigned_genesis ||
  device_signature
)
```

```
SpaceId = SHA-256(
  "seed:space-id:v1\0" ||
  GenesisId
)
```

### Protocol-freeze blocker: creation nonce

当前 Genesis v1 Spike 没有独立随机 creation nonce。

协议冻结前应增加高熵随机 nonce，使 Space uniqueness 不依赖 wall clock。

该变更必须：

- version-bump；
- 更新 canonical vector；
- 更新 GenesisId / SpaceId test vectors；
- 不得静默改变现有 v1 编码。

---

## 8. Event

当前 Event Header：

```text
EventHeader {
    protocol_version
    space
    author
    device
    sequence
    timestamp_ms
    schema
}
```

Event 还包含：

- payload；
- Device signature；
- derived EventId。

当前：

```
EventId = SHA-256(
  "seed:event-id:v1\0" ||
  canonical_unsigned_event ||
  device_signature
)
```

Device signature 覆盖 domain-separated canonical unsigned Event。

`timestamp_ms` 不能单独作为授权依据。

---

## 9. Event acceptance pipeline

网络收到的数据不直接等于有效状态。

目标 pipeline：

```
decode/framing
  -> schema/version validation
  -> Space lookup
  -> Identity/DeviceAuthorization validation
  -> signature validation
  -> duplicate/replay validation
  -> Capability/Governance authorization
  -> state-transition validation
  -> append accepted Event
  -> materialize state
```

无效 Event：

- 不进入 authoritative accepted history；
- 不修改 materialized state；
- 可进入受限 diagnostics，但不能与合法历史混淆。

---

## 10. Capability

Core 当前定义基础 Capability primitives，例如：

- MemberInvite；
- MemberRemove；
- MessageSend；
- MessageDelete；
- SpaceCreateChild；
- PluginInstall；
- PluginRemove；
- PluginUpdate；
- PluginConfigure；
- Custom。

默认 evaluator 是 **DefaultDeny**。

这是一个重要安全基线：

> 没有 Governance 插件认领并授权一个敏感动作，不等于创建者自动拥有它。

最终 evaluation 需要支持：

```
Allow
Deny
Pending
```

Pending 用于插件实现：

- vote；
- multisig；
- approval；
- timelock。

Core 不需要内置“投票”这一社会概念。

---

## 11. Plugin pinning

Genesis 已经通过 package digest 固定插件内容。

运行中安装/升级插件也应记录至少：

- PluginId；
- version；
- package digest；
- API compatibility metadata。

同名同版本内容不可被静默替换后继续被视作同一个历史对象。

---

## 12. Storage semantics

当前 Spike 有 append-only in-memory Event Store 和 content-derived duplicate suppression。

长期目标：

```
Validated Event Log = authoritative accepted history
Materialized View    = rebuildable current state
```

持久化 backend 尚未决定。

---

## 13. Transport boundary

Transport 尚未进入实现主干。

设计目标：

```
Direct path
Relay path
Tree Host path
```

上层 Event/Space 不依赖具体 TCP/QUIC/WebRTC adapter。

Relay 只负责传输端到端密文，不天然拥有 Governance 权限。

---

## 14. Tree Host trust model

客户端从 Tree Host 收到 Event 时仍必须执行协议验证。

禁止：

```
if source_is_tree_host {
    bypass_governance();
}
```

Tree Host 能：

- 路由；
- 存储；
- 拒绝服务；
- 延迟或隐藏数据。

但仅凭 Host 身份不能构造合法管理员权限。

---

## 15. Test vectors

当前实现分支已经有 deterministic Identity -> DeviceAuthorization -> Genesis -> Event vector。

现有 vector 固定：

- Root secret seed；
- Device secret seed；
- public keys；
- IdentityId；
- DeviceId；
- DeviceAuthorization signature；
- Genesis canonical bytes；
- GenesisId；
- SpaceId；
- Genesis Device signature；
- EventId；
- Event signature。

任何 incompatible canonical framing 变化都必须 version-bump，而不是直接修改旧 vector 的含义。

---

## 16. Protocol freeze gate

在把协议称为 Stable/Alpha wire contract 前，至少完成：

1. Genesis explicit nonce；
2. Device revocation model；
3. canonical decoder fuzzing；
4. independent/cross-language canonical vector；
5. Event acceptance pipeline；
6. unauthorized governance Event rejection；
7. persistent Event Store recovery；
8. Direct authenticated session；
9. plugin package/runtime permission model；
10. minimum version negotiation。

在这些条件前，当前 v1 只是可执行 Spike protocol。
