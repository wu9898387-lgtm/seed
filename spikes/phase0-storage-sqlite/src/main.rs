use rusqlite::{params, Connection};
use seed_core::codec::{encode_event_for_signing, SignableEvent};
use seed_core::event::EventKind;
use seed_core::identity::{DeviceId, IdentityId};
use seed_core::space::SpaceId;
use std::path::PathBuf;

fn temp_path() -> PathBuf {
    std::env::temp_dir().join(format!("seed-phase0-sqlite-{}.db", std::process::id()))
}

fn canonical_event() -> [u8; 119] {
    let event = SignableEvent {
        space: SpaceId::from_bytes([0x22; 32]),
        author: IdentityId::from_bytes([0x33; 32]),
        device: DeviceId::from_bytes([0x44; 32]),
        kind: EventKind::MESSAGE,
        payload: b"hello",
    };

    let mut output = [0u8; 119];
    let written = encode_event_for_signing(&event, &mut output).expect("fixed Event must encode");
    assert_eq!(written, output.len());
    output
}

fn exercise_sqlite() -> rusqlite::Result<()> {
    let path = temp_path();
    let _ = std::fs::remove_file(&path);
    let event = canonical_event();

    {
        let connection = Connection::open(&path)?;
        connection.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS events (
                 sequence INTEGER PRIMARY KEY AUTOINCREMENT,
                 record   BLOB NOT NULL
             );",
        )?;
        connection.execute("INSERT INTO events(record) VALUES (?1)", params![event.as_slice()])?;
        connection.execute("INSERT INTO events(record) VALUES (?1)", params![b"second-record"])?;
    }

    {
        let connection = Connection::open(&path)?;
        let count: i64 =
            connection.query_row("SELECT COUNT(*) FROM events", (), |row| row.get(0))?;
        let first: Vec<u8> = connection.query_row(
            "SELECT record FROM events ORDER BY sequence LIMIT 1",
            (),
            |row| row.get(0),
        )?;

        assert_eq!(count, 2);
        assert_eq!(first, event);
    }

    let _ = std::fs::remove_file(format!("{}-wal", path.display()));
    let _ = std::fs::remove_file(format!("{}-shm", path.display()));
    let _ = std::fs::remove_file(&path);
    Ok(())
}

fn main() -> rusqlite::Result<()> {
    exercise_sqlite()?;
    println!("Seed Phase 0 system SQLite storage spike");
    println!("reopen_query=ok");
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn sqlite_insert_reopen_and_query() {
        super::exercise_sqlite().unwrap();
    }
}
