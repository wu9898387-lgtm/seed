use std::{collections::HashSet, path::Path, time::Duration};

use rusqlite::{params, Connection};

use crate::{
    event::Event,
    id::{EventId, SpaceId},
    storage::{AppendOutcome, EventStore, StorageError},
};

const SQLITE_SCHEMA_VERSION: i64 = 1;

const CREATE_SCHEMA: &str = r#"
CREATE TABLE events (
    ordinal     INTEGER PRIMARY KEY,
    event_id    BLOB NOT NULL UNIQUE CHECK(length(event_id) = 32),
    space_id    BLOB NOT NULL CHECK(length(space_id) = 32),
    event_bytes BLOB NOT NULL
);
CREATE INDEX events_space_ordinal ON events(space_id, ordinal);
PRAGMA user_version = 1;
"#;

/// Optional Phase-0 SQLite adapter used only for storage comparison.
///
/// This adapter intentionally stores the same canonical Event wire bytes as the
/// append-file backend. SQLite is therefore an implementation detail rather
/// than a second protocol format.
pub struct SqliteEventStore {
    connection: Connection,
    seen: HashSet<EventId>,
    events: Vec<Event>,
}

impl SqliteEventStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=FULL;
             PRAGMA foreign_keys=ON;",
        )?;

        let schema_version: i64 =
            connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;

        match schema_version {
            0 => connection.execute_batch(CREATE_SCHEMA)?,
            SQLITE_SCHEMA_VERSION => {}
            found => return Err(StorageError::UnsupportedSqliteSchemaVersion { found }),
        }

        let (seen, events) = Self::load_cache(&connection)?;

        Ok(Self {
            connection,
            seen,
            events,
        })
    }

    /// Query recent accepted Events directly through SQLite's space index.
    ///
    /// The returned Vec is chronological within this local accepted-history
    /// order. This method exists specifically so the Phase-0 comparison can
    /// measure indexed history reads without pretending the current
    /// reference-returning EventStore trait is a final database API.
    pub fn recent_events_for_space(
        &self,
        space: &SpaceId,
        limit: usize,
    ) -> Result<Vec<Event>, StorageError> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        let mut statement = self.connection.prepare(
            "SELECT ordinal, event_id, space_id, event_bytes
             FROM events
             WHERE space_id = ?1
             ORDER BY ordinal DESC
             LIMIT ?2",
        )?;

        let rows = statement.query_map(params![&space.as_bytes()[..], limit], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Vec<u8>>(1)?,
                row.get::<_, Vec<u8>>(2)?,
                row.get::<_, Vec<u8>>(3)?,
            ))
        })?;

        let mut events = Vec::new();
        for row in rows {
            let (ordinal, event_id, space_id, event_bytes) = row?;
            events.push(decode_row(ordinal, event_id, space_id, event_bytes)?);
        }
        events.reverse();
        Ok(events)
    }

    fn load_cache(connection: &Connection) -> Result<(HashSet<EventId>, Vec<Event>), StorageError> {
        let mut statement = connection.prepare(
            "SELECT ordinal, event_id, space_id, event_bytes
             FROM events
             ORDER BY ordinal",
        )?;

        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Vec<u8>>(1)?,
                row.get::<_, Vec<u8>>(2)?,
                row.get::<_, Vec<u8>>(3)?,
            ))
        })?;

        let mut seen = HashSet::new();
        let mut events = Vec::new();

        for row in rows {
            let (ordinal, event_id, space_id, event_bytes) = row?;
            let event = decode_row(ordinal, event_id, space_id, event_bytes)?;

            if !seen.insert(event.id()) {
                return Err(StorageError::SqliteEventCollision { id: event.id() });
            }
            events.push(event);
        }

        Ok((seen, events))
    }
}

impl EventStore for SqliteEventStore {
    fn append(&mut self, event: Event) -> Result<AppendOutcome, StorageError> {
        if self.seen.contains(&event.id()) {
            return Ok(AppendOutcome::Duplicate);
        }

        let event_bytes = event.canonical_bytes().map_err(StorageError::EventEncode)?;
        let event_id = event.id();
        let space_id = event.header().space;

        let inserted = self.connection.execute(
            "INSERT OR IGNORE INTO events(event_id, space_id, event_bytes)
             VALUES (?1, ?2, ?3)",
            params![
                &event_id.as_bytes()[..],
                &space_id.as_bytes()[..],
                &event_bytes
            ],
        )?;

        if inserted == 0 {
            let existing: Vec<u8> = self.connection.query_row(
                "SELECT event_bytes FROM events WHERE event_id = ?1",
                params![&event_id.as_bytes()[..]],
                |row| row.get(0),
            )?;

            if existing == event_bytes {
                return Ok(AppendOutcome::Duplicate);
            }

            return Err(StorageError::SqliteEventCollision { id: event_id });
        }

        self.seen.insert(event_id);
        self.events.push(event);
        Ok(AppendOutcome::Inserted)
    }

    fn len(&self) -> usize {
        self.events.len()
    }

    fn has_event(&self, id: &EventId) -> bool {
        self.seen.contains(id)
    }

    fn events_for_space(&self, space: &SpaceId) -> Vec<&Event> {
        self.events
            .iter()
            .filter(|event| &event.header().space == space)
            .collect()
    }

    fn checkpoint(&mut self) -> Result<(), StorageError> {
        self.connection
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        Ok(())
    }
}

fn decode_row(
    ordinal: i64,
    event_id: Vec<u8>,
    space_id: Vec<u8>,
    event_bytes: Vec<u8>,
) -> Result<Event, StorageError> {
    let event_id: [u8; 32] = event_id
        .try_into()
        .map_err(|_| StorageError::SqliteMalformedRow { ordinal })?;
    let space_id: [u8; 32] = space_id
        .try_into()
        .map_err(|_| StorageError::SqliteMalformedRow { ordinal })?;

    let stored_event_id = EventId::from_bytes(event_id);
    let stored_space_id = SpaceId::from_bytes(space_id);
    let event = Event::from_canonical_bytes(&event_bytes)
        .map_err(|error| StorageError::SqliteEventDecode { ordinal, error })?;

    if event.id() != stored_event_id {
        return Err(StorageError::SqliteEventIdMismatch { ordinal });
    }
    if event.header().space != stored_space_id {
        return Err(StorageError::SqliteSpaceIndexMismatch { ordinal });
    }

    Ok(event)
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use crate::{
        event::{Event, EventHeader},
        identity::{DeviceIdentity, RootIdentity},
        PROTOCOL_VERSION,
    };

    use super::*;

    fn event(sequence: u64, space: u8) -> Event {
        let root = RootIdentity::generate().unwrap();
        let device = DeviceIdentity::generate().unwrap();
        let authorization = root.authorize_device(&device, 1, 0);

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
                timestamp_ms: sequence as i64,
                schema: "test/v1".to_owned(),
            },
            vec![sequence as u8],
        )
        .unwrap()
    }

    fn test_path(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "seed-core-sqlite-{name}-{}.sqlite3",
            std::process::id()
        ));
        cleanup(&path);
        path
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

    #[test]
    fn sqlite_store_survives_reopen_and_suppresses_duplicates() {
        let path = test_path("reopen");
        let first = event(1, 9);
        let duplicate = first.clone();
        let first_id = first.id();

        {
            let mut store = SqliteEventStore::open(&path).unwrap();
            assert_eq!(store.append(first).unwrap(), AppendOutcome::Inserted);
            assert_eq!(store.append(duplicate).unwrap(), AppendOutcome::Duplicate);
            store.checkpoint().unwrap();
        }

        {
            let store = SqliteEventStore::open(&path).unwrap();
            assert_eq!(store.len(), 1);
            assert!(store.has_event(&first_id));
        }

        cleanup(&path);
    }

    #[test]
    fn sqlite_recent_history_uses_space_scope() {
        let path = test_path("recent");

        {
            let mut store = SqliteEventStore::open(&path).unwrap();
            store.append(event(1, 9)).unwrap();
            store.append(event(2, 8)).unwrap();
            store.append(event(3, 9)).unwrap();

            let recent = store
                .recent_events_for_space(&SpaceId::from_bytes([9; 32]), 1)
                .unwrap();
            assert_eq!(recent.len(), 1);
            assert_eq!(recent[0].header().sequence, 3);
        }

        cleanup(&path);
    }

    #[test]
    fn sqlite_corrupt_event_bytes_are_rejected_on_reopen() {
        let path = test_path("corrupt");

        {
            let mut store = SqliteEventStore::open(&path).unwrap();
            store.append(event(1, 9)).unwrap();
            store.checkpoint().unwrap();
        }

        {
            let connection = Connection::open(&path).unwrap();
            connection
                .execute(
                    "UPDATE events SET event_bytes = x'00' WHERE ordinal = 1",
                    [],
                )
                .unwrap();
        }

        assert!(matches!(
            SqliteEventStore::open(&path),
            Err(StorageError::SqliteEventDecode { .. })
        ));

        cleanup(&path);
    }

    #[test]
    fn unsupported_sqlite_schema_version_is_rejected() {
        let path = test_path("version");

        {
            let connection = Connection::open(&path).unwrap();
            connection
                .execute_batch("PRAGMA user_version = 99;")
                .unwrap();
        }

        assert!(matches!(
            SqliteEventStore::open(&path),
            Err(StorageError::UnsupportedSqliteSchemaVersion { found: 99 })
        ));

        cleanup(&path);
    }
}
