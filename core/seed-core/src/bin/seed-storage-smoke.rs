use seed_core::{
    event::{Event, EventHeader},
    id::SpaceId,
    identity::{DeviceIdentity, RootIdentity},
    storage::{EventStore, FileEventStore},
    PROTOCOL_VERSION,
};

fn main() {
    let root = RootIdentity::generate().expect("root");
    let device = DeviceIdentity::generate().expect("device");
    let authorization = root.authorize_device(&device, 1, 1);

    let event = Event::sign(
        root.document(),
        &authorization,
        &device,
        EventHeader {
            protocol_version: PROTOCOL_VERSION,
            space: SpaceId::from_bytes([9; 32]),
            author: root.document().id(),
            device: device.id(),
            sequence: 1,
            timestamp_ms: 2,
            schema: "seed.storage.smoke/v1".to_owned(),
        },
        b"storage".to_vec(),
    )
    .expect("event");

    let path = std::env::temp_dir().join(format!(
        "seed-storage-smoke-{}.eventlog",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);

    {
        let mut store = FileEventStore::open(&path).expect("open store");
        store.append(event).expect("append event");
        store.sync().expect("sync store");
    }

    let reopened = FileEventStore::open(&path).expect("reopen store");
    assert_eq!(reopened.len(), 1);

    let _ = std::fs::remove_file(path);
}
