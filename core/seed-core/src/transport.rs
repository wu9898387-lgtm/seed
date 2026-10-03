use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError};

/// Maximum payload carried by one transport frame in the current spike.
///
/// This is a defensive transport limit, not a promise that every higher-level
/// Seed message type may use the full amount.
pub const MAX_TRANSPORT_FRAME_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportPath {
    Direct,
    Relay,
    TreeHost,
}

/// A transport payload that has already passed the generic frame-size gate.
///
/// TransportFrame deliberately contains bytes only. Peer network endpoints are
/// not part of the value and therefore cannot accidentally become identity
/// metadata in upper layers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransportFrame(Vec<u8>);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameError {
    TooLarge,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportConfigError {
    ZeroQueueDepth,
}

impl TransportFrame {
    pub fn new(bytes: Vec<u8>) -> Result<Self, FrameError> {
        if bytes.len() > MAX_TRANSPORT_FRAME_BYTES {
            return Err(FrameError::TooLarge);
        }
        Ok(Self(bytes))
    }

    pub fn from_slice(bytes: &[u8]) -> Result<Self, FrameError> {
        if bytes.len() > MAX_TRANSPORT_FRAME_BYTES {
            return Err(FrameError::TooLarge);
        }
        Ok(Self(bytes.to_vec()))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportError {
    WouldBlock,
    Disconnected,
}

/// Connected-session transport boundary used by upper Seed layers.
///
/// Identity authentication and end-to-end session crypto sit above this raw
/// byte transport. A concrete adapter may use TCP, QUIC, a relay, or a Tree
/// Host path without changing Space/Event semantics.
///
/// The error type is deliberately shared across adapters so callers can erase
/// the concrete adapter behind `dyn Transport` without coupling session logic
/// to Loopback/TCP/Relay-specific error enums.
pub trait Transport {
    fn path(&self) -> TransportPath;
    fn try_send(&self, frame: TransportFrame) -> Result<(), TransportError>;
    fn try_recv(&self) -> Result<Option<TransportFrame>, TransportError>;
}

/// In-process connected transport used for protocol and multi-node tests.
///
/// The bounded queue is intentional: even the test transport should expose
/// backpressure instead of growing memory without limit.
#[derive(Debug)]
pub struct LoopbackTransport {
    tx: SyncSender<TransportFrame>,
    rx: Receiver<TransportFrame>,
    path: TransportPath,
}

impl LoopbackTransport {
    pub const DEFAULT_QUEUE_DEPTH: usize = 64;

    pub fn pair() -> (Self, Self) {
        Self::pair_with_capacity(Self::DEFAULT_QUEUE_DEPTH)
            .expect("default loopback queue depth is non-zero")
    }

    pub fn pair_with_capacity(queue_depth: usize) -> Result<(Self, Self), TransportConfigError> {
        if queue_depth == 0 {
            return Err(TransportConfigError::ZeroQueueDepth);
        }

        let (a_tx, a_rx) = mpsc::sync_channel(queue_depth);
        let (b_tx, b_rx) = mpsc::sync_channel(queue_depth);

        Ok((
            Self {
                tx: a_tx,
                rx: b_rx,
                path: TransportPath::Direct,
            },
            Self {
                tx: b_tx,
                rx: a_rx,
                path: TransportPath::Direct,
            },
        ))
    }
}

impl Transport for LoopbackTransport {
    fn path(&self) -> TransportPath {
        self.path
    }

    fn try_send(&self, frame: TransportFrame) -> Result<(), TransportError> {
        self.tx.try_send(frame).map_err(|error| match error {
            TrySendError::Full(_) => TransportError::WouldBlock,
            TrySendError::Disconnected(_) => TransportError::Disconnected,
        })
    }

    fn try_recv(&self) -> Result<Option<TransportFrame>, TransportError> {
        match self.rx.try_recv() {
            Ok(frame) => Ok(Some(frame)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(TransportError::Disconnected),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn send_through_erased_transport(
        transport: &dyn Transport,
        bytes: &[u8],
    ) -> Result<(), TransportError> {
        transport.try_send(TransportFrame::from_slice(bytes).expect("bounded test frame"))
    }

    #[test]
    fn frame_enforces_size_limit() {
        let too_large = vec![0; MAX_TRANSPORT_FRAME_BYTES + 1];
        assert_eq!(TransportFrame::new(too_large), Err(FrameError::TooLarge));
    }

    #[test]
    fn loopback_is_bidirectional_and_preserves_bytes() {
        let (alice, bob) = LoopbackTransport::pair();

        alice
            .try_send(TransportFrame::from_slice(b"alice").unwrap())
            .unwrap();
        bob.try_send(TransportFrame::from_slice(b"bob").unwrap())
            .unwrap();

        assert_eq!(bob.try_recv().unwrap().unwrap().as_bytes(), b"alice");
        assert_eq!(alice.try_recv().unwrap().unwrap().as_bytes(), b"bob");
    }

    #[test]
    fn transport_can_be_used_through_trait_object() {
        let (alice, bob) = LoopbackTransport::pair();
        let erased: &dyn Transport = &alice;

        send_through_erased_transport(erased, b"opaque adapter").unwrap();

        assert_eq!(
            bob.try_recv().unwrap().unwrap().as_bytes(),
            b"opaque adapter"
        );
    }

    #[test]
    fn loopback_reports_direct_path_without_endpoint_metadata() {
        let (alice, _) = LoopbackTransport::pair();
        assert_eq!(alice.path(), TransportPath::Direct);
    }

    #[test]
    fn loopback_exposes_backpressure() {
        let (alice, _bob) = LoopbackTransport::pair_with_capacity(1).unwrap();

        alice
            .try_send(TransportFrame::from_slice(b"one").unwrap())
            .unwrap();
        assert_eq!(
            alice.try_send(TransportFrame::from_slice(b"two").unwrap()),
            Err(TransportError::WouldBlock)
        );
    }

    #[test]
    fn zero_queue_depth_is_rejected() {
        assert!(matches!(
            LoopbackTransport::pair_with_capacity(0),
            Err(TransportConfigError::ZeroQueueDepth)
        ));
    }

    #[test]
    fn dropping_peer_disconnects_transport() {
        let (alice, bob) = LoopbackTransport::pair();
        drop(bob);

        assert_eq!(
            alice.try_send(TransportFrame::from_slice(b"hello").unwrap()),
            Err(TransportError::Disconnected)
        );
        assert_eq!(alice.try_recv(), Err(TransportError::Disconnected));
    }
}
