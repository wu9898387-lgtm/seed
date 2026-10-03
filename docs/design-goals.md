# Seed 整体设计目标

> 文档状态：Draft v0.1  
> 本文档定义“Seed 应该保持什么样”，用于约束协议、Core、客户端、插件与 Tree Host 的设计。

---

## 1. 设计目标总览

Seed 的设计目标不是“用 P2P 重做一个现有聊天软件”，而是构建一个足够小、足够稳定、可被插件扩展的通信底座。

优先级顺序：

1. **边界正确**
2. **安全正确**
3. **协议可验证**
4. **模块可替换**
5. **插件可扩展**
6. **用户体验简单**
7. **体积小**
8. **功能数量多**

体积与功能都不能以破坏安全和边界为代价。

---

## 2. 必须长期保持的系统不变量

以下规则一旦违反，应被视为架构回归。

### INV-01：Core 不定义社会身份

Core 不硬编码：

- Owner
- Admin
- Moderator
- Leader
- Primary Administrator

Core 可以保存插件产生的状态，但不能让这些角色成为底层协议必需字段。

### INV-02：创建者没有永久隐式特权

创建者只在 Draft / Genesis 阶段拥有配置初始状态的能力。

Space 激活以后：

- 创建者的权限来自插件；
- 不能使用“creator bypass”绕过治理；
- 如果插件允许罢免创建者，Core 必须接受结果。

### INV-03：Host 不等于 Authority

Tree Host 是基础设施角色，不是社会角色。

控制 Host 机器不能在协议语义上自动获得：

- member.remove
- plugin.install
- role.assign
- governance.override

Host 当然可以在物理层停止服务或篡改自己的程序，因此协议应尽可能让这种行为可检测、可迁移或可恢复，而不是假装物理控制权不存在。

### INV-04：Genesis 是历史，不是可编辑配置文件

Genesis 激活后：

- 原始记录保持不变；
- 制度变化通过新的签名事件表达；
- 不允许覆盖 Genesis 后声称“最初就是这样”。

### INV-05：Capability 是敏感动作的唯一入口

治理相关敏感动作不得绕过 Capability 层。

例如删除成员不应出现：

- UI 直接改数据库；
- Host 直接改成员表；
- 某插件直接写内部结构。

正确路径必须类似：

```
Intent
  -> Capability request
  -> Authorization / Governance
  -> Validated event
  -> State transition
```

### INV-06：插件默认没有权限

插件默认：

- 不能读 Root Private Key；
- 不能读全部聊天；
- 不能访问任意文件；
- 不能访问任意网络；
- 不能执行任意系统调用。

所有敏感能力必须显式声明和授权。

### INV-07：网络地址不是社会身份

IP / endpoint / relay path 只属于网络层信息。

它们不能被 Core 当作：

- 实名；
- 唯一用户身份；
- 信任等级；
- 合法性证明。

### INV-08：Core Security 与 Community Governance 分离

以下属于 Core Security：

- 身份认证；
- 加密；
- Block；
- Disconnect；
- 插件沙箱；
- 速率限制原语；
- 本地数据保护。

以下属于插件/社区治理：

- 谁能踢人；
- 谁能禁言；
- 举报给谁；
- 什么内容违规；
- 管理员如何产生；
- 是否允许罢免。

---

## 3. 总体逻辑架构

```
+--------------------------------------------------+
|                    Client UI                     |
+----------------------+---------------------------+
| Built-in Minimal UI  | Plugin UI Extensions      |
+----------------------+---------------------------+
|                Plugin Runtime                    |
+--------------------------------------------------+
|            Capability / Authorization            |
+--------------------------------------------------+
|     Space / Event / State / Sync Abstractions    |
+--------------------------------------------------+
| Identity | Crypto | Storage | Transport | Device |
+--------------------------------------------------+
| OS / Network / Filesystem / Secure Key Storage   |
+--------------------------------------------------+
```

Tree 模式额外存在：

```
+----------------------+
|      Tree Host       |
+----------------------+
| Tree routing/state   |
| Durable event log    |
| Plugin host runtime  |
| Storage              |
+----------------------+
```

Tree Host 使用相同的身份、事件和 Capability 语义，但运行形态可以是 headless daemon。

---

## 4. Identity 设计目标

### 4.1 身份层级

目标结构：

```
Root Identity
├── Device A
├── Device B
└── Device C
```

Root Identity 是长期连续性锚点。

Device Identity 用于日常通信。

### 4.2 Root Key 最小使用

Root Private Key 不应频繁参与日常聊天。

优先用于：

- 授权设备；
- 撤销设备；
- 恢复关键身份状态；
- 签署极少数身份级操作。

### 4.3 身份应可离线验证

只要拥有所需公钥/证书链，应能验证：

- Device 是否属于 Root Identity；
- 事件签名是否合法；
- Genesis 签名是否合法。

不应依赖“询问 Seed 官方账号服务器”。

### 4.4 身份恢复必须独立设计

“无中心账号”意味着忘记密码不能简单发送短信重置。

实现前必须明确：

- Root Key backup；
- recovery material；
- trusted recovery 是否可选；
- 丢失全部密钥后是否视为新身份。

---

## 5. Space 设计目标

### 5.1 统一基础模型

Direct、Group、Tree、Branch、Channel 尽量复用：

- Space ID
- Event
- Membership
- Plugin attachment
- Capability context
- Local view

但不要为了“统一”强行让不同场景承担无意义字段。

### 5.2 Space ID

最终方案应满足：

- 全局冲突概率极低；
- 可离线创建；
- 不依赖中心分配；
- 可以稳定引用；
- 必要时可绑定 Genesis Hash。

### 5.3 Membership 与 Governance 分开

“某人是成员”和“某人有管理员权力”必须是两个不同概念。

Membership 是基础事实。

Authority 是插件解释出来的权限状态。

---

## 6. Event 设计目标

推荐把状态变化表达为事件，而不是直接同步整个数据库。

概念：

```
Event {
    id
    space_id
    author
    device
    kind
    payload
    timestamp/logical_time
    signature
}
```

具体编码暂未冻结。

目标：

- 可验证；
- 可重放；
- 可审计关键治理变化；
- 支持离线/断线后同步；
- 能从事件推导当前状态；
- 能对恶意或无授权事件拒绝应用。

MVP 可以使用中心化程度较低但简单的事件序列，不必第一版就解决所有分布式一致性理论问题。

---

## 7. Genesis 设计目标

Genesis 是 Space 的第一个特殊记录。

必须能够表达：

- Space 类型；
- 初始成员；
- 初始插件；
- 插件版本；
- 初始插件配置；
- 初始 Host 信息（Tree）；
- 创建时间；
- 创建者签名；
- Canonical Hash。

### 7.1 Canonical Encoding

同一份 Genesis 在不同实现中必须得到同一个 Hash。

因此正式协议前必须固定：

- 字段顺序或 canonical serialization；
- 字符串编码；
- 数字表示；
- plugin ID/version 表示；
- signature coverage。

### 7.2 Genesis Bundle

产品层可以提供模板：

- Empty Group
- Traditional Group
- Democratic Group
- Community Tree
- Organization Tree

Bundle 只是：

```
plugins + config + suggested UI
```

不是 Core 特殊模式。

---

## 8. Capability 设计目标

Capability 名称应稳定、可命名空间化。

例如：

```
seed.member.invite
seed.member.remove
seed.message.send
seed.message.delete
seed.space.child.create
seed.plugin.install
seed.plugin.remove
seed.plugin.update
seed.plugin.configure
```

第三方插件可以定义：

```
com.example.game.match.create
```

### Capability Request

建议抽象：

```text
CapabilityRequest {
    actor
    space
    capability
    resource
    parameters
    context
}
```

Governance 插件返回：

- Allow
- Deny
- Pending

Pending 可以对应：

- 投票；
- 多签；
- 审批；
- 延时执行。

这样投票不是 Core 特殊机制，而是授权流程的一种实现。

---

## 9. Plugin Runtime 设计目标

### 9.1 稳定接口优先于高性能魔法

插件 API 要：

- 小；
- 稳定；
- 明确；
- 可版本化；
- 可沙箱；
- 可测试。

不要暴露内部对象指针或数据库结构给插件。

### 9.2 插件能力分类

逻辑上可以分为：

- Application
- Governance
- Meta-Governance
- Moderation
- UI
- Integration

这主要用于理解和 UI 展示，不必全部成为 Core 硬编码类型。

### 9.3 Permission Manifest

示例：

```text
permissions:
  - messages.read.current_space
  - members.read.current_space
  - storage.private
  - ui.message_action

capabilities:
  - request: seed.member.remove
```

### 9.4 插件状态

每个插件应拥有独立命名空间：

```
(space_id, plugin_id) -> scoped state
```

卸载插件时不能误删其他插件状态。

是否保留卸载插件数据应由明确策略决定。

### 9.5 插件崩溃隔离

一个插件异常不应使：

- 整个客户端崩溃；
- Tree Host 进程失去所有服务；
- 其他插件状态损坏。

---

## 10. Governance 设计目标

### 10.1 治理必须可以递归到“修改治理”

必须能表达：

- 谁能调用普通管理能力；
- 谁能改治理插件；
- 谁能改“谁能改治理插件”的规则。

最终必须存在可计算的授权根。

### 10.2 不强制民主

Seed Core 对下列制度保持中立：

- 单一群主；
- 多管理员；
- 投票民主；
- 公司层级；
- 多签；
- 永久不可罢免；
- 无治理。

产品可以推荐模板，但协议不应宣布某一种制度是唯一合法制度。

### 10.3 制度可见

客户端应将关键治理状态转译为普通用户能理解的摘要，而不要求用户读插件源码。

---

## 11. Transport 设计目标

Transport 是可替换模块。

统一上层接口应尽量隐藏：

- TCP / QUIC / WebRTC；
- 直连 / Relay；
- IPv4 / IPv6；
- NAT traversal 细节。

上层只需要知道：

- 当前 Peer 是否可达；
- 会话是否认证；
- 是否端到端加密；
- 传输质量；
- 当前是否通过 Relay。

### 11.1 Direct-first，不是 Direct-only

正常情况下优先直连。

但必须承认：

- NAT；
- CGNAT；
- 手机后台；
- 防火墙；
- 企业网络；

会导致直连不可用。

Relay 是兼容性基础设施，不应被视为架构失败。

---

## 12. Group 网络设计目标

不要把 Group 简单实现为 N 人全部互连的永久 full mesh。

原因：

- IP 暴露；
- 连接数平方增长；
- 移动设备负担；
- 大群不可扩展。

MVP 可以先支持小群简化拓扑，但 Transport API 必须允许未来替换成：

- relay fanout；
- selective mesh；
- elected relay；
- hybrid topology。

治理语义不得依赖某一种网络拓扑。

---

## 13. Tree Host 设计目标

Tree Host 应尽量是：

- headless；
- 可自托管；
- 可备份；
- 可迁移；
- 可观测；
- 不要求官方云。

Tree Host 与客户端共享协议，但不共享全部 UI 代码。

长期希望允许：

- 家庭服务器；
- VPS；
- NAS；
- 桌面常驻；
- 容器部署。

MVP 可先选择一种部署形式。

---

## 14. Storage 设计目标

本地与 Tree Host 存储层应通过接口隔离。

Core 不应把协议设计绑定到某个数据库。

需要存储：

- Identity metadata；
- Device records；
- contacts；
- events；
- materialized Space state；
- plugin state；
- message blobs；
- Genesis；
- verification metadata。

大文件/附件应避免全部塞入事件数据库。

---

## 15. Sync 设计目标

优先保证：

1. 不丢已确认事件；
2. 重复事件幂等；
3. 无权事件不生效；
4. 断线后可追赶；
5. 多设备不会静默覆盖历史。

MVP 不追求任意网络分区下的完美实时一致。

需要明确区分：

- authoritative event history；
- local UI cache；
- transient presence。

Presence 不应进入永久事件历史。

---

## 16. Crypto 设计目标

密码学实现必须：

- 使用成熟库；
- 使用现代 AEAD；
- 有明确密钥派生；
- 支持会话密钥轮换；
- 考虑前向保密；
- 防重放；
- 签名与加密用途分离。

不得为了压缩二进制体积自行发明“轻量加密算法”。

---

## 17. 客户端 UX 设计目标

Seed 的底层概念复杂，但表面必须简单。

主导航尽量只暴露：

- Chats
- Groups
- Trees
- Plugins / Settings

高级治理细节在需要时出现。

### 17.1 渐进式复杂度

普通用户：

- 可以像普通聊天软件一样使用；
- 不需要理解 Capability。

高级用户：

- 可以看到 Genesis；
- 可以配置治理；
- 可以审查插件权限；
- 可以运行 Tree Host。

### 17.2 不制造假的安全感

UI 不应显示：

- “P2P = 真实身份”；
- “看到 IP = 已实名”；
- “自托管 = 绝对安全”。

应清楚区分：

- verified cryptographic identity；
- user-provided profile；
- network metadata。

---

## 18. 体积设计目标

### 18.1 预算意识

任何进入 Core 的新依赖都要回答：

- 增加多少二进制体积；
- 是否可以使用系统能力；
- 是否能改为可选 feature；
- 是否应该属于插件。

### 18.2 不使用大型嵌入式 Web Runtime 作为默认核心 UI

如果某平台系统已经提供 WebView，可以讨论插件 UI 使用系统 WebView，但不把 Chromium 一类完整运行时捆入小核心目标。

### 18.3 CI 自动记录 Size Budget

每个 release build 自动报告：

- Core binary；
- stripped size；
- dependency contribution（可用时）；
- 与上一版本差异。

---

## 19. 可观测性设计目标

自托管不能等于无法调试。

Tree Host 应支持：

- structured logs；
- health status；
- connection count；
- event queue depth；
- plugin failures；
- storage errors。

但默认日志不得泄露：

- 私钥；
- 明文敏感消息；
- 完整 session secrets。

---

## 20. 协议演进设计目标

所有网络与插件协议都需要显式版本。

需要支持：

- version negotiation；
- unknown field tolerance（适用时）；
- feature negotiation；
- deprecated capability 生命周期；
- migration documentation。

在协议稳定前，可以快速迭代；进入公开生态后，破坏性变化必须非常谨慎。

---

## 21. 测试设计目标

测试金字塔：

```
Unit
  -> Module
    -> Protocol
      -> Multi-node simulation
        -> End-to-end
```

必须重点测试“架构不变量”，而不只是 UI。

例如：

- 无治理 Group 中 creator 没有 member.remove；
- 插件不能绕过 Capability；
- Host 不能自动获得治理权；
- Genesis hash 在多个实现/平台一致；
- 未授权事件不会改变 materialized state。

---

## 22. 设计决策记录

所有重大选择应通过 ADR 记录，例如：

```
docs/adr/
  0001-core-language.md
  0002-plugin-runtime.md
  0003-event-model.md
  0004-identity-keys.md
  0005-transport.md
```

ADR 至少记录：

- Context
- Decision
- Alternatives
- Consequences
- Revisit conditions

这样可以防止后续只记得“选了什么”，忘记“为什么选”。

---

## 23. 设计完成的判断标准

Seed 的整体设计达到第一阶段稳定，不是因为所有问题都被解决，而是因为以下边界已经稳定：

- Identity / Device 模型明确；
- Space / Event 模型明确；
- Genesis canonical form 明确；
- Capability 授权路径明确；
- Plugin Runtime 安全边界明确；
- Direct / Relay / Tree Host transport interface 明确；
- Storage interface 明确；
- MVP Governance 能证明插件治理成立；
- Core size budget 可测；
- 协议版本策略存在。

达到这些条件后再开始大规模功能扩展。
