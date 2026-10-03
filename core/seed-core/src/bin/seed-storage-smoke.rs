use std::fs;

use seed_core::{
    event::{Event, EventHeader},
    genesis::{GenesisDraft, GenesisPlugin, GenesisRecord},
    id::{PluginDigest, PluginId},
    identity::{DeviceIdentity, RootIdentity},
    plugin::PluginVersion,
    space::SpaceKind,
    storage::{EventStore, FileEventStore},
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
        schema: "seed.storage.smoke/v1".to_owned(),
    };

    let event = Event::sign(
        root.document(),
        &authorization,
        &device,
        header,
        b"seed".to_vec(),
    )
    .expect("signed event");
    let event_id = event.id();

    event
        .verify(root.document(), &authorization)
        .expect("verified event");

    let path = std::env::temp_dir().join(format!(
        "seed-storage-smoke-{}.eventlog",
        std::process::id()
    ));
    let _ = fs::remove_file(&path);

    {
        let mut store = FileEventStore::open(&path).expect("open event store");
        store.append(event).expect("append event");
        store.checkpoint().expect("checkpoint event store");
    }

    let store = FileEventStore::open(&path).expect("reopen event store");
    assert!(store.has_event(&event_id));
    assert_eq!(store.len(), 1);
    drop(store);

    fs::remove_file(&path).expect("remove smoke event store");

    println!(
        "seed-storage-smoke protocol={} genesis={} space={} event={}",
        PROTOCOL_VERSION,
        genesis.id(),
        genesis.space_id(),
        event_id
    );
}
