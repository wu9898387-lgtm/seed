#![cfg(feature = "sqlite-storage")]

use std::{
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use seed_core::{
    sqlite_storage::SqliteEventStore,
    storage::{EventStore, FileEventStore},
};

const EVENT_COUNT: usize = 32;

#[test]
fn completed_appends_survive_process_kill_without_clean_shutdown() {
    run_case("append-file");
    run_case("sqlite");
}

fn run_case(backend: &str) {
    let path = test_path(backend);
    cleanup(&path);

    let mut child = Command::new(env!("CARGO_BIN_EXE_seed-storage-crash-worker"))
        .arg(backend)
        .arg(&path)
        .arg(EVENT_COUNT.to_string())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn crash worker");

    let stdout = child.stdout.take().expect("worker stdout");
    let mut reader = BufReader::new(stdout);
    let mut ready = String::new();
    let bytes = reader.read_line(&mut ready).expect("read ready marker");
    assert!(bytes > 0, "worker exited before ready marker");
    assert_eq!(
        ready.trim(),
        format!("ready backend={backend} count={EVENT_COUNT}")
    );

    child.kill().expect("kill worker");
    let _ = child.wait().expect("reap worker");

    match backend {
        "append-file" => {
            let store = FileEventStore::open(&path).expect("reopen append-file after kill");
            assert_eq!(store.len(), EVENT_COUNT);
        }
        "sqlite" => {
            let store = SqliteEventStore::open(&path).expect("reopen sqlite after kill");
            assert_eq!(store.len(), EVENT_COUNT);
        }
        other => panic!("unsupported backend: {other}"),
    }

    cleanup(&path);
}

fn test_path(backend: &str) -> PathBuf {
    let extension = if backend == "sqlite" {
        "sqlite3"
    } else {
        "eventlog"
    };

    std::env::temp_dir().join(format!(
        "seed-storage-process-kill-{backend}-{}.{}",
        std::process::id(),
        extension
    ))
}

fn cleanup(path: &Path) {
    let _ = fs::remove_file(path);

    for suffix in ["-wal", "-shm"] {
        let mut sidecar = path.as_os_str().to_os_string();
        sidecar.push(suffix);
        let _ = fs::remove_file(PathBuf::from(sidecar));
    }
}
