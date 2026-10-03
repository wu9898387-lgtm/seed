# ADR-0004: 插件采用沙箱化 WebAssembly + Seed Host ABI

- Status: Proposed
- Date: 2026-10-03
- Decision scope: Plugin Runtime / Plugin SDK

## Context

Seed 的核心产品原则是 Plugin-first。

插件既可能实现普通功能，也可能参与 Governance、Meta-Governance、Moderation、UI、Bot 和 Tree behavior，因此插件必须被视为真正的安全边界，而不是“可信脚本”。

Native in-process 插件如果与 Core 共享完整地址空间，会直接削弱：

- 私钥隔离；
- scoped storage；
- Capability enforcement；
- crash isolation；
- 第三方插件的供应链边界。

## Decision

插件格式优先采用 **WebAssembly module**，由 Seed 暴露一个尽量窄的 **Host ABI / Capability API**。

关键约束：

1. 默认不给插件完整 WASI；
2. 文件、网络、时钟、随机数等能力必须由 Seed Host 明确授予；
3. 插件不能直接访问 Core 内存；
4. 插件不能直接访问 Root Private Key；
5. 每个插件拥有独立 state namespace；
6. 所有敏感动作通过 Capability request；
7. Runtime engine 本身必须可替换；
8. 插件 package bytes 必须由历史中的 digest 固定，升级必须产生治理事件。

具体 Wasm engine **暂不冻结**。

Phase 0 至少比较：

- WAMR 或同类小 footprint runtime；
- 一个适合 Rust embedding 的轻量 runtime；
- Wasmtime 作为功能、安全边界和工具链参考基线。

最终选择必须依据 Seed 自己的 stripped binary-size、资源限制和 sandbox 测试，不依据宣传数据。

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
- Core 内部指针；
- 任意 filesystem handle；
- 未声明 network socket；
- Root/Device 私钥字节。

## Alternatives

### Native dynamic library

优点：

- 性能高；
- 调用成本低。

缺点：

- in-process 隔离弱；
- 插件崩溃可直接影响 Host；
- 第三方插件供应链风险高。

### Separate native process

优点：

- OS 级隔离；
- 可使用任意语言。

缺点：

- IPC、生命周期、分发复杂；
- 移动端和小体积场景成本较高。

### 自定义 VM

优点：

- 理论上可做到很小；
- API 可完全定制。

缺点：

- 需要自己承担 VM 安全、工具链和长期维护；
- 插件开发生态较差。

## Validation

Spike 必须记录：

- runtime 增加的 stripped binary size；
- hello plugin 大小；
- instantiate latency；
- Host ABI 调用开销；
- peak RSS；
- memory limit；
- infinite loop / trap；
- OOM；
- 未授权 filesystem/network access；
- foreign plugin state access；
- plugin crash isolation。

## Acceptance gate

任何 runtime 只要无法可靠限制插件资源与权限，就不能因为“体积更小”而被接受。

## Revisit conditions

- 所有可接受的 Wasm runtime 都使 Core size budget 明显不可行；
- 目标平台无法安全或稳定运行所选 runtime；
- 后续出现更轻量且具有成熟安全边界的替代方案。
