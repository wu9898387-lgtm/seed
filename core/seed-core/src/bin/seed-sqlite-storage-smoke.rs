use std::{
    fs,
    path::{Path, PathBuf},
};

use seed_core::{
    event::{Event, EventHeader},
    id::SpaceId,
    identity::{DeviceIdentity, RootIdentity},
    sqlite_storage::SqliteEventStore,
    storage::EventStore,
    PROTOCOL_VERSION,
};

fn main() {
    let root = RootIdentity::generate().expect("root identity");
    let device = DeviceIdentity::generate().expect("device identity");
    let authorization = root.authorize_device(&device, 1, 0);

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
            schema: "seed.storage.sqlite-smoke/v1".to_owned(),
        },
        b"sqlite".to_vec(),
    )
    .expect("signed event");
    let event_id = event.id();

    let path = std::env::temp_dir().join(format!(
        "seed-sqlite-storage-smoke-{}.sqlite3",
        std::process::id()
    ));
    cleanup(&path);

    {
        let mut store = SqliteEventStore::open(&path).expect("open sqlite store");
        store.append(event).expect("append sqlite event");
        store.checkpoint().expect("checkpoint sqlite store");
    }

    {
        let store = SqliteEventStore::open(&path).expect("reopen sqlite store");
        assert_eq!(store.len(), 1);
        assert!(store.has_event(&event_id));

        let recent = store
            .recent_events_for_space(&SpaceId::from_bytes([9; 32]), 8)
            .expect("query recent sqlite history");
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].id(), event_id);
    }

    let db_bytes = fs::metadata(&path).expect("sqlite db metadata").len();
    cleanup(&path);

    println!(
        "seed-sqlite-storage-smoke protocol={} event={} db_bytes={}",
        PROTOCOL_VERSION, event_id, db_bytes
    );
}

fn cleanup(path: &Path) {
    let _ = fs::remove_file(path);

    let mut wal = path.as_os_str().to_os_string();
    wal.push("-wal");
    let _ = fs::remove_file(PathBuf::from(wal));

    let mut shm = path.as_os_str().to_os_string();
    shm.push("-shm");
    let _ = fs::remove_file(PathBuf::from(shm));
}
