use seed_core::{
    event::{Event, EventHeader},
    genesis::{GenesisDraft, GenesisPlugin, GenesisRecord},
    id::{PluginDigest, PluginId},
    identity::{DeviceIdentity, RootIdentity},
    plugin::PluginVersion,
    space::SpaceKind,
    transport::{LoopbackTransport, Transport, TransportFrame},
    PROTOCOL_VERSION,
};

fn send(transport: &dyn Transport, bytes: &[u8]) {
    transport
        .try_send(TransportFrame::from_slice(bytes).expect("bounded frame"))
        .expect("send");
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
    let (alice, bob) = LoopbackTransport::pair();

    send(&alice, &wire);

    let received = bob.try_recv().expect("receive").expect("frame available");
    assert_eq!(received.as_bytes(), wire.as_slice());

    let decoded_event =
        Event::from_canonical_bytes(received.as_bytes()).expect("decode event wire");
    decoded_event
        .verify(root.document(), &authorization)
        .expect("verify transported event");

    println!(
        "seed-transport-smoke protocol={} path={:?} space={} event={} bytes={}",
        PROTOCOL_VERSION,
        alice.path(),
        genesis.space_id(),
        event.id(),
        wire.len()
    );
}
