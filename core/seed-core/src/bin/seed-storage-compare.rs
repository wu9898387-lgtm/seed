use std::{
    env, fs,
    path::{Path, PathBuf},
    time::Instant,
};

use seed_core::{
    event::{Event, EventHeader},
    id::SpaceId,
    identity::{DeviceAuthorization, DeviceIdentity, RootIdentity},
    sqlite_storage::SqliteEventStore,
    storage::{EventStore, FileEventStore},
    PROTOCOL_VERSION,
};

const TARGET_SPACE: u8 = 9;
const RECENT_LIMIT: usize = 64;

fn main() {
    let counts = benchmark_counts();

    let root = RootIdentity::generate().expect("root identity");
    let device = DeviceIdentity::generate().expect("device identity");
    let authorization = root.authorize_device(&device, 1, 0);

    println!(
        "storage-bench contract=v1 protocol={} recent_limit={} counts={:?}",
        PROTOCOL_VERSION, RECENT_LIMIT, counts
    );

    for count in counts {
        let generation_started = Instant::now();
        let events = build_events(count, &root, &device, &authorization);
        let generation_ms = generation_started.elapsed().as_millis();
        println!(
            "storage-bench generated count={} generation_ms={}",
            count, generation_ms
        );

        run_file_backend(count, &events);
        run_sqlite_backend(count, &events);
    }
}

fn benchmark_counts() -> Vec<usize> {
    let raw = env::var("SEED_STORAGE_BENCH_COUNTS").unwrap_or_else(|_| "10000".to_owned());
    let counts: Vec<_> = raw
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            value
                .parse::<usize>()
                .expect("SEED_STORAGE_BENCH_COUNTS must be comma-separated positive integers")
        })
        .filter(|count| *count > 0)
        .collect();

    assert!(
        !counts.is_empty(),
        "SEED_STORAGE_BENCH_COUNTS must contain at least one positive integer"
    );
    counts
}

fn build_events(
    count: usize,
    root: &RootIdentity,
    device: &DeviceIdentity,
    authorization: &DeviceAuthorization,
) -> Vec<Event> {
    (0..count)
        .map(|index| {
            let sequence = u64::try_from(index + 1).expect("benchmark count fits u64");
            let space_byte = if index % 4 == 0 {
                TARGET_SPACE
            } else {
                u8::try_from((index % 3) + 1).expect("space byte")
            };

            Event::sign(
                root.document(),
                authorization,
                device,
                EventHeader {
                    protocol_version: PROTOCOL_VERSION,
                    space: SpaceId::from_bytes([space_byte; 32]),
                    author: root.document().id(),
                    device: device.id(),
                    sequence,
                    timestamp_ms: i64::try_from(sequence).expect("sequence fits i64"),
                    schema: "seed.storage.bench/v1".to_owned(),
                },
                vec![u8::try_from(index % 251).expect("payload byte"); 32],
            )
            .expect("signed benchmark event")
        })
        .collect()
}

fn run_file_backend(count: usize, events: &[Event]) {
    let path = temp_path("append-file", count, "eventlog");
    let _ = fs::remove_file(&path);

    let mut store = FileEventStore::open(&path).expect("open append-file store");
    let append_started = Instant::now();
    for event in events {
        store.append(event.clone()).expect("append file event");
    }
    store.checkpoint().expect("checkpoint append-file store");
    let append_ms = append_started.elapsed().as_millis();
    drop(store);

    let bytes = fs::metadata(&path).expect("append-file metadata").len();

    let reopen_started = Instant::now();
    let store = FileEventStore::open(&path).expect("reopen append-file store");
    let reopen_ms = reopen_started.elapsed().as_millis();
    assert_eq!(store.len(), count);

    let target = SpaceId::from_bytes([TARGET_SPACE; 32]);
    let query_started = Instant::now();
    let matching = store.events_for_space(&target);
    let recent_start = matching.len().saturating_sub(RECENT_LIMIT);
    std::hint::black_box(&matching[recent_start..]);
    let query_us = query_started.elapsed().as_micros();
    let recent_count = matching.len().min(RECENT_LIMIT);
    drop(store);

    fs::remove_file(&path).expect("remove append-file benchmark");

    println!(
        "storage-bench backend=append-file count={} append_ms={} reopen_ms={} recent_query_us={} recent_count={} bytes={}",
        count, append_ms, reopen_ms, query_us, recent_count, bytes
    );
}

fn run_sqlite_backend(count: usize, events: &[Event]) {
    let path = temp_path("sqlite", count, "sqlite3");
    cleanup_sqlite(&path);

    let mut store = SqliteEventStore::open(&path).expect("open sqlite store");
    let append_started = Instant::now();
    for event in events {
        store.append(event.clone()).expect("append sqlite event");
    }
    store.checkpoint().expect("checkpoint sqlite store");
    let append_ms = append_started.elapsed().as_millis();
    drop(store);

    let bytes = sqlite_bytes(&path);

    let reopen_started = Instant::now();
    let store = SqliteEventStore::open(&path).expect("reopen sqlite store");
    let reopen_ms = reopen_started.elapsed().as_millis();
    assert_eq!(store.len(), count);

    let target = SpaceId::from_bytes([TARGET_SPACE; 32]);
    let query_started = Instant::now();
    let recent = store
        .recent_events_for_space(&target, RECENT_LIMIT)
        .expect("query sqlite recent history");
    std::hint::black_box(&recent);
    let query_us = query_started.elapsed().as_micros();
    let recent_count = recent.len();
    drop(store);

    cleanup_sqlite(&path);

    println!(
        "storage-bench backend=sqlite count={} append_ms={} reopen_ms={} recent_query_us={} recent_count={} bytes={}",
        count, append_ms, reopen_ms, query_us, recent_count, bytes
    );
}

fn temp_path(backend: &str, count: usize, extension: &str) -> PathBuf {
    env::temp_dir().join(format!(
        "seed-storage-bench-{backend}-{count}-{}.{}",
        std::process::id(),
        extension
    ))
}

fn sqlite_bytes(path: &Path) -> u64 {
    let mut total = fs::metadata(path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    for suffix in ["-wal", "-shm"] {
        let mut sidecar = path.as_os_str().to_os_string();
        sidecar.push(suffix);
        total += fs::metadata(PathBuf::from(sidecar))
            .map(|metadata| metadata.len())
            .unwrap_or(0);
    }
    total
}

fn cleanup_sqlite(path: &Path) {
    let _ = fs::remove_file(path);
    for suffix in ["-wal", "-shm"] {
        let mut sidecar = path.as_os_str().to_os_string();
        sidecar.push(suffix);
        let _ = fs::remove_file(PathBuf::from(sidecar));
    }
}
