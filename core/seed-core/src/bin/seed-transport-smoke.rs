use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    thread,
    time::{Duration, Instant},
};

use seed_core::{
    event::{Event, EventHeader},
    genesis::{GenesisDraft, GenesisPlugin, GenesisRecord},
    id::{PluginDigest, PluginId},
    identity::{DeviceIdentity, RootIdentity},
    plugin::PluginVersion,
    space::SpaceKind,
    transport::{
        connect_direct_or_relay, LoopbackTransport, TcpTransport, Transport, TransportFrame,
        TransportPath, MAX_TRANSPORT_FRAME_BYTES,
    },
    PROTOCOL_VERSION,
};

fn send(transport: &dyn Transport, bytes: &[u8]) {
    transport
        .try_send(TransportFrame::from_slice(bytes).expect("bounded frame"))
        .expect("send");
}

fn wait_recv(transport: &dyn Transport) -> TransportFrame {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match transport.try_recv() {
            Ok(Some(frame)) => return frame,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
            Ok(None) => panic!("transport receive timed out"),
            Err(error) => panic!("transport receive failed: {error:?}"),
        }
    }
}

fn read_raw_frame(stream: &mut TcpStream) -> Vec<u8> {
    let mut len_bytes = [0u8; 4];
    stream
        .read_exact(&mut len_bytes)
        .expect("read frame length");
    let len = u32::from_be_bytes(len_bytes) as usize;
    assert!(len <= MAX_TRANSPORT_FRAME_BYTES);
    let mut bytes = vec![0u8; len];
    stream.read_exact(&mut bytes).expect("read frame body");
    bytes
}

fn write_raw_frame(stream: &mut TcpStream, bytes: &[u8]) {
    let len = u32::try_from(bytes.len()).expect("bounded frame");
    stream
        .write_all(&len.to_be_bytes())
        .expect("write frame length");
    stream.write_all(bytes).expect("write frame body");
    stream.flush().expect("flush frame");
}

fn main() {
    let root = RootIdentity::generate().expect("root identity");
    let device = DeviceIdentity::generate().expect("device identity");
    let authorization = root.authorize_device(&device, 1, 0);

    let mut draft = GenesisDraft::new(SpaceKind::Group, root.document(), &device, 1, [9; 16]);
    draft
        .add_plugin(
            GenesisPlugin::new(
                PluginId::from_bytes([7; 32]),
                PluginVersion {
                    major: 0,
                    minor: 1,
                    patch: 0,
                },
                PluginDigest::from_bytes([8; 32]),
                b"transport-smoke".to_vec(),
            )
            .expect("genesis plugin"),
        )
        .expect("attach genesis plugin");

    let genesis = draft
        .activate(root.document(), &authorization, &device)
        .expect("activate genesis");
    let encoded_genesis = genesis.canonical_bytes().expect("encode genesis");
    let decoded_genesis =
        GenesisRecord::from_canonical_bytes(&encoded_genesis).expect("decode genesis");
    decoded_genesis
        .verify(root.document(), &authorization)
        .expect("verify genesis");

    let event = Event::sign(
        root.document(),
        &authorization,
        &device,
        EventHeader {
            protocol_version: PROTOCOL_VERSION,
            space: genesis.space_id(),
            author: root.document().id(),
            device: device.id(),
            sequence: 1,
            timestamp_ms: 2,
            schema: "seed.transport.smoke/v1".to_owned(),
        },
        b"seed".to_vec(),
    )
    .expect("signed event");

    event
        .verify(root.document(), &authorization)
        .expect("verify event");

    let wire = event.canonical_bytes().expect("encode event wire");

    // 1. Existing in-process abstraction.
    let (alice_loopback, bob_loopback) = LoopbackTransport::pair();
    send(&alice_loopback, &wire);
    let loopback_received = wait_recv(&bob_loopback);
    assert_eq!(loopback_received.as_bytes(), wire.as_slice());

    // 2. Real localhost Direct TCP path.
    let direct_listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind direct listener");
    let direct_address = direct_listener.local_addr().expect("direct address");
    let direct_server = thread::spawn(move || {
        let (mut stream, _) = direct_listener.accept().expect("accept direct");
        read_raw_frame(&mut stream)
    });

    let alice_direct =
        TcpTransport::connect(direct_address, TransportPath::Direct).expect("connect direct");
    send(&alice_direct, &wire);
    let direct_received = direct_server.join().expect("direct server thread");
    assert_eq!(direct_received, wire);

    // 3. Forced Direct failure -> actual TCP relay bridge.
    let dead_listener = TcpListener::bind(("127.0.0.1", 0)).expect("reserve dead direct port");
    let dead_direct = dead_listener.local_addr().expect("dead direct address");
    drop(dead_listener);

    let relay_listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind relay");
    let relay_address = relay_listener.local_addr().expect("relay address");
    let relay_server = thread::spawn(move || {
        let (mut source, _) = relay_listener.accept().expect("accept relay source");
        let (mut destination, _) = relay_listener.accept().expect("accept relay destination");
        let frame = read_raw_frame(&mut source);
        write_raw_frame(&mut destination, &frame);
        frame
    });

    let alice_relay =
        connect_direct_or_relay(dead_direct, relay_address, Duration::from_millis(250))
            .expect("fallback to relay");
    assert_eq!(alice_relay.path(), TransportPath::Relay);
    let bob_relay =
        TcpTransport::connect(relay_address, TransportPath::Relay).expect("connect relay receiver");

    send(&alice_relay, &wire);
    let relay_received = wait_recv(&bob_relay);
    let relay_observed = relay_server.join().expect("relay server thread");

    assert_eq!(relay_observed, wire);
    assert_eq!(relay_received.as_bytes(), wire.as_slice());

    let decoded_event =
        Event::from_canonical_bytes(relay_received.as_bytes()).expect("decode relayed event wire");
    decoded_event
        .verify(root.document(), &authorization)
        .expect("verify relayed event");

    println!(
        "seed-transport-smoke protocol={} loopback={:?} direct={:?} fallback={:?} space={} event={} bytes={}",
        PROTOCOL_VERSION,
        alice_loopback.path(),
        alice_direct.path(),
        alice_relay.path(),
        genesis.space_id(),
        event.id(),
        wire.len()
    );
}
