# ADR-0005: Direct-first Transport，Relay 为一等 fallback

- Status: Proposed
- Date: 2026-10-03
- Decision scope: Transport / Direct / Group / Tree

## Context

Seed 希望让用户设备直接参与网络，但现实环境存在：

- NAT；
- CGNAT；
- 防火墙；
- 企业网络；
- 手机后台休眠；
- 不希望向陌生成员直接暴露 endpoint 的隐私需求。

因此 Seed 不能把“P2P”误实现成“任何时候都必须直接 socket 互联”。

## Decision

上层只依赖统一 Transport interface。

路径至少有：

```
Direct secure path
Relay secure path
Tree host path
```

原则：

1. **Direct-first，不是 Direct-only**；
2. Relay 是协议一等公民，不拥有社会治理权；
3. Relay 默认只处理端到端密文 envelope；
4. 上层 Space/Event 语义不依赖 TCP/QUIC/WebRTC；
5. Group 治理不依赖网络拓扑；
6. App/UI 默认不把 Peer IP 当作身份信息；
7. Transport 能报告 path type，但不应把底层 endpoint 广泛暴露给插件。

## MVP

Phase 0/1 可以先使用简单的可靠字节流路径完成协议：

- localhost / TCP direct spike；
- TCP relay spike。

这只是 reference transport，不代表长期网络协议冻结为 TCP。

QUIC、NAT traversal、UDP hole punching 等在 Core 模型稳定以后再加入。

## Secure session

安全会话不依赖 transport 自带 TLS 身份语义。

Seed 需要自己的 Identity/Device authentication。

推荐基于成熟的 Noise Protocol Framework 设计 handshake profile；具体 pattern 在 Crypto ADR / protocol spec 中冻结。

## Alternatives

### Direct-only

拒绝。实际网络可用性不足，且会迫使用户暴露网络信息。

### Central server-only

不符合 Direct / Group 的目标架构，并把服务可用性和数据路径重新集中化。

### 第一版直接完整 WebRTC

功能丰富，但依赖与体积成本过高，不适合作为最小 Core 的首个证明路径。

## Validation

- direct connect；
- direct fail -> relay fallback；
- relay 无明文；
- connection migration/reconnect 上层消息语义不变；
- endpoint metadata 不泄漏到无权限插件；
- transport adapter 可替换。

## Revisit conditions

- NAT traversal 成为 MVP 必要条件；
- 移动平台要求完全不同的后台模型；
- 新 transport 能在体积与可用性上提供显著优势。

## References

- https://noiseprotocol.org/
