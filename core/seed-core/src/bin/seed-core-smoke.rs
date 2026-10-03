use seed_core::{
    event::{Event, EventHeader},
    genesis::{GenesisDraft, GenesisPlugin, GenesisRecord},
    id::{PluginDigest, PluginId},
    identity::{DeviceIdentity, RootIdentity},
    plugin::PluginVersion,
    space::SpaceKind,
    PROTOCOL_VERSION,
};

fn main() {
    let root = RootIdentity::generate().expect("root identity");
    let device = DeviceIdentity::generate().expect("device identity");
    let authorization = root.authorize_device(&device, 1, 0);

    let mut draft = GenesisDraft::new(SpaceKind::Group, root.document(), &device, 1);
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

    event
        .verify(root.document(), &authorization)
        .expect("verified event");

    println!(
        "seed-core protocol={} identity={} genesis={} space={} event={}",
        PROTOCOL_VERSION,
        root.document().id(),
        genesis.id(),
        genesis.space_id(),
        event.id()
    );
}
