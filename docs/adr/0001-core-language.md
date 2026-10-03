# ADR-0001: Reference Core 使用 Rust

- Status: Proposed
- Date: 2026-10-03
- Decision scope: Core / Host / SDK boundary

## Context

Seed 需要同时满足：

- Core 尽量小；
- 网络、密码学和插件边界需要较强内存安全；
- 可构建桌面客户端与 headless Tree Host；
- 未来需要稳定 FFI / Plugin ABI；
- 不希望把大型语言运行时捆入 Core。

候选主要为 Rust、C++、C。

## Decision

**Reference Core 优先使用 Rust。**

但这一决定在 Phase 0 Size Spike 完成前保持 Proposed，而不是 Accepted。

实现约束：

1. Core 尽量保持 runtime-agnostic；
2. 不把大型 async runtime 视为默认依赖；
3. 平台 IO、UI、Key Store 等通过 adapter 隔离；
4. 对外稳定边界优先使用协议/明确 ABI，而不是暴露 Rust 内部类型；
5. release build 必须开启 size-oriented profile 实验并记录 stripped size。

Rust 被选择的主要原因不是语法偏好，而是：

- Core 涉及不可信网络输入；
- 插件系统涉及明确安全边界；
- 身份与密钥处理需要减少内存错误风险；
- 仍可生成无 GC 的原生二进制。

## Alternatives

### C++

优点：

- 成熟跨平台生态；
- 原生性能；
- 对系统 API 与现有库连接方便。

缺点：

- Core 的不可信输入面较大，内存安全成本更高；
- 插件、网络和序列化边界需要更多人工防御。

### C

优点：

- 最小 runtime；
- 体积可控；
- ABI 最直接。

缺点：

- 安全工程成本最高；
- 大量状态机、网络数据和插件边界更容易产生内存安全问题。

## Consequences

### Positive

- 更适合把 Core 做成可验证的安全边界；
- 便于模块化；
- 可为 Tree Host 复用核心代码。

### Negative / Cost

- 某些 Rust 依赖可能显著膨胀体积；
- async/泛型/格式化等使用不克制时会伤害 2 MB 目标；
- 需要持续 size report，而不能只在发布前优化。

## Validation

Phase 0 必须构建至少三个 stripped release 样本：

1. empty core；
2. crypto + canonical serialization；
3. crypto + serialization + storage + transport skeleton。

同时记录：

- binary size；
- dependency tree；
- cold start；
- idle memory。

## Revisit conditions

以下任一情况发生时重新评估：

- 合理裁剪后基础 Core 明显无法接近 size budget；
- Rust 目标平台出现关键兼容性问题；
- 某个替代方案在安全性与体积上有可重复的显著优势。
