# Seed Architecture Decision Records

ADR 用于记录 Seed 的重大技术与协议决策。

状态：

- **Proposed**：方向已经提出，但仍需 Spike / review / test vector 才能接受；
- **Accepted**：已经成为当前实现必须遵守的决策；
- **Superseded**：被新的 ADR 替代；
- **Rejected**：方案明确不采用。

## 当前 ADR

| ADR | 主题 | 状态 |
|---|---|---|
| [0000](0000-template.md) | ADR 模板 | Template |
| [0001](0001-core-language.md) | Reference Core 使用 Rust | Proposed |
| [0002](0002-plugin-runtime.md) | Wasm + Seed Host ABI | Proposed |
| [0003](0003-canonical-serialization.md) | Deterministic CBOR | Proposed |
| [0004](0004-storage-model.md) | Event Log + Materialized View | Proposed |
| [0005](0005-transport.md) | Direct-first + Relay fallback | Proposed |
| [0006](0006-identity-and-crypto.md) | Root Identity / Device Keys | Proposed |
| [0007](0007-event-genesis.md) | Genesis 根 + Validated Event | Proposed |

## 接受规则

ADR 从 Proposed 变成 Accepted 前至少满足其 Validation 部分。

涉及以下内容时，还需要额外安全 review：

- 密码学 primitive/profile；
- Root Identity；
- Plugin sandbox；
- Capability authorization；
- Protocol signature coverage；
- Host / Authority 边界。

## 变更规则

不要直接删除已经实施过的 ADR。

如果方向发生变化：

1. 创建新 ADR；
2. 在旧 ADR 标记 `Superseded by ADR-XXXX`；
3. 说明迁移影响；
4. 更新 protocol/test vectors。
