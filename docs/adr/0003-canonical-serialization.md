# ADR-0003: 协议签名对象采用 Deterministic CBOR

- Status: Proposed
- Date: 2026-10-03
- Decision scope: Protocol / Genesis / Event / Identity certificates

## Context

Seed 的 Genesis、Identity certificate、治理事件和普通 Event 都需要：

- 跨平台；
- 跨语言；
- 紧凑；
- 可稳定 hash；
- 可稳定签名；
- 不依赖 JSON 文本格式细节。

同一个逻辑对象必须在不同实现中产生同一 canonical byte sequence，否则：

- Genesis Hash 不一致；
- Event ID 不一致；
- 签名无法跨实现验证。

## Decision

Seed Protocol v0 的签名/哈希对象采用 **CBOR**，并遵循 RFC 8949 Section 4.2 的 deterministic encoding 原则。

额外约束：

1. Protocol v0 的签名核心对象禁止浮点数；
2. 长度已知时只允许 definite-length；
3. map key 使用协议规定的小整数 key；
4. 不允许同义字段存在多种编码；
5. 字符串统一 UTF-8，协议字段不得依赖 locale normalization；
6. unknown extension 字段的签名语义必须明确；
7. sign/hash 的输入永远是 canonical bytes，不是内存对象。

建议：

```text
logical object
  -> schema validation
  -> deterministic CBOR
  -> domain-separated hash/signature
```

## Domain separation

不同对象类型必须使用不同签名前缀/上下文，例如：

```
seed:v0:identity:
seed:v0:genesis:
seed:v0:event:
seed:v0:device-cert:
```

具体二进制 framing 后续在 protocol spec 固定。

## Alternatives

### JSON / Canonical JSON

优点：

- 可读；
- 工具丰富。

缺点：

- 体积较大；
- 数字/Unicode/canonicalization 规则更容易被实现差异影响。

### Protobuf

优点：

- 生态成熟；
- schema 清晰。

缺点：

- deterministic serialization 和未知字段/签名边界需要非常谨慎定义；
- 对 Seed 的“小协议对象 + hash identity”并无决定性优势。

### 自定义二进制格式

优点：

- 可以极致紧凑。

缺点：

- 解析器、兼容性与安全负担全部自己承担。

## Validation

必须提供跨实现 test vectors：

- canonical hex；
- object hash；
- valid signature；
- malformed encodings；
- non-deterministic equivalent encodings 必须拒绝或 canonicalize 后处理。

## Revisit conditions

只有出现以下情况才重新评估：

- CBOR 实现体积显著破坏预算；
- deterministic behavior 在目标语言间出现不可解决差异；
- 协议生态有更强的标准化需求。

## References

- https://www.rfc-editor.org/rfc/rfc8949.html
