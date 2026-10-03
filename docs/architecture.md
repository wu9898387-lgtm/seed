# Seed 初步架构草案

> 状态：探索阶段 / 非冻结协议

本文档记录 Seed 当前讨论形成的基础架构思想。目标不是一次性规定最终实现，而是先固定几个不应轻易破坏的系统边界。

---

## 1. 产品定位

Seed 不是传统“平台服务器 -> 用户客户端”的聊天产品。

它更接近一个：

- Local-first 通信系统
- P2P 节点网络
- 用户自托管社区系统
- Plugin-first 社会规则运行时

传统模型：

```
Company Platform
      |
 Central Server
      |
    Users
```

Seed 目标模型：

```
User
├── Identity
├── Device Node
├── Local Data
├── Plugins
├── Direct relations
├── Groups
└── Hosted Trees
```

用户不是平台数据库中的一个被动账号，而是一个可以独立持有身份、数据和计算能力的节点。

---

## 2. 三个一级场景

### 2.1 Direct

一对一聊天由双方节点直接建立关系。

```
Alice Device <==== secure transport ====> Bob Device
```

双方各自持有：

- 对方密码学身份
- 设备信息
- 会话状态
- 本地消息记录
- 会话插件状态
- 本地偏好

Direct 不要求存在一个拥有会话的中心业务服务器。

现实网络环境中可以通过 Relay / Mailbox 传输密文，但 Relay 不拥有会话治理权。

---

### 2.2 Group

Group 默认是一个无社会身份的多人共享 Space。

```
Group
├── Alice
├── Bob
├── Carol
└── David
```

Core 不天然知道：

- Owner
- Admin
- Moderator
- Leader

没有安装治理插件时，原则上不存在这些角色。

“谁能踢人”“谁能邀请”“谁能删别人消息”等问题不由 Group Core 硬编码回答，而由 Capability + Plugin 共同决定。

---

### 2.3 Tree

Tree 是需要长期 Host 的结构化社区。

```
Tree Host
│
├── Branch
│   ├── Channel
│   └── Channel
│
├── Branch
│   └── Space
│
└── Channel
```

Tree 可以承载：

- 兴趣社区
- 游戏社区
- 公司社区
- 项目空间
- 话题网络
- 插件定义的新型空间

Tree Host 天然拥有的是：

- 网络可达性
- Tree topology
- 存储
- 同步
- 插件运行所需基础设施

它不应因此自动获得“社区所有者”这一社会身份。

---

## 3. Core 的职责边界

建议 Core 尽量只包含以下概念：

```
Identity
Device
Crypto
Transport
Storage
Sync
Space
Event
Capability
Plugin Runtime
```

### Core 应该知道的

- 身份密钥
- 设备授权
- 加密与签名
- 消息/事件传输
- 本地数据
- Space 的拓扑与成员关系
- 插件声明和隔离
- Capability 调用
- 网络连接
- 基础阻断与断开能力

### Core 不应该知道的

- 群主是什么
- 管理员是什么
- 民主制度是什么
- 谁应该被罢免
- 什么是公司组织结构
- 什么内容应该被社区删除
- 谁拥有更高社会地位

原则：

> Core verifies machines and cryptographic identities. Plugins govern humans and communities.

---

## 4. Space 统一抽象

Direct、Group、Tree、Branch、Channel 应尽量复用同一套基础抽象。

概念示意：

```text
Space {
    id
    kind
    members
    parent?
    children?
    plugins
    state
}
```

可能的类型：

```
Space<Direct>
Space<Group>
Space<Tree>
Space<Branch>
Space<Channel>
```

这不是最终数据结构，只是架构方向。

优势：

- 事件模型统一
- 插件接口统一
- 权限模型统一
- UI 可以复用
- 新 Space 类型更容易扩展

---

## 5. Capability 模型

Core 提供动作，而不是身份。

例如：

```
member.invite
member.remove
member.ban

message.send
message.delete
message.pin

space.create_child
space.delete_child

plugin.propose
plugin.install
plugin.remove
plugin.update
plugin.configure
```

Core 可以判断调用是否满足底层安全条件，但“谁有资格调用”主要由当前生效的治理插件决定。

例如：

### 群主制

```
Owner Plugin
    |
    +-- owner -> member.remove
    +-- owner -> plugin.install
    +-- owner -> role.assign
```

### 投票制

```
Proposal
   |
 Vote
   |
Threshold reached
   |
member.remove(target)
```

### 陪审制

```
random members
     |
  decision
     |
Capability
```

---

## 6. Genesis：创世阶段

插件先行必须解决一个关键问题：

> 如果运行中的插件变更需要治理授权，那么第一个治理插件是谁批准的？

Seed 使用 Genesis 阶段解决。

生命周期：

```
Draft
  |
Select space type
  |
Install/configure Genesis plugins
  |
Sign Genesis Record
  |
Space becomes active
  |
Normal governance begins
```

在 Draft 阶段，空间还没有正式成为一个运行中的社会系统，所以创建者可以直接配置创世插件。

Genesis 完成以后，插件变更进入当前制度控制。

---

## 7. Genesis Record

Genesis 应成为可验证的协议记录之一。

概念示意：

```text
Genesis {
    space_id
    space_kind
    creator_identity

    plugins[]
    plugin_configs

    timestamp
    signature
}
```

Genesis 的价值：

- 可以证明空间初始规则
- 可以验证创始插件组合
- 减少“最初规则到底是什么”的争议
- 允许新成员在加入前审查制度
- 支持治理历史追踪

未来可以考虑把 Genesis hash 作为 Space identity 的组成部分。

---

## 8. 群聊治理

Group 可以选择多种模型。

### 无治理插件

```
Alice = Member
Bob   = Member
Carol = Member
```

不存在 Owner/Admin。

### 传统群主插件

创建前预装：

```
Owner Governance
```

Genesis 后：

```
Alice = Owner
Bob   = Member
Carol = Member
```

Alice 成为 Owner 的原因不是“因为她创建了群”，而是：

> Genesis 中安装的插件把 Alice 定义为了 Owner。

### 其他模型

可以实现：

- 多 Owner
- Admin hierarchy
- Democracy
- Rotating leader
- Jury
- Reputation governance
- 完全不可踢人

---

## 9. Tree 治理

Tree 更偏社区，因此官方模型不建议默认引入不可挑战的 Owner。

可以定义：

```
Primary Administrator
Administrator
Member
```

但这些仍然是插件角色。

### 社区 Tree 示例

```
Genesis:
- Community Governance
- Voting
- Recall
```

初始：

```
Creator -> Primary Administrator
```

后续成员可以通过制度规定的投票程序罢免主管理人。

### 公司 Tree 示例

```
Genesis:
- Organization Governance
- Organization Root Key
- No Member Recall
```

这时普通员工无法通过成员投票罢免公司指定主管理人。

Core 不负责判断“谁是真的公司”。

普通社区如果愿意，也可以采用同样不可罢免的制度。

Seed 提供能力和制度表达方式，而不是替所有社区规定统一社会制度。

---

## 10. 治理的递归问题

如果某插件规定：

```
Alice cannot be recalled
```

成员可能试图直接卸载该插件。

所以必须处理更高一层问题：

> 谁可以修改规则本身？

可以把插件大致理解为三类。

### 应用插件

例如：

- Game
- Music
- AI
- Translation
- Economy
- UI extension

### 治理插件

决定：

- 谁能踢人
- 谁能禁言
- 谁能建立频道
- 谁能任命角色

### 宪法/Meta-Governance 插件

决定：

- 谁能安装插件
- 谁能卸载插件
- 谁能更新插件
- 谁能修改治理制度
- 修改制度需要什么阈值

示例：

```
Governance change:
    2/3 members

Constitution change:
    3/4 members
```

或者：

```
Governance change:
    Organization Root Key only
```

这三层不一定必须在实现中成为硬编码的三个技术等级，但系统必须能够表达这种元治理关系。

---

## 11. 加入前可见的制度

Tree / Group 的制度信息应尽量可在加入前检查。

例如邀请页面可以展示：

```
Primary Administrator:
Alice

Administrator selection:
appointed by Primary Administrator

Primary Administrator recall:
2/3 member vote

Plugin changes:
simple majority

Hosting:
tree.example

Identity policy:
cryptographic identity only
```

目标是：

> 先看清制度，再决定是否加入。

而不是加入以后才发现该空间如何运作。

---

## 12. Identity 与 Device

现实身份和密码学身份必须分开。

建议结构：

```
Root Identity
├── Desktop Device Key
├── Phone Device Key
└── Tablet Device Key
```

Root Identity 负责：

- 身份连续性
- 授权新设备
- 撤销设备
- 对关键身份事件签名

Device Key 负责：

- 日常会话认证
- 设备级签名
- 会话密钥协商

Seed 主要提供 Proof of Control，而不是 Proof of Legal Identity。

需要重点设计：

- key rotation
- device revocation
- lost-device recovery
- multi-device synchronization
- identity fingerprint
- trust-on-first-use / verification

---

## 13. 网络模型

### Direct

优先：

```
Peer <-> Peer
```

无法直接通信时：

```
Peer -> Relay -> Peer
```

Relay 原则上只处理密文。

### Group

Group 不应简单要求所有成员互相暴露公网网络信息。

可根据实现选择：

- 小群 mesh
- selective peer topology
- elected relay
- stateless relay
- user-hosted relay
- hybrid distribution

拓扑属于工程问题，不应改变社会治理语义。

### Tree

```
Clients
   |
Tree Host
```

Tree Host 可以负责：

- topology
- event distribution
- durable storage
- plugin runtime
- discovery

但 Tree Host 的机器控制权与社区社会权限需要尽可能在模型上分离。

---

## 14. 隐私与安全

P2P 不是“自动安全”。

必须分别处理：

### 传输安全

- authenticated encryption
- forward secrecy
- replay protection
- key rotation

### 终端安全

- local data encryption
- device key protection
- database access isolation

### 插件安全

插件必须至少具备：

- Permission Manifest
- Capability-based API
- Sandbox
- scoped storage
- scoped network access
- explicit sensitive permissions

插件不能因为“安装了”就天然获得：

- 私钥
- 所有聊天记录
- 任意本地文件
- 任意系统调用

### 网络隐私

公网 IP 不等于真实身份。

P2P 连接可能泄露网络元信息，因此应允许 Relay / Privacy Mode，尤其是陌生人、公开群和大型社区场景。

---

## 15. 审核与社区规则

Seed Core 不承担统一的中心化社区内容治理角色。

Core 应保留基础个人安全原语：

- block peer
- disconnect
- local hide
- rate-limit primitives
- cryptographic verification

社会治理可以通过插件实现：

- Anti-Spam
- Moderation
- Report
- Community Jury
- Company Compliance
- Local-only moderation

重要区别：

```
Core Security != Community Moderation
```

---

## 16. 插件系统

插件是 Seed 的核心扩展机制。

### Manifest 概念

```text
PluginManifest {
    id
    name
    version

    permissions[]
    capabilities[]

    events[]
    ui_extensions[]

    storage_scope
    network_scope
}
```

### 可能的 UI 扩展点

- Sidebar
- MessageAction
- Composer
- ContextMenu
- Settings
- MemberPanel
- ChannelPanel
- Command
- TreePanel

例如 Voting Plugin 可以同时提供：

- 后台投票状态机
- Capability 决策逻辑
- `/vote` command
- 消息中的投票卡片
- Tree 管理面板

---

## 17. 插件 ABI 与长期兼容

插件生态一旦形成，ABI/API 兼容性会成为核心资产。

因此需要尽早考虑：

- plugin API versioning
- capability namespacing
- manifest schema version
- event schema version
- deterministic upgrade rules
- plugin migration
- plugin state backup
- incompatible plugin handling

原则上，Core 应提供稳定、窄而清晰的接口，而不是把内部实现直接暴露给插件。

---

## 18. 模块化开发

代码组织也应该遵循插件化哲学。

可能的高层模块：

```
core/
  identity
  crypto
  transport
  storage
  sync
  space
  events
  capability
  plugin

plugins/
  governance-basic
  governance-community
  voting
  moderation
  ...
```

具体语言和目录暂未确定。

---

## 19. 2 MB Core 目标

Seed 希望把“体积约束”当作架构纪律。

目标：

> Core Runtime 尽量控制在 2 MB 以内。

这里的 Core Runtime 指最基础的通信和插件运行能力，不包含用户后续安装的完整插件生态。

基础版本尽量只做：

- Identity
- Direct
- Group
- Tree
- Text message
- Basic file transfer
- Secure transport
- Local storage
- Plugin Runtime
- Capability API

第一阶段尽量避免直接内置：

- Voice
- Video
- AI
- Economy
- Large media stack
- Complex moderation
- Full Web runtime

影响最终体积的最大因素往往不是业务源码，而是：

- runtime
- crypto dependencies
- QUIC/WebRTC
- database
- image/media codecs
- embedded browser
- plugin VM

因此“< 2 MB”是明确的优化方向，但在技术选型完成前不应视为已经保证的最终指标。

---

## 20. MVP

第一阶段可以非常克制：

```
Identity

Direct
Group
Tree

Text Message
File Message

P2P Transport
Relay fallback

Plugin Runtime
Capability API

Basic Governance Plugin
```

目标首先是验证以下问题：

1. 用户身份是否能在无中心账号体系下自然使用；
2. Direct / Group / Tree 是否可以共享 Space/Event 抽象；
3. Genesis 是否足以解决初始治理问题；
4. Capability 是否能支撑插件式治理；
5. 插件权限隔离是否足够清晰；
6. Tree Host 与社区社会权限是否能解耦；
7. 小核心是否真的能支撑扩展生态。

---

## 21. 当前暂不冻结的问题

以下问题需要原型验证后再决定：

- 实现语言
- 插件运行时（WASM / native / custom VM / other）
- Direct 与 Group 的具体 P2P 协议
- NAT traversal
- Relay protocol
- 消息存储格式
- 多设备冲突解决
- CRDT / event log / other sync model
- Tree federation
- plugin distribution/signing
- plugin trust/reputation
- Genesis Record 的最终编码方式
- Space ID 生成规则
- identity recovery
- 2 MB Core 的精确定义

---

## 22. 当前最重要的设计原则

1. **Everything social is a plugin.**
2. **Core 提供能力，不定义社会身份。**
3. **Genesis 决定初始制度，运行中的制度由插件治理。**
4. **用户身份属于用户，而不是中心平台账号数据库。**
5. **Direct / Group 优先 Local-first 与 P2P。**
6. **Tree 可以有长期服务器，但服务器本身不等于社会意义上的所有者。**
7. **安全原语属于 Core，社区审核属于插件。**
8. **插件权限必须默认最小化。**
9. **保持 Core 小、稳定、可验证。**
10. **复杂性优先进入插件，而不是不断膨胀 Core。**
