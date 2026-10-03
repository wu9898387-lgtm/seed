# Seed 产品需求文档（PRD）

> 文档状态：Draft v0.1  
> 项目阶段：概念验证 / MVP 定义  
> 适用范围：Seed Core、官方基础客户端、首批官方插件、Tree Host 参考实现

---

## 1. 产品一句话

Seed 是一个 **Local-first、P2P、Plugin-first、User-hosted** 的通信系统：

- 用户拥有自己的密码学身份与本地数据；
- Direct 与 Group 优先由用户节点直接组成；
- Tree 是用户自行托管的结构化社区；
- Core 只提供通信、安全、存储、事件与 Capability；
- 群主、管理员、投票、罢免、审核、公司治理等社会规则由插件定义。

核心原则：

> **Core defines capabilities. Genesis defines the initial rules. Plugins define the social system.**

---

## 2. 背景与问题

传统聊天平台通常把以下内容绑定在同一个中心平台中：

- 用户账号
- 消息服务器
- 社区托管
- 权限体系
- 管理员模型
- 审核规则
- 功能扩展
- 平台身份

这会带来几个结构性限制：

1. 用户是平台账号，而不是独立网络身份；
2. 社区只能使用平台预设的管理制度；
3. 群主、管理员等身份被硬编码；
4. 扩展能力通常由平台审核和发布节奏决定；
5. 数据、身份、社区与服务器控制权高度耦合；
6. 用户无法自然地把自己的设备或服务器当成网络的一部分。

Seed 的目标不是复制一个更轻量的 Discord / Telegram，而是把聊天软件拆回几个基础原语，再允许用户与插件重新组合。

---

## 3. 产品愿景

Seed 希望形成一个“用户本身就是基础设施参与者”的通信网络。

### 用户可以

- 创建一个不依赖中心账号数据库的密码学身份；
- 让自己的设备成为本地节点；
- 与其他身份直接建立关系；
- 创建无群主、无管理员的 Group；
- 在创建前安装治理插件，使自己成为传统群主；
- 创建一个 Tree，并自行运行其 Host；
- 为 Tree 选择社区、公司、民主、永久管理等不同治理制度；
- 安装插件扩展功能、治理和 UI；
- 在加入 Group / Tree 前查看其治理规则；
- 在不改变 Core 的情况下创造新的社会组织方式。

### 开发者可以

- 开发应用插件；
- 开发治理插件；
- 开发审核/反垃圾插件；
- 开发 UI 扩展；
- 开发 Tree 玩法；
- 开发不同网络与存储适配层；
- 在稳定 Capability API 上扩展，而不 fork 整个客户端。

---

## 4. 非目标

MVP 阶段明确不追求：

- 成为功能完整的 Discord / Telegram 替代品；
- 内置语音、视频、直播；
- 内置支付、钱包、交易所；
- 内置 AI 平台；
- 建立全网统一的实名/KYC 系统；
- 建立中心化内容推荐系统；
- 建立中心化社交图谱；
- Core 内置复杂管理员制度；
- Core 内置统一社区审核规则；
- 第一版实现跨 Tree 联邦；
- 第一版解决所有 NAT 与移动端后台限制；
- 为了“功能齐全”牺牲小核心与模块边界。

---

## 5. 目标用户

### 5.1 普通个人用户

需求：

- 与朋友直接聊天；
- 建立小群；
- 本地保存数据；
- 不希望必须提交现实身份；
- 希望软件简单、快速、可理解。

### 5.2 社区创建者

需求：

- 创建自己的 Tree；
- 自己托管；
- 自定义管理制度；
- 安装投票、审核、Bot、游戏等插件；
- 让成员加入前看到规则。

### 5.3 小团队与公司

需求：

- 使用自己的 Tree Host；
- 使用公司式治理；
- 明确主管理人；
- 可以禁止普通成员罢免组织管理员；
- 使用独立的合规、审计或目录插件。

### 5.4 插件开发者

需求：

- 稳定的 API / ABI；
- 权限明确；
- 可测试；
- 可独立发布；
- 不需要修改 Core 就能实现社会规则和新功能。

### 5.5 自托管与极简软件用户

需求：

- 小体积；
- 可运行自己的节点；
- 不依赖单一商业服务器；
- 可理解的数据路径；
- 可替换 Relay / Tree Host。

---

## 6. 核心对象

Seed 第一阶段只有三个用户可直接理解的一级场景：

### 6.1 Direct

两个身份之间的直接关系与会话。

特点：

- 优先 P2P；
- 本地保存；
- 支持 Relay fallback；
- 不存在群体治理问题；
- 支持会话级插件。

### 6.2 Group

多人离散式通信空间。

默认：

- 无 Owner；
- 无 Admin；
- 无 Moderator；
- 无天然创建者特权。

通过 Genesis 插件可以变成传统群主制或其他制度。

### 6.3 Tree

长期 Host 承载的结构化社区。

特点：

- 有 Tree Host；
- 有分支与频道；
- 支持持久化社区状态；
- Host 不自动获得社会意义上的 Owner 权限；
- 官方默认语义使用“主管理人（Primary Administrator）”，而非不可挑战的“所有者”。

---

## 7. 产品原则

### P-01：插件先行

社会规则尽量不进入 Core。

### P-02：创建者不是天然统治者

创建前可以通过 Genesis 安装插件取得特定身份，但该身份来源于插件规则，而不是 Core 隐式特权。

### P-03：Host 不等于 Authority

控制 Tree Host 的机器，不应在协议语义上自动等价于拥有全部社区管理能力。

### P-04：用户身份属于用户

Seed Identity 是密码学身份，不是中心服务器数据库里的账号 ID。

### P-05：安全原语属于 Core

加密、设备认证、本地阻断、插件权限隔离等不能依赖第三方治理插件。

### P-06：社区治理属于插件

踢人、禁言、投票、版主、审核、举报流程等由插件决定。

### P-07：加入前可理解

用户应尽量能够在加入 Group / Tree 前看到关键治理规则。

### P-08：Local-first

正常使用时，本地节点应保持可用；网络不可用时尽可能保留可访问的历史数据与可排队操作。

### P-09：不把 P2P 当作真实身份

IP、端口、网络路径不能被产品描述为现实身份凭证。

### P-10：保持 Core 极小

复杂能力优先以插件形式存在。

---

## 8. 关键用户流程

## 8.1 首次启动

用户应能够：

1. 创建 Seed Identity；
2. 自动生成 Root Identity Key / Device Key；
3. 选择本地显示名称与头像；
4. 看到可备份/恢复身份的入口；
5. 进入一个极简主界面。

MVP 不要求现实身份信息。

---

## 8.2 添加联系人并建立 Direct

流程：

1. Alice 获取 Bob 的身份信息 / 邀请信息；
2. Alice 验证 Bob 的 Identity Fingerprint；
3. 双方进行身份认证握手；
4. 尝试直接连接；
5. 失败时可使用 Relay；
6. 建立 Direct Space；
7. 双方开始发送文本消息。

验收点：

- Relay 不需要看到明文；
- 身份变化必须对用户可见；
- Direct 历史默认存在本地。

---

## 8.3 创建无治理 Group

流程：

1. 创建 Group Draft；
2. 不安装治理插件；
3. 邀请成员；
4. 签署 Genesis；
5. Group 激活。

结果：

- 创建者没有隐式 Owner；
- 所有人在社会身份层面没有角色；
- Core 不显示“群主”字段。

---

## 8.4 创建传统群主 Group

流程：

1. 创建 Group Draft；
2. 选择“传统群聊” Genesis Bundle；
3. Bundle 安装 Owner Governance；
4. 配置创建者为初始 Owner；
5. 签署 Genesis；
6. Group 激活。

之后新增/删除治理插件必须遵循当前 Governance。

---

## 8.5 创建社区 Tree

流程：

1. 创建 Tree Draft；
2. 配置 Tree Host；
3. 选择 Community Governance；
4. 配置 Primary Administrator；
5. 可选安装 Voting / Recall；
6. 创建初始 Branch / Channel；
7. 签署 Genesis；
8. Tree 激活。

用户加入前可看到：

- 当前主管理人；
- 产生方式；
- 是否可罢免；
- 插件变更规则；
- Host 信息；
- 身份策略。

---

## 8.6 创建公司 Tree

允许 Genesis 配置：

- Organization Governance；
- Primary Administrator；
- Organization Root Key；
- No Member Recall；
- Audit / Directory 等插件。

普通成员无法通过默认成员投票修改公司治理，除非公司选择允许。

Core 不判断该 Tree 是否“真正属于公司”。

---

## 8.7 运行中安装插件

流程：

1. 用户提出 plugin.install 请求；
2. 当前 Meta-Governance 判断是否允许；
3. 需要时产生投票/批准流程；
4. 通过后安装；
5. 插件获得 Manifest 声明范围内的权限；
6. 产生可验证的治理事件。

禁止绕过现有规则直接“因为创建者是创建者”安装。

---

## 9. 功能需求

优先级：

- **P0**：MVP 必须；
- **P1**：MVP 后优先；
- **P2**：后续扩展。

### FR-01 Identity / Device — P0

必须支持：

- 创建 Root Identity；
- 创建当前 Device Identity；
- Root -> Device 授权；
- Identity Fingerprint；
- 本地私钥安全存储接口；
- 设备撤销模型；
- 身份导出/恢复的基础方案。

P1：

- 多设备同步；
- 更完整的 key rotation；
- 社交验证与共同联系人信任提示。

---

### FR-02 Direct — P0

必须支持：

- 建立联系人；
- 安全握手；
- 文本消息；
- 消息时间与发送状态；
- 本地历史；
- 直接连接；
- Relay fallback；
- Block / Disconnect。

P1：

- 文件消息；
- 多设备 Direct 同步；
- 消息回复与反应作为插件/扩展。

---

### FR-03 Group — P0

必须支持：

- 创建 Group；
- Genesis；
- 邀请成员；
- 文本消息；
- 成员列表；
- 插件列表；
- 无角色默认模式；
- Governance 插件控制 Capability。

P1：

- 更复杂 Group 网络拓扑；
- 大群优化；
- Group 历史同步策略。

---

### FR-04 Tree — P0

必须支持：

- 创建 Tree Host；
- 创建 Tree；
- Branch / Channel 最小层级；
- Tree 成员；
- Tree 文本频道；
- Genesis；
- 插件运行；
- Host 持久化；
- 加入前治理信息。

P1：

- 多 Host / 高可用；
- Tree 迁移；
- Tree 备份恢复；
- 更复杂 Branch 类型。

---

### FR-05 Genesis — P0

必须支持：

- Draft 状态；
- 创建前安装插件；
- 创建前配置插件；
- Genesis Record；
- Genesis 签名；
- Genesis Hash；
- 激活后不可静默改写 Genesis。

Genesis 之后的制度变化只能通过新治理事件表达。

---

### FR-06 Capability — P0

至少定义：

- member.invite
- member.remove
- message.send
- message.delete
- space.create_child
- plugin.install
- plugin.remove
- plugin.update
- plugin.configure

所有敏感 Capability 必须有明确授权路径。

---

### FR-07 Plugin Runtime — P0

必须支持：

- Manifest；
- 插件 ID / Version；
- Capability 请求；
- Event 订阅；
- Scoped Storage；
- 权限声明；
- 插件启停；
- 兼容性检查；
- 最小 Sandbox。

P1：

- UI Extension；
- 网络权限；
- 插件迁移；
- 插件签名与来源验证；
- 插件依赖。

---

### FR-08 官方基础插件 — P0

至少需要：

1. Basic Owner Governance
2. Community Governance
3. Voting / Recall 的最小实现

可以合并实现，但必须能够验证：

- 传统群主制；
- 可罢免主管理人；
- 插件变更由规则控制。

---

### FR-09 Transport — P0

需要抽象：

- Direct transport；
- Relay transport；
- Tree client-host transport。

协议必须：

- 有认证；
- 有加密；
- 有重放保护；
- 能区分身份和设备。

P1：

- NAT traversal 优化；
- QUIC / UDP 路径；
- Relay discovery；
- 用户自定义 Relay。

---

### FR-10 Storage / Sync — P0

必须支持：

- 本地身份数据；
- 本地聊天历史；
- Space 状态；
- Plugin state；
- Genesis；
- 治理事件。

MVP 可优先使用简单事件日志，不要求第一版直接采用复杂 CRDT。

---

### FR-11 Safety & Privacy — P0

Core 必须提供：

- Block Peer；
- Disconnect；
- Local Hide；
- Rate-limit primitive；
- 插件最小权限；
- 敏感权限明确授权；
- 网络元信息不作为现实身份宣称；
- Relay / privacy path 的架构空间。

不要求 Core 建立统一社区内容审核制度。

---

### FR-12 UI — P0

目标是非常基础、低学习成本。

至少提供：

- Direct 列表；
- Group 列表；
- Tree 列表；
- 消息区；
- 成员/空间信息；
- 插件信息；
- Genesis 创建流程；
- 加入前规则摘要。

第一版不追求复杂视觉效果。

---

## 10. 非功能需求

### NFR-01 Core 体积

工程目标：

> **Core Runtime 尽量 < 2 MB。**

该目标必须先定义测量口径。

建议口径：

- Release build；
- stripped；
- 不包含用户插件；
- 不包含平台系统库；
- 不包含 Tree 数据；
- 不包含可选媒体组件。

如果技术验证证明 2 MB 与安全/可移植性冲突，应记录原因，而不是通过删除必要安全功能“达标”。

---

### NFR-02 启动性能

MVP 目标：

- 冷启动尽量接近原生轻量应用体验；
- 基础界面不依赖下载远端网页；
- 本地消息历史可快速打开。

具体数字在首个可运行原型后建立基线。

---

### NFR-03 安全

必须：

- 不自行发明未经审查的密码算法；
- 密钥与普通插件隔离；
- 插件不能直接读取 Root Private Key；
- 关键治理事件可验证；
- Genesis 可验证；
- 网络消息有来源认证。

---

### NFR-04 可移植性

参考实现优先支持至少一个桌面平台。

架构上避免把协议绑定到单一 OS。

后续目标：

- Desktop
- Mobile
- Headless Tree Host

---

### NFR-05 可测试性

Core 模块必须能够：

- 单元测试；
- 使用模拟 Transport；
- 使用内存 Storage；
- 模拟多个节点；
- 在 CI 中运行多节点协议测试。

---

### NFR-06 向后兼容

在插件生态公开前必须建立：

- Protocol Version；
- Plugin API Version；
- Manifest Version；
- Event Schema Version。

---

## 11. MVP 范围

MVP 的定义不是“能演示聊天”，而是能验证 Seed 的核心架构是否成立。

MVP 必须完整走通：

### 场景 A：Direct

两台节点：

- 创建身份；
- 验证身份；
- 建立安全连接；
- 双向文本聊天；
- 本地保存。

### 场景 B：无群主 Group

三个节点：

- 创建无治理 Group；
- Genesis；
- 所有人进入；
- 无 Owner 字段；
- 能正常聊天。

### 场景 C：传统群主 Group

- Genesis 安装 Owner Governance；
- 创建者成为 Owner；
- Owner 可调用 member.remove；
- 普通成员不可以。

### 场景 D：社区 Tree

- 启动 Tree Host；
- Genesis 设置主管理人；
- 创建频道；
- 三个成员加入；
- 发起罢免投票；
- 规则通过后主管理人变更。

### 场景 E：治理不可绕过

- 运行中的普通成员尝试直接安装治理插件；
- 当前治理规则拒绝；
- 通过合法流程后才能安装。

---

## 12. MVP 验收标准

MVP 只有同时满足以下条件才算完成：

1. Core 中没有硬编码 Owner/Admin/Moderator 角色；
2. Group 可以在没有治理插件时存在；
3. 同一个 Group Core 可以通过插件变成群主制；
4. Tree Host 不自动获得协议层全部治理 Capability；
5. Genesis 可以被独立验证；
6. Genesis 激活后不能被直接重写；
7. 插件不能读取 Root Private Key；
8. Direct 可以不依赖中心业务服务器工作；
9. Relay 路径保持端到端密文；
10. 至少一种 Governance 插件能够真正拦截/批准 Capability；
11. 多节点测试可以在 CI 中自动运行；
12. Core 体积有可重复测量脚本与报告。

---

## 13. 产品成功指标

早期不以 DAU 或增长作为首要目标，而以架构验证指标为主。

### 技术指标

- Core release size；
- Core dependency count；
- 启动时间；
- Direct 建链成功率；
- Relay fallback 成功率；
- 消息送达延迟；
- 插件加载时间；
- 多节点一致性测试通过率；
- Crash-free sessions。

### 架构指标

- 新治理模型是否无需修改 Core；
- 新 UI 插件是否无需修改通信协议；
- Tree 治理变化是否可追踪；
- Host 与 Governance 是否保持解耦；
- 插件 API 变化频率是否逐步下降。

### 用户体验指标

在可用性测试中，用户能否理解：

- “这个群没有群主”；
- “主管理人来自插件规则”；
- “创建前插件与创建后插件安装规则不同”；
- “Tree Host 与社区管理员不是同一个概念”。

---

## 14. 风险

### R-01 插件系统过早复杂化

应优先实现最小可行 Capability + Plugin Runtime，不先追求插件商店。

### R-02 P2P 网络复杂度吞噬产品开发

Transport 必须模块化。MVP 可允许较简单的 Relay fallback。

### R-03 治理递归导致模型难以理解

必须让 Meta-Governance 有明确、可验证的最终授权路径。

### R-04 2 MB 目标导致错误技术决策

体积是架构约束，不应凌驾于密码学安全、正确性和可维护性。

### R-05 Host 与社会权限重新耦合

协议测试必须专门验证：控制 Host 不能等价于自动获得所有 Governance Capability。

### R-06 插件成为安全边界漏洞

插件 Runtime 与 Permission Manifest 必须在第一版就存在，而不是后补。

---

## 15. 待决策项

进入编码前至少需要形成 ADR / 原型结论：

- Core 主实现语言；
- Plugin Runtime；
- Plugin ABI；
- Identity key hierarchy；
- Direct secure session protocol；
- Event log / state model；
- Storage backend；
- Relay 协议；
- Tree Host wire protocol；
- Genesis canonical encoding；
- Space ID 生成规则；
- Core 体积测量口径。

---

## 16. PRD 总结

Seed 第一阶段不是要做最多功能，而是验证一组强约束能否共同成立：

> 用户拥有身份；节点可以直接通信；Tree 可以自托管；社会身份不进入 Core；Genesis 能定义初始制度；插件能治理 Capability；安全能力保持在 Core；整个基础运行时仍然足够小。

只要这套骨架成立，后续语音、游戏、Bot、审核、公司组织、AI、经济系统等功能都可以在其上生长，而不需要不断扩大 Seed Core。
