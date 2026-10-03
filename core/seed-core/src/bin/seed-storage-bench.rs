use std::{
    env, fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use seed_core::{
    event::{Event, EventHeader},
    id::SpaceId,
    identity::{DeviceIdentity, RootIdentity},
    sqlite_storage::SqliteEventStore,
    storage::{AppendOutcome, EventStore, FileEventStore},
    PROTOCOL_VERSION,
};

const DEFAULT_EVENTS: usize = 1_000;
const DEFAULT_SPACES: usize = 16;
const DEFAULT_RECENT: usize = 64;
const MAX_EVENTS: usize = 1_000_000;
const MAX_SPACES: usize = 255;

#[derive(Clone, Copy, Debug)]
struct Config {
    events: usize,
    spaces: usize,
    recent: usize,
}

#[derive(Debug)]
struct Metrics {
    backend: &'static str,
    append: Duration,
    reopen: Duration,
    recent_query: Duration,
    storage_bytes: u64,
    recent_returned: usize,
}

fn main() {
    let config = parse_config();
    let (events, generation) = build_events(config);

    println!(
        "seed-storage-bench events={} spaces={} recent={} event_generation_ms={:.3}",
        config.events,
        config.spaces,
        config.recent,
        millis(generation)
    );

    let file = benchmark_file(&events, config);
    let sqlite = benchmark_sqlite(&events, config);

    print_metrics(&file, config.events);
    print_metrics(&sqlite, config.events);
}

fn parse_config() -> Config {
    let mut config = Config {
        events: DEFAULT_EVENTS,
        spaces: DEFAULT_SPACES,
        recent: DEFAULT_RECENT,
    };

    let mut args = env::args().skip(1);
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .unwrap_or_else(|| panic!("missing value for {flag}"));
        match flag.as_str() {
            "--events" => config.events = parse_positive(&flag, &value),
            "--spaces" => config.spaces = parse_positive(&flag, &value),
            "--recent" => config.recent = parse_positive(&flag, &value),
            _ => panic!("unknown argument: {flag}"),
        }
    }

    assert!(
        config.events <= MAX_EVENTS,
        "--events must be <= {MAX_EVENTS}"
    );
    assert!(
        config.spaces <= MAX_SPACES,
        "--spaces must be <= {MAX_SPACES}"
    );

    config
}

fn parse_positive(flag: &str, value: &str) -> usize {
    let parsed = value
        .parse::<usize>()
        .unwrap_or_else(|_| panic!("{flag} must be a positive integer"));
    assert!(parsed > 0, "{flag} must be greater than zero");
    parsed
}

fn build_events(config: Config) -> (Vec<Event>, Duration) {
    let root = RootIdentity::generate().expect("root identity");
    let device = DeviceIdentity::generate().expect("device identity");
    let authorization = root.authorize_device(&device, 1, 0);

    let started = Instant::now();
    let mut events = Vec::with_capacity(config.events);

    for index in 0..config.events {
        let sequence = u64::try_from(index + 1).expect("bounded event count");
        let space_byte =
            u8::try_from((index % config.spaces) + 1).expect("spaces bounded to one byte");
        let mut payload = vec![0u8; 64];
        payload[..8].copy_from_slice(&sequence.to_be_bytes());
        payload[8] = space_byte;

        events.push(
            Event::sign(
                root.document(),
                &authorization,
                &device,
                EventHeader {
                    protocol_version: PROTOCOL_VERSION,
                    space: SpaceId::from_bytes([space_byte; 32]),
                    author: root.document().id(),
                    device: device.id(),
                    sequence,
                    timestamp_ms: i64::try_from(sequence).expect("bounded timestamp"),
                    schema: "seed.storage.bench/v1".to_owned(),
                },
                payload,
            )
            .expect("signed benchmark event"),
        );
    }

    (events, started.elapsed())
}

fn benchmark_file(events: &[Event], config: Config) -> Metrics {
    let path = temp_path("append-file", "eventlog");
    let _ = fs::remove_file(&path);

    let mut store = FileEventStore::open(&path).expect("open append-file store");
    let started = Instant::now();
    for event in events {
        assert_eq!(
            store.append(event.clone()).expect("append file event"),
            AppendOutcome::Inserted
        );
    }
    store.checkpoint().expect("checkpoint append-file store");
    let append = started.elapsed();
    drop(store);

    let storage_bytes = fs::metadata(&path).expect("append-file metadata").len();

    let started = Instant::now();
    let store = FileEventStore::open(&path).expect("reopen append-file store");
    let reopen = started.elapsed();
    assert_eq!(store.len(), config.events);

    let target = SpaceId::from_bytes([1; 32]);
    let expected_recent = target_event_count(config).min(config.recent);
    let started = Instant::now();
    let matching = store.events_for_space(&target);
    let recent_start = matching.len().saturating_sub(config.recent);
    let recent_returned = matching[recent_start..].len();
    let recent_query = started.elapsed();
    assert_eq!(matching.len(), target_event_count(config));
    assert_eq!(recent_returned, expected_recent);

    drop(store);
    fs::remove_file(&path).expect("remove append-file benchmark");

    Metrics {
        backend: "append-file",
        append,
        reopen,
        recent_query,
        storage_bytes,
        recent_returned,
    }
}

fn benchmark_sqlite(events: &[Event], config: Config) -> Metrics {
    let path = temp_path("sqlite", "sqlite3");
    cleanup_sqlite(&path);

    let mut store = SqliteEventStore::open(&path).expect("open sqlite store");
    let started = Instant::now();
    for event in events {
        assert_eq!(
            store.append(event.clone()).expect("append sqlite event"),
            AppendOutcome::Inserted
        );
    }
    store.checkpoint().expect("checkpoint sqlite store");
    let append = started.elapsed();
    drop(store);

    let storage_bytes = sqlite_persisted_bytes(&path);

    let started = Instant::now();
    let store = SqliteEventStore::open(&path).expect("reopen sqlite store");
    let reopen = started.elapsed();
    assert_eq!(store.len(), config.events);

    let target = SpaceId::from_bytes([1; 32]);
    let expected_recent = target_event_count(config).min(config.recent);
    let started = Instant::now();
    let recent = store
        .recent_events_for_space(&target, config.recent)
        .expect("indexed sqlite recent-history query");
    let recent_query = started.elapsed();
    assert_eq!(recent.len(), expected_recent);

    drop(store);
    cleanup_sqlite(&path);

    Metrics {
        backend: "sqlite",
        append,
        reopen,
        recent_query,
        storage_bytes,
        recent_returned: recent.len(),
    }
}

fn target_event_count(config: Config) -> usize {
    ((config.events - 1) / config.spaces) + 1
}

fn print_metrics(metrics: &Metrics, event_count: usize) {
    let append_seconds = metrics.append.as_secs_f64();
    let events_per_second = if append_seconds > 0.0 {
        event_count as f64 / append_seconds
    } else {
        f64::INFINITY
    };

    println!(
        "backend={} append_ms={:.3} append_events_per_sec={:.1} reopen_ms={:.3} recent_query_ms={:.3} storage_bytes={} recent_returned={}",
        metrics.backend,
        millis(metrics.append),
        events_per_second,
        millis(metrics.reopen),
        millis(metrics.recent_query),
        metrics.storage_bytes,
        metrics.recent_returned
    );
}

fn millis(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn temp_path(label: &str, extension: &str) -> PathBuf {
    env::temp_dir().join(format!(
        "seed-storage-bench-{label}-{}.{}",
        std::process::id(),
        extension
    ))
}

fn sqlite_persisted_bytes(path: &Path) -> u64 {
    let mut bytes = fs::metadata(path).expect("sqlite db metadata").len();
    let wal = sidecar_path(path, "-wal");
    if let Ok(metadata) = fs::metadata(wal) {
        bytes += metadata.len();
    }
    bytes
}

fn cleanup_sqlite(path: &Path) {
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(sidecar_path(path, "-wal"));
    let _ = fs::remove_file(sidecar_path(path, "-shm"));
}

fn sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut sidecar = path.as_os_str().to_os_string();
    sidecar.push(suffix);
    PathBuf::from(sidecar)
}
