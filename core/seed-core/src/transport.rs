use std::{
    io::{self, ErrorKind, Read, Write},
    net::{Shutdown, SocketAddr, TcpStream},
    sync::mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError},
    thread,
    time::Duration,
};

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
    InvalidFrame,
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

/// Reference TCP byte transport for Phase 0.
///
/// The socket is intentionally hidden behind the same bounded queue contract as
/// LoopbackTransport. Blocking socket I/O happens on background worker threads;
/// callers only see non-blocking queue operations through Transport.
///
/// This is a reference adapter, not a commitment to TCP as Seed's long-term
/// transport.
#[derive(Debug)]
pub struct TcpTransport {
    tx: SyncSender<TransportFrame>,
    rx: Receiver<Result<TransportFrame, TransportError>>,
    control: TcpStream,
    path: TransportPath,
}

impl TcpTransport {
    pub const DEFAULT_QUEUE_DEPTH: usize = 64;

    pub fn connect(address: SocketAddr, path: TransportPath) -> Result<Self, TransportError> {
        let stream = TcpStream::connect(address).map_err(|_| TransportError::Disconnected)?;
        Self::from_stream(stream, path)
    }

    pub fn connect_timeout(
        address: SocketAddr,
        path: TransportPath,
        timeout: Duration,
    ) -> Result<Self, TransportError> {
        let stream = TcpStream::connect_timeout(&address, timeout)
            .map_err(|_| TransportError::Disconnected)?;
        Self::from_stream(stream, path)
    }

    pub fn from_stream(stream: TcpStream, path: TransportPath) -> Result<Self, TransportError> {
        Self::from_stream_with_capacity(stream, path, Self::DEFAULT_QUEUE_DEPTH)
    }

    pub fn from_stream_with_capacity(
        stream: TcpStream,
        path: TransportPath,
        queue_depth: usize,
    ) -> Result<Self, TransportError> {
        if queue_depth == 0 {
            return Err(TransportError::Disconnected);
        }

        stream
            .set_nodelay(true)
            .map_err(|_| TransportError::Disconnected)?;

        let control = stream
            .try_clone()
            .map_err(|_| TransportError::Disconnected)?;
        let mut reader = stream
            .try_clone()
            .map_err(|_| TransportError::Disconnected)?;
        let mut writer = stream;

        let (outbound_tx, outbound_rx) = mpsc::sync_channel::<TransportFrame>(queue_depth);
        let (inbound_tx, inbound_rx) =
            mpsc::sync_channel::<Result<TransportFrame, TransportError>>(queue_depth);

        thread::spawn(move || {
            while let Ok(frame) = outbound_rx.recv() {
                if write_tcp_frame(&mut writer, frame.as_bytes()).is_err() {
                    break;
                }
            }
            let _ = writer.shutdown(Shutdown::Both);
        });

        thread::spawn(move || loop {
            match read_tcp_frame(&mut reader) {
                Ok(frame) => {
                    if inbound_tx.send(Ok(frame)).is_err() {
                        break;
                    }
                }
                Err(error) => {
                    let _ = inbound_tx.send(Err(error));
                    break;
                }
            }
        });

        Ok(Self {
            tx: outbound_tx,
            rx: inbound_rx,
            control,
            path,
        })
    }
}

impl Transport for TcpTransport {
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
            Ok(Ok(frame)) => Ok(Some(frame)),
            Ok(Err(error)) => Err(error),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(TransportError::Disconnected),
        }
    }
}

impl Drop for TcpTransport {
    fn drop(&mut self) {
        let _ = self.control.shutdown(Shutdown::Both);
    }
}

/// Attempt a Direct TCP path first and fall back to a Relay TCP endpoint.
///
/// The returned object still implements the same Transport trait. Upper layers
/// can inspect only TransportPath, not the socket endpoint that was selected.
pub fn connect_direct_or_relay(
    direct: SocketAddr,
    relay: SocketAddr,
    connect_timeout: Duration,
) -> Result<TcpTransport, TransportError> {
    TcpTransport::connect_timeout(direct, TransportPath::Direct, connect_timeout)
        .or_else(|_| TcpTransport::connect_timeout(relay, TransportPath::Relay, connect_timeout))
}

fn write_tcp_frame(stream: &mut TcpStream, bytes: &[u8]) -> io::Result<()> {
    let len = u32::try_from(bytes.len())
        .map_err(|_| io::Error::new(ErrorKind::InvalidInput, "transport frame exceeds u32"))?;
    stream.write_all(&len.to_be_bytes())?;
    stream.write_all(bytes)?;
    stream.flush()
}

fn read_tcp_frame(stream: &mut TcpStream) -> Result<TransportFrame, TransportError> {
    let mut len_bytes = [0u8; 4];
    match stream.read_exact(&mut len_bytes) {
        Ok(()) => {}
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::UnexpectedEof
                    | ErrorKind::ConnectionReset
                    | ErrorKind::ConnectionAborted
                    | ErrorKind::BrokenPipe
            ) =>
        {
            return Err(TransportError::Disconnected);
        }
        Err(_) => return Err(TransportError::Disconnected),
    }

    let len = u32::from_be_bytes(len_bytes) as usize;
    if len > MAX_TRANSPORT_FRAME_BYTES {
        return Err(TransportError::InvalidFrame);
    }

    let mut bytes = vec![0u8; len];
    if stream.read_exact(&mut bytes).is_err() {
        return Err(TransportError::InvalidFrame);
    }

    TransportFrame::new(bytes).map_err(|_| TransportError::InvalidFrame)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        net::TcpListener,
        time::{Duration, Instant},
    };

    fn send_through_erased_transport(
        transport: &dyn Transport,
        bytes: &[u8],
    ) -> Result<(), TransportError> {
        transport.try_send(TransportFrame::from_slice(bytes).expect("bounded test frame"))
    }

    fn wait_recv(transport: &dyn Transport) -> Result<TransportFrame, TransportError> {
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            match transport.try_recv() {
                Ok(Some(frame)) => return Ok(frame),
                Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
                Ok(None) => return Err(TransportError::WouldBlock),
                Err(error) => return Err(error),
            }
        }
    }

    fn wait_error(transport: &dyn Transport) -> TransportError {
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            match transport.try_recv() {
                Err(error) => return error,
                Ok(_) if Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
                Ok(_) => return TransportError::WouldBlock,
            }
        }
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

    #[test]
    fn tcp_direct_is_bidirectional_and_preserves_bytes() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            TcpTransport::from_stream(stream, TransportPath::Direct).unwrap()
        });

        let client = TcpTransport::connect(address, TransportPath::Direct).unwrap();
        let peer = server.join().unwrap();

        client
            .try_send(TransportFrame::from_slice(b"tcp-alice").unwrap())
            .unwrap();
        peer.try_send(TransportFrame::from_slice(b"tcp-bob").unwrap())
            .unwrap();

        assert_eq!(wait_recv(&peer).unwrap().as_bytes(), b"tcp-alice");
        assert_eq!(wait_recv(&client).unwrap().as_bytes(), b"tcp-bob");
    }

    #[test]
    fn direct_connect_failure_falls_back_to_relay_path() {
        let dead_listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let direct = dead_listener.local_addr().unwrap();
        drop(dead_listener);

        let relay_listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let relay = relay_listener.local_addr().unwrap();
        let accepted = thread::spawn(move || {
            let (stream, _) = relay_listener.accept().unwrap();
            TcpTransport::from_stream(stream, TransportPath::Relay).unwrap()
        });

        let selected = connect_direct_or_relay(direct, relay, Duration::from_millis(250)).unwrap();
        let relay_peer = accepted.join().unwrap();

        assert_eq!(selected.path(), TransportPath::Relay);
        selected
            .try_send(TransportFrame::from_slice(b"fallback").unwrap())
            .unwrap();
        assert_eq!(wait_recv(&relay_peer).unwrap().as_bytes(), b"fallback");
    }

    #[test]
    fn tcp_rejects_malicious_oversized_length_before_allocation() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let accepted = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            TcpTransport::from_stream(stream, TransportPath::Direct).unwrap()
        });

        let mut attacker = TcpStream::connect(address).unwrap();
        let peer = accepted.join().unwrap();
        let malicious_len = (MAX_TRANSPORT_FRAME_BYTES as u32 + 1).to_be_bytes();
        attacker.write_all(&malicious_len).unwrap();

        assert_eq!(wait_error(&peer), TransportError::InvalidFrame);
    }

    #[test]
    fn tcp_rejects_truncated_frame() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let accepted = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            TcpTransport::from_stream(stream, TransportPath::Direct).unwrap()
        });

        let mut attacker = TcpStream::connect(address).unwrap();
        let peer = accepted.join().unwrap();
        attacker.write_all(&10u32.to_be_bytes()).unwrap();
        attacker.write_all(b"abc").unwrap();
        attacker.shutdown(Shutdown::Write).unwrap();

        assert_eq!(wait_error(&peer), TransportError::InvalidFrame);
    }
}
