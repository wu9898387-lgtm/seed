use seed_core::{
    event::{Event, EventHeader},
    genesis::{GenesisDraft, GenesisPlugin},
    id::{PluginDigest, PluginId},
    identity::{DeviceIdentity, RootIdentity},
    plugin::PluginVersion,
    space::SpaceKind,
    PROTOCOL_VERSION,
};

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use core::fmt::Write as _;
        write!(&mut out, "{byte:02x}").expect("hex write");
    }
    out
}

fn main() {
    let root = RootIdentity::from_secret_bytes([1u8; 32]);
    let device = DeviceIdentity::from_secret_bytes([2u8; 32]);
    let authorization = root.authorize_device(&device, 1, 1_700_000_000_000);

    let mut draft = GenesisDraft::new(
        SpaceKind::Group,
        root.document(),
        &device,
        1_700_000_000_100,
        [3u8; 16],
    );
    draft
        .add_plugin(
            GenesisPlugin::new(
                PluginId::from_bytes([5u8; 32]),
                PluginVersion {
                    major: 1,
                    minor: 2,
                    patch: 3,
                },
                PluginDigest::from_bytes([6u8; 32]),
                b"owner=creator".to_vec(),
            )
            .expect("plugin"),
        )
        .expect("attach plugin");

    let genesis = draft
        .activate(root.document(), &authorization, &device)
        .expect("genesis");

    let header = EventHeader {
        protocol_version: PROTOCOL_VERSION,
        space: genesis.space_id(),
        author: root.document().id(),
        device: device.id(),
        sequence: 7,
        timestamp_ms: 1_700_000_000_123,
        schema: "seed.message.text/v1".to_owned(),
    };

    let event = Event::sign(
        root.document(),
        &authorization,
        &device,
        header,
        b"hello".to_vec(),
    )
    .expect("event");

    println!(
        "root_public={}",
        hex(root.document().root_public_key().as_bytes())
    );
    println!("identity_id={}", root.document().id());
    println!("device_public={}", hex(device.public_key().as_bytes()));
    println!("device_id={}", device.id());
    println!(
        "device_authorization_signature={}",
        hex(authorization.root_signature().as_bytes())
    );
    println!("genesis_id={}", genesis.id());
    println!("space_id={}", genesis.space_id());
    println!("genesis_signature={}", hex(genesis.signature().as_bytes()));
    println!(
        "genesis_bytes={}",
        hex(&genesis.canonical_bytes().expect("encode genesis"))
    );
    println!("event_id={}", event.id());
    println!("event_signature={}", hex(event.signature().as_bytes()));
}
