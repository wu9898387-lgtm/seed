use seed_core::{
    event::{Event, EventHeader},
    genesis::{GenesisDraft, GenesisPlugin},
    id::{PluginDigest, PluginId},
    identity::{DeviceIdentity, RootIdentity},
    plugin::PluginVersion,
    space::SpaceKind,
    storage::{AppendOutcome, EventStore, FileEventStore},
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
                b"storage-smoke".to_vec(),
            )
            .expect("genesis plugin"),
        )
        .expect("attach genesis plugin");

    let genesis = draft
        .activate(root.document(), &authorization, &device)
        .expect("activate genesis");

    let header = EventHeader {
        protocol_version: PROTOCOL_VERSION,
        space: genesis.space_id(),
        author: root.document().id(),
        device: device.id(),
        sequence: 1,
        timestamp_ms: 2,
        schema: "seed.storage-smoke/v1".to_owned(),
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

    let path = std::env::temp_dir().join(format!(
        "seed-storage-smoke-{}.log",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);

    {
        let mut store = FileEventStore::open(&path).expect("open event log");
        assert_eq!(
            store.append(event).expect("append event"),
            AppendOutcome::Inserted
        );
        store.sync().expect("sync event log");
    }

    {
        let store = FileEventStore::open(&path).expect("reopen event log");
        assert_eq!(store.len(), 1);
        assert_eq!(
            store.events_for_space(&genesis.space_id())[0].id(),
            event_id
        );
    }

    std::fs::remove_file(&path).expect("remove event log");

    println!(
        "seed-storage-smoke space={} event={} recovered=1",
        genesis.space_id(),
        event_id
    );
}
