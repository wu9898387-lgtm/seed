use crate::identity::DeviceId;

/// Logical path selected for delivery.
///
/// Route selection is transport infrastructure, not governance authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Route {
    Direct,
    Relay,
    Tree,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Peer {
    pub device: DeviceId,
}

/// Transport adapter boundary used by Phase 0.
///
/// Async model, framing, authentication handshake, congestion control, NAT
/// traversal, QUIC/TCP choice, and relay protocol remain deliberately open.
pub trait Transport {
    type Error;

    fn send(&mut self, route: Route, peer: Peer, frame: &[u8]) -> Result<(), Self::Error>;
}
