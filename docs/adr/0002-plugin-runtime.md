# ADR-0002: 插件采用沙箱化 WebAssembly + Seed Host ABI

- Status: Proposed
- Date: 2026-10-03
- Decision scope: Plugin Runtime / Plugin SDK

## Context

Seed 的核心产品原则是 Plugin-first。

插件既可能是普通功能，也可能参与：

- Governance；
- Meta-Governance；
- Moderation；
- UI；
- Bot；
- Tree behavior。

因此插件不是“主题脚本”，而是一个真正的安全边界。

Native in-process 插件如果拥有与 Core 相同地址空间，会直接破坏：

- 私钥隔离；
- scoped storage；
- Capability enforcement；
- crash isolation。

## Decision

插件格式优先采用 **WebAssembly module**，由 Seed 暴露一个非常窄的 **Host ABI / Capability API**。

关键约束：

1. 默认不给插件完整 WASI；
2. 文件、网络、时钟、随机数等能力必须由 Seed Host 明确授予；
3. 插件无法直接访问 Core 内存；
4. 插件无法直接访问 Root Private Key；
5. 每个插件拥有独立 state namespace；
6. 所有敏感操作通过 Capability request；
7. Runtime engine 本身必须可以替换。

具体 Wasm engine **暂不冻结**。

Phase 0 至少实测：

- WAMR；
- 一个适合 Rust embedding 的小型解释器/runtime；
- 必要时以 Wasmtime 作为安全/功能参考基线。

WAMR 当前仍将自己定位为小 footprint、可嵌入且高度可配置的 WebAssembly runtime；Wasmtime 则提供成熟的 Wasm/WASI/Component Model 实现与强安全工程体系。Seed 的最终选择必须基于本项目的实际 binary-size 与 sandbox 测试，而不是宣传数据。

## Host ABI v0 原则

ABI 只暴露句柄和窄接口，例如：

```text
seed_event_subscribe(...)
seed_event_emit(...)
seed_state_get(...)
seed_state_put(...)
seed_capability_request(...)
seed_members_query(...)
seed_ui_register(...)
```

插件不得获得：

- 原始数据库连接；
- Core 指针；
- 任意 filesystem handle；
- 未声明网络 socket。

## Alternatives

### Native dynamic library

优点：

- 性能高；
- ABI 简单时调用开销小。

缺点：

- in-process 安全隔离弱；
- 崩溃可直接带走 Host；
- 第三方插件供应链风险高。

### Separate native process

优点：

- OS 级隔离；
- 可使用任意语言。

缺点：

- IPC、生命周期、分发复杂；
- 移动端与小体积场景不友好。

### 自定义 VM

优点：

- 理论上可极小；
- API 可以完全定制。

缺点：

- 自己承担 VM 安全与工具链；
- 插件开发生态差；
- 长期维护成本过高。

## Validation

Spike 必须测量：

- runtime 增加的 stripped binary size；
- hello plugin 大小；
- instantiate latency；
- 100k Host ABI calls 的开销；
- memory limit；
- infinite loop / trap；
- OOM；
- 未授权 filesystem/network access；
- plugin crash isolation。

## Revisit conditions

- 所有可接受的 Wasm runtime 都使 Core size budget 不可行；
- 目标平台无法安全或稳定运行所选 runtime；
- 后续出现更轻量且有成熟安全边界的替代方案。

## References

- https://github.com/wasm-micro-runtime/wasm-micro-runtime
- https://github.com/bytecodealliance/wasmtime
