use std::{fs, net::TcpListener};

use seed_core::{
    event::{Event, EventHeader},
    genesis::{GenesisDraft, GenesisPlugin, GenesisRecord},
    id::{PluginDigest, PluginId},
    identity::{DeviceIdentity, RootIdentity},
    plugin::PluginVersion,
    space::SpaceKind,
    storage::{AppendOutcome, FileEventStore},
    transport::{FrameTransport, TcpFrameTransport},
    PROTOCOL_VERSION,
};

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
                b"smoke".to_vec(),
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

    let header = EventHeader {
        protocol_version: PROTOCOL_VERSION,
        space: genesis.space_id(),
        author: root.document().id(),
        device: device.id(),
        sequence: 1,
        timestamp_ms: 2,
        schema: "seed.smoke/v1".to_owned(),
    };

    let event = Event::sign(
        root.document(),
        &authorization,
        &device,
        header,
        b"seed".to_vec(),
    )
    .expect("signed event");

    let encoded_event = event.canonical_bytes().expect("encode event");
    let decoded_event = Event::from_canonical_bytes(&encoded_event).expect("decode event");
    decoded_event
        .verify(root.document(), &authorization)
        .expect("verified event");

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let address = listener.local_addr().expect("loopback address");
    let mut client = TcpFrameTransport::connect(address).expect("connect loopback");
    let (server_stream, _) = listener.accept().expect("accept loopback");
    let mut server = TcpFrameTransport::from_stream(server_stream);

    client
        .send_frame(&encoded_event)
        .expect("send canonical event");
    let transported_bytes = server.recv_frame().expect("receive canonical event");
    let transported_event =
        Event::from_canonical_bytes(&transported_bytes).expect("decode transported event");
    transported_event
        .verify(root.document(), &authorization)
        .expect("verify transported event");

    let log_path = std::env::temp_dir().join(format!("seed-core-smoke-{}.log", std::process::id()));
    let _ = fs::remove_file(&log_path);

    {
        let mut store = FileEventStore::open(&log_path).expect("open event log");
        assert_eq!(
            store
                .append(transported_event.clone())
                .expect("append event"),
            AppendOutcome::Inserted
        );
    }

    let reopened = FileEventStore::open(&log_path).expect("reopen event log");
    let restored = reopened.events_for_space(&genesis.space_id());
    assert_eq!(restored.len(), 1);
    restored[0]
        .verify(root.document(), &authorization)
        .expect("verify restored event");
    let _ = fs::remove_file(&log_path);

    println!(
        "seed-core protocol={} identity={} genesis={} space={} event={} stored={} transport=loopback-tcp",
        PROTOCOL_VERSION,
        root.document().id(),
        genesis.id(),
        genesis.space_id(),
        event.id(),
        reopened.len()
    );
}
