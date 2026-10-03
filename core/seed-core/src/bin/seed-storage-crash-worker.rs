use std::{
    env,
    io::{self, Write},
    path::PathBuf,
    thread,
};

use seed_core::{
    event::{Event, EventHeader},
    id::SpaceId,
    identity::{DeviceIdentity, RootIdentity},
    sqlite_storage::SqliteEventStore,
    storage::{EventStore, FileEventStore},
    PROTOCOL_VERSION,
};

fn main() {
    let mut args = env::args().skip(1);
    let backend = args.next().expect("backend");
    let path = PathBuf::from(args.next().expect("path"));
    let count = args
        .next()
        .expect("count")
        .parse::<usize>()
        .expect("count is usize");
    assert!(args.next().is_none(), "unexpected extra arguments");

    let root = RootIdentity::from_secret_bytes([0x11; 32]);
    let device = DeviceIdentity::from_secret_bytes([0x22; 32]);
    let authorization = root.authorize_device(&device, 1, 0);

    match backend.as_str() {
        "append-file" => {
            let mut store = FileEventStore::open(&path).expect("open append-file store");
            append_events(&mut store, count, &root, &device, &authorization);
        }
        "sqlite" => {
            let mut store = SqliteEventStore::open(&path).expect("open sqlite store");
            append_events(&mut store, count, &root, &device, &authorization);
        }
        other => panic!("unsupported backend: {other}"),
    }

    println!("ready backend={backend} count={count}");
    io::stdout().flush().expect("flush ready marker");

    // The parent integration test kills this process after it observes the
    // marker. Deliberately never checkpoint or cleanly drop the store.
    loop {
        thread::park();
    }
}

fn append_events(
    store: &mut dyn EventStore,
    count: usize,
    root: &RootIdentity,
    device: &DeviceIdentity,
    authorization: &seed_core::identity::DeviceAuthorization,
) {
    for index in 0..count {
        let sequence = u64::try_from(index + 1).expect("count fits u64");
        let event = Event::sign(
            root.document(),
            authorization,
            device,
            EventHeader {
                protocol_version: PROTOCOL_VERSION,
                space: SpaceId::from_bytes([9; 32]),
                author: root.document().id(),
                device: device.id(),
                sequence,
                timestamp_ms: i64::try_from(sequence).expect("sequence fits i64"),
                schema: "seed.storage.process-kill/v1".to_owned(),
            },
            vec![u8::try_from(index % 251).expect("payload byte"); 16],
        )
        .expect("sign event");

        store.append(event).expect("durable append");
    }
}
