# Seed Architecture Decision Records

ADR 用于记录 Seed 的重大技术与协议决策。

状态：

- **Provisional / Proposed**：方向已经进入实现或设计，但仍需 Spike / review / test vector 才能冻结；
- **Accepted**：成为当前实现必须遵守的决策；
- **Superseded**：被新的 ADR 替代；
- **Rejected**：明确不采用。

## 当前编号

ADR-0001 ~ ADR-0003 当前由 `core-kernel-spike` / PR #1 提供实测版本。它们在 PR 合并后进入 main：

| ADR | 主题 | 当前状态 |
|---|---|---|
| 0001 | Core language spike uses Rust | Provisional / PR #1 |
| 0002 | Root and Device signing keys | Provisional / PR #1 |
| 0003 | Narrow canonical binary encoding for signed kernel records | Provisional / PR #1 |
| [0004](0004-plugin-runtime.md) | Wasm + Seed Host ABI | Proposed |
| [0005](0005-storage-model.md) | Validated Event Log + Materialized View | Proposed |
| [0006](0006-transport.md) | Direct-first + Relay fallback | Proposed |
| [0007](0007-event-genesis.md) | Genesis root + Validated Event | Proposed |

[0000](0000-template.md) 是 ADR 模板。

## 编号规则

ADR 编号只表示决策序列，不表示模块优先级。

同一个编号不能同时代表两个不同决策。

如果实现分支已经占用编号，main 上的新设计必须顺延，避免合并后出现“两个 ADR-0002”一类歧义。

## 接受规则

ADR 从 Provisional/Proposed 进入 Accepted 前，至少满足其 Validation / Revisit 条件要求，并有可重复证据。

涉及以下内容时还需要额外安全 review：

- 密码学 primitive/profile；
- Root Identity；
- Plugin sandbox；
- Capability authorization；
- Protocol signature coverage；
- Host / Authority 边界；
- key recovery / revocation。

## 变更规则

不要直接删除已经实施或发布过的 ADR。

如果方向发生变化：

1. 创建新 ADR；
2. 旧 ADR 标记 `Superseded by ADR-XXXX`；
3. 说明迁移影响；
4. 更新 protocol/test vectors；
5. 对 wire-breaking change 明确升级版本。

## 当前原则

文档必须服从已经验证的实现事实，而不是反过来假装某个尚未验证的技术方案已经冻结。

例如当前签名内核已经有 deterministic custom binary framing 与 test vector，因此 main 的设计文档以该 Spike 为当前基线；是否在未来换成 CBOR/其他标准格式，必须通过新 ADR 与兼容性验证重新决定。
