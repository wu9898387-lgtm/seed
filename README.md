# Seed

Seed 是一个以 **Local-first / P2P / Plugin-first / User-hosted** 为核心理念的通信软件实验。

它不以“平台拥有服务器、用户只是账号”的传统模式为基础，而是尽量把每个用户视为一个独立节点：用户拥有自己的身份、设备、数据和插件环境，并可直接与其他节点建立关系。

Seed 目前围绕三种一级使用场景设计：

1. **一对一聊天（Direct）**
2. **群聊（Group）**
3. **树（Tree）**

其中 Direct 与 Group 默认采用分布式/离散节点模型；Tree 则是一种需要长期主服务器的结构化社区。

> Core defines capabilities. Genesis defines the initial rules. Plugins define the social system.

## 核心理念

### 1. 插件先行

Seed Core 不内置“群主”“管理员”“版主”“投票”“罢免”等社会身份和治理规则。

Core 只提供可调用的基础能力，例如：

- 成员加入/退出
- 邀请
- 移除成员
- 消息发送/处理
- 插件安装、移除、更新和配置
- 分支与频道管理
- 本地存储
- 网络连接
- 密码学身份与签名

至于“谁可以调用这些能力、在什么条件下可以调用”，由插件决定。

因此：

- 群主制可以是插件；
- 民主投票可以是插件；
- 管理员制度可以是插件；
- 公司式组织结构可以是插件；
- 无角色、完全平权的空间也可以不安装任何治理插件。

### 2. 创世配置（Genesis）

空间正式创建之前，创建者可以直接选择并配置一组创世插件。

这意味着用户可以在创建 Group 或 Tree 之前，预装：

- 群主/主管理人插件；
- 管理员插件；
- 投票插件；
- 罢免插件；
- 不可罢免规则；
- 公司治理规则；
- 其他功能插件。

Genesis 完成后，空间进入正常运行状态。之后的插件安装、卸载、更新和治理变更，都必须按照当前已经生效的插件规则执行。

创建者之所以可能成为“群主”或“主管理人”，并不是因为 Core 天然赋予其特权，而是因为创世插件明确赋予了这一角色。

### 3. Tree 是社区，不是超级群

Tree 是一个由长期主服务器承载的结构化社区，可以包含：

- Branch
- Channel
- Group-like spaces
- Topics
- Game spaces
- 其他插件定义的子空间

Tree 不默认存在“所有者”这一垄断身份。官方建议使用“主管理人（Primary Administrator）”一类可由插件定义、并可进一步配置罢免/继任规则的角色。

特殊组织（例如公司）也可以安装不同治理插件，使主管理人不可由普通成员罢免。

### 4. Local-first 与 P2P

Direct 与 Group 尽量不依赖中心业务服务器。

理想模型：

```
Alice Node <---- encrypted direct/relay transport ----> Bob Node
```

每个用户设备既是客户端，也是自己的本地节点。

现实网络环境下仍允许使用无治理权的 Relay / Mailbox / Discovery 基础设施，以处理：

- NAT
- CGNAT
- 移动网络
- 离线消息
- 设备休眠
- 多设备同步

Relay 只是传输和暂存设施，不应天然拥有社区治理权或明文读取权。

### 5. 不依赖现实身份审核

Seed 更关注 **密码学身份连续性**，而不是强制现实身份认证。

系统应证明的是：

- 某个身份控制对应密钥；
- 某条消息由某个身份/设备签名；
- 某个新设备确实被该身份授权；
- 连接对象的身份指纹是否发生变化。

而不是强制要求：

- 身份证
- 人脸
- 手机号实名
- 公司认证

P2P 本身并不能消灭诈骗、洗钱或其他非法活动，因此协议设计不应把网络地址当作“真实身份”。安全应主要依靠端到端加密、设备认证、密钥管理、插件沙箱和权限隔离。

## 初步核心模块

```
Identity
Device
Transport
Crypto
Storage
Sync
Space
Event
Capability
Plugin Runtime
```

社会规则不进入 Core。

## Space 模型

长期目标是尽量统一不同场景：

```
Space
├── Direct
├── Group
├── Tree
├── Branch
└── Channel
```

不同 Space 共享身份、事件、插件、能力和存储抽象；Tree 额外拥有长期 Host 与层级拓扑。

## 插件

插件预计可以扩展：

- Governance
- Moderation
- Voting
- Roles
- Bots
- Games
- Economy
- AI
- UI
- Commands
- Message actions
- Tree/Channel behavior

插件需要使用明确的 Permission Manifest、Capability API 与 Sandbox，默认不能任意读取私钥、全部聊天记录或本地文件。

## 工程目标

Seed 希望保持一个非常小的基础核心。

初步目标：

> **Core Runtime 尽量控制在 2 MB 以内。**

这不是“整个生态永远小于 2 MB”，而是把核心保持极简，将复杂能力按需放入插件。

为了实现这一点，项目会尽量避免：

- 大型 Web Runtime
- 不必要的重量级依赖
- 把业务功能硬编码进 Core
- 重复实现系统已经可靠提供的能力

## 核心文档

- [产品需求文档（PRD）](docs/PRD.md) — 产品范围、核心流程、MVP 与验收标准。
- [初步架构草案](docs/architecture.md) — 总体系统模型。
- [整体设计目标](docs/design-goals.md) — 架构不变量与安全边界。
- [实现目标与开发路线](docs/implementation-goals.md) — 开发阶段、测试与发布 Gate。
- [Protocol Kernel Draft](docs/protocol-kernel.md) — 当前可执行内核协议事实与冻结条件。
- [Threat Model](docs/threat-model.md) — 攻击面、安全目标与明确非目标。
- [Phase 0 Spike Plan](docs/phase0-spikes.md) — 已完成实验、实测体积与下一批 Spike。
- [Architecture Decision Records](docs/adr/README.md) — 技术决策与接受条件。

## 当前开发状态

项目已经从纯概念阶段进入 **Phase 0 / Core Kernel Spike**。

当前 `main` 已经包含 Root/Device signing、DeviceAuthorization、signed Event、Capability DefaultDeny、Genesis schema/wire v2、128-bit creation nonce、plugin digest pinning、deterministic protocol vector v2、std-only persistent FileEventStore 和 release-size CI。

在同一 CI 构建中，`seed-core-smoke` 为 **380.8 KiB**，包含 persistent file-store 路径的 `seed-storage-smoke` 为 **389.6 KiB**，持久化路径增量约 **8.7 KiB**。当前仍不包含 Transport 与 Plugin Runtime。

协议仍未冻结。持久化 Event Store 已有 append-file baseline；当前优先级转为大历史/SQLite 对照、Transport/Relay 与插件沙箱。
