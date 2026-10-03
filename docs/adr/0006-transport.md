# ADR-0006: Direct-first Transport，Relay 为一等 fallback

- Status: Proposed
- Date: 2026-10-03
- Decision scope: Transport / Direct / Group / Tree

## Context

Seed 希望用户设备直接参与网络，但现实环境存在：

- NAT；
- CGNAT；
- 防火墙；
- 企业网络；
- 手机后台休眠；
- 用户不希望向陌生成员直接暴露 endpoint 的隐私需求。

因此 Seed 不能把“P2P”误实现成“所有场景都必须直接 socket 互联”。

## Decision

上层只依赖统一 Transport abstraction。

至少支持三类路径：

```
Direct secure path
Relay secure path
Tree Host path
```

原则：

1. **Direct-first，不是 Direct-only**；
2. Relay 是协议一等公民，但不拥有社会治理权；
3. Relay 默认只处理端到端密文 envelope；
4. Space/Event 语义不依赖 TCP/QUIC/WebRTC；
5. Group 治理不依赖网络拓扑；
6. App/UI 默认不把 Peer IP 当作身份信息；
7. Transport 可以报告 path type，但底层 endpoint 不默认暴露给插件。

## MVP reference path

Phase 0/1 可以先用简单可靠字节流完成协议验证：

- Loopback；
- TCP direct；
- TCP relay。

这只是 reference transport，不代表长期网络协议被冻结为 TCP。

QUIC、NAT traversal、UDP hole punching 等在 Core 模型稳定后再进入独立 Spike。

## Secure session

Seed 的密码学身份不能直接等同于 transport 自带 TLS 服务器身份。

需要 Seed 自己的 Identity/Device authenticated session。

具体 key agreement / Noise pattern 在后续独立 ADR 中冻结；不得自行发明握手算法。

## Alternatives

### Direct-only

拒绝。

原因：

- 实际网络可用性不足；
- 会迫使更多用户暴露 endpoint；
- 对移动端不现实。

### Central server-only

拒绝作为 Direct/Group 的唯一架构，因为它会重新集中数据路径与服务可用性。

### 第一版直接完整 WebRTC

暂不采用作为最小 Core 起点，主要因为依赖、体积和复杂度成本较大。

## Validation

- direct connect；
- forced direct failure -> relay fallback；
- relay 无明文；
- reconnect 后上层消息语义不变；
- endpoint metadata 不泄漏给无权限插件；
- transport adapter 可替换；
- malformed/truncated/oversized frame tests。

## Revisit conditions

- NAT traversal 成为 MVP 必需条件；
- 移动平台要求明显不同的后台模型；
- 新 transport 能在体积与可用性上提供显著优势。
