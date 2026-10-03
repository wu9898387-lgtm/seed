use std::{
    fs,
    hint::black_box,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use seed_core::{
    event::{Event, EventHeader},
    id::SpaceId,
    identity::{DeviceIdentity, RootIdentity},
    sqlite_storage::SqliteEventStore,
    storage::{EventStore, FileEventStore},
    PROTOCOL_VERSION,
};

const SPACE_COUNT: usize = 16;
const RECENT_LIMIT: usize = 64;

#[derive(Clone, Copy)]
struct Measurement {
    append: Duration,
    reopen: Duration,
    recent_query: Duration,
    bytes: u64,
    recent_count: usize,
}

fn main() {
    let count = std::env::args()
        .nth(1)
        .map(|value| {
            value
                .parse::<usize>()
                .expect("event count must be an integer")
        })
        .unwrap_or(256);
    assert!(count > 0, "event count must be greater than zero");

    let events = build_events(count);
    let target_space = SpaceId::from_bytes([0; 32]);

    let file_path = temp_path("eventlog", count);
    let sqlite_path = temp_path("sqlite3", count);
    cleanup_file(&file_path);
    cleanup_sqlite(&sqlite_path);

    let file = measure_file_store(&file_path, &events, &target_space);
    let sqlite = measure_sqlite_store(&sqlite_path, &events, &target_space);

    println!(
        "seed-storage-compare protocol={} events={} spaces={} recent_limit={}",
        PROTOCOL_VERSION, count, SPACE_COUNT, RECENT_LIMIT
    );
    print_measurement("append-file", file, count);
    print_measurement("sqlite-full", sqlite, count);

    cleanup_file(&file_path);
    cleanup_sqlite(&sqlite_path);
}

fn build_events(count: usize) -> Vec<Event> {
    let root = RootIdentity::generate().expect("root identity");
    let device = DeviceIdentity::generate().expect("device identity");
    let authorization = root.authorize_device(&device, 1, 0);

    (0..count)
        .map(|index| {
            let space = (index % SPACE_COUNT) as u8;
            let sequence = u64::try_from(index + 1).expect("sequence fits u64");
            Event::sign(
                root.document(),
                &authorization,
                &device,
                EventHeader {
                    protocol_version: PROTOCOL_VERSION,
                    space: SpaceId::from_bytes([space; 32]),
                    author: root.document().id(),
                    device: device.id(),
                    sequence,
                    timestamp_ms: i64::try_from(sequence).expect("timestamp fits i64"),
                    schema: "seed.storage.compare/v1".to_owned(),
                },
                sequence.to_be_bytes().to_vec(),
            )
            .expect("sign comparison event")
        })
        .collect()
}

fn measure_file_store(path: &Path, events: &[Event], space: &SpaceId) -> Measurement {
    let append_started = Instant::now();
    {
        let mut store = FileEventStore::open(path).expect("open file store");
        for event in events.iter().cloned() {
            store.append(event).expect("append file event");
        }
        store.checkpoint().expect("checkpoint file store");
    }
    let append = append_started.elapsed();
    let bytes = fs::metadata(path).expect("file store metadata").len();

    let reopen_started = Instant::now();
    let store = FileEventStore::open(path).expect("reopen file store");
    let reopen = reopen_started.elapsed();
    assert_eq!(store.len(), events.len());

    let query_started = Instant::now();
    let recent = store.recent_events_for_space(space, RECENT_LIMIT);
    black_box(&recent);
    let recent_query = query_started.elapsed();

    Measurement {
        append,
        reopen,
        recent_query,
        bytes,
        recent_count: recent.len(),
    }
}

fn measure_sqlite_store(path: &Path, events: &[Event], space: &SpaceId) -> Measurement {
    let append_started = Instant::now();
    {
        let mut store = SqliteEventStore::open(path).expect("open sqlite store");
        for event in events.iter().cloned() {
            store.append(event).expect("append sqlite event");
        }
        store.checkpoint().expect("checkpoint sqlite store");
    }
    let append = append_started.elapsed();
    let bytes = fs::metadata(path).expect("sqlite metadata").len();

    let reopen_started = Instant::now();
    let store = SqliteEventStore::open(path).expect("reopen sqlite store");
    let reopen = reopen_started.elapsed();
    assert_eq!(store.len(), events.len());

    let query_started = Instant::now();
    let recent = store
        .recent_events_for_space(space, RECENT_LIMIT)
        .expect("query sqlite recent history");
    black_box(&recent);
    let recent_query = query_started.elapsed();

    Measurement {
        append,
        reopen,
        recent_query,
        bytes,
        recent_count: recent.len(),
    }
}

fn print_measurement(backend: &str, measurement: Measurement, events: usize) {
    let append_seconds = measurement.append.as_secs_f64();
    let events_per_second = if append_seconds == 0.0 {
        f64::INFINITY
    } else {
        events as f64 / append_seconds
    };

    println!(
        "backend={} append_ms={:.3} append_events_per_s={:.1} reopen_ms={:.3} recent_query_us={:.3} bytes={} recent_count={}",
        backend,
        measurement.append.as_secs_f64() * 1_000.0,
        events_per_second,
        measurement.reopen.as_secs_f64() * 1_000.0,
        measurement.recent_query.as_secs_f64() * 1_000_000.0,
        measurement.bytes,
        measurement.recent_count
    );
}

fn temp_path(extension: &str, count: usize) -> PathBuf {
    std::env::temp_dir().join(format!(
        "seed-storage-compare-{}-{count}.{extension}",
        std::process::id()
    ))
}

fn cleanup_file(path: &Path) {
    let _ = fs::remove_file(path);
}

fn cleanup_sqlite(path: &Path) {
    cleanup_file(path);

    let mut wal = path.as_os_str().to_os_string();
    wal.push("-wal");
    let _ = fs::remove_file(PathBuf::from(wal));

    let mut shm = path.as_os_str().to_os_string();
    shm.push("-shm");
    let _ = fs::remove_file(PathBuf::from(shm));
}
