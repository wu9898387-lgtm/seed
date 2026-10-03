use std::{
    collections::HashSet,
    fs::{File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    path::Path,
};

use crate::{
    crypto::hash32,
    event::{EncodeError as EventEncodeError, Event, EventDecodeError},
    id::{EventId, SpaceId},
};

const LOG_MAGIC: &[u8; 4] = b"SLOG";
const LOG_VERSION: u16 = 1;
const RECORD_CHECKSUM_DOMAIN: &[u8] = b"seed:event-log-record:v1\0";
const MAX_STORED_EVENT_BYTES: usize = 17 * 1024 * 1024 + 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppendOutcome {
    Inserted,
    Duplicate,
}

pub trait EventStore {
    fn append(&mut self, event: Event) -> AppendOutcome;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn events_for_space(&self, space: &SpaceId) -> Vec<&Event>;
}

#[derive(Debug, Default)]
pub struct InMemoryEventStore {
    seen: HashSet<EventId>,
    events: Vec<Event>,
}

impl EventStore for InMemoryEventStore {
    fn append(&mut self, event: Event) -> AppendOutcome {
        if !self.seen.insert(event.id()) {
            return AppendOutcome::Duplicate;
        }
        self.events.push(event);
        AppendOutcome::Inserted
    }

    fn len(&self) -> usize {
        self.events.len()
    }

    fn events_for_space(&self, space: &SpaceId) -> Vec<&Event> {
        self.events
            .iter()
            .filter(|event| &event.header().space == space)
            .collect()
    }
}

#[derive(Debug)]
pub enum StorageError {
    Io(io::Error),
    InvalidMagic,
    UnsupportedVersion,
    TruncatedRecord,
    RecordTooLarge,
    ChecksumMismatch,
    DuplicateRecord,
    EventEncode(EventEncodeError),
    EventDecode(EventDecodeError),
}

impl From<io::Error> for StorageError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Append-only local Event log.
///
/// The file stores only canonical Event records. Cryptographic authorization is
/// re-verified by callers after replay because the store intentionally does not
/// own Identity/Device authorization state.
#[derive(Debug)]
pub struct FileEventStore {
    file: File,
    seen: HashSet<EventId>,
    events: Vec<Event>,
}

impl FileEventStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let path = path.as_ref();
        let mut file = OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(path)?;

        if file.metadata()?.len() == 0 {
            file.write_all(LOG_MAGIC)?;
            file.write_all(&LOG_VERSION.to_be_bytes())?;
            file.sync_data()?;
        }

        file.seek(SeekFrom::Start(0))?;

        let mut magic = [0u8; 4];
        file.read_exact(&mut magic)?;
        if &magic != LOG_MAGIC {
            return Err(StorageError::InvalidMagic);
        }

        let mut version = [0u8; 2];
        file.read_exact(&mut version)?;
        if u16::from_be_bytes(version) != LOG_VERSION {
            return Err(StorageError::UnsupportedVersion);
        }

        let mut store = Self {
            file,
            seen: HashSet::new(),
            events: Vec::new(),
        };
        store.replay()?;
        Ok(store)
    }

    pub fn append(&mut self, event: Event) -> Result<AppendOutcome, StorageError> {
        if self.seen.contains(&event.id()) {
            return Ok(AppendOutcome::Duplicate);
        }

        let encoded = event
            .canonical_bytes()
            .map_err(StorageError::EventEncode)?;
        if encoded.len() > MAX_STORED_EVENT_BYTES {
            return Err(StorageError::RecordTooLarge);
        }

        let len = u32::try_from(encoded.len()).map_err(|_| StorageError::RecordTooLarge)?;
        let checksum = hash32(RECORD_CHECKSUM_DOMAIN, &[&encoded]);

        self.file.write_all(&len.to_be_bytes())?;
        self.file.write_all(&encoded)?;
        self.file.write_all(&checksum)?;
        self.file.flush()?;
        self.file.sync_data()?;

        self.seen.insert(event.id());
        self.events.push(event);
        Ok(AppendOutcome::Inserted)
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn events_for_space(&self, space: &SpaceId) -> Vec<&Event> {
        self.events
            .iter()
            .filter(|event| &event.header().space == space)
            .collect()
    }

    fn replay(&mut self) -> Result<(), StorageError> {
        loop {
            let Some(len) = read_record_len(&mut self.file)? else {
                break;
            };

            let len = len as usize;
            if len > MAX_STORED_EVENT_BYTES {
                return Err(StorageError::RecordTooLarge);
            }

            let mut encoded = vec![0u8; len];
            read_exact_record(&mut self.file, &mut encoded)?;

            let mut expected_checksum = [0u8; 32];
            read_exact_record(&mut self.file, &mut expected_checksum)?;

            let actual_checksum = hash32(RECORD_CHECKSUM_DOMAIN, &[&encoded]);
            if actual_checksum != expected_checksum {
                return Err(StorageError::ChecksumMismatch);
            }

            let event =
                Event::from_canonical_bytes(&encoded).map_err(StorageError::EventDecode)?;
            if !self.seen.insert(event.id()) {
                return Err(StorageError::DuplicateRecord);
            }
            self.events.push(event);
        }

        self.file.seek(SeekFrom::End(0))?;
        Ok(())
    }
}

fn read_record_len(file: &mut File) -> Result<Option<u32>, StorageError> {
    let mut len = [0u8; 4];
    let read = file.read(&mut len)?;
    if read == 0 {
        return Ok(None);
    }
    if read < len.len() {
        file.read_exact(&mut len[read..])
            .map_err(|error| match error.kind() {
                io::ErrorKind::UnexpectedEof => StorageError::TruncatedRecord,
                _ => StorageError::Io(error),
            })?;
    }
    Ok(Some(u32::from_be_bytes(len)))
}

fn read_exact_record(file: &mut File, buffer: &mut [u8]) -> Result<(), StorageError> {
    file.read_exact(buffer).map_err(|error| match error.kind() {
        io::ErrorKind::UnexpectedEof => StorageError::TruncatedRecord,
        _ => StorageError::Io(error),
    })
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        io::{Seek, SeekFrom, Write},
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    use crate::{
        event::{Event, EventHeader},
        id::SpaceId,
        identity::{DeviceIdentity, RootIdentity},
        PROTOCOL_VERSION,
    };

    use super::*;

    static NEXT_PATH: AtomicU64 = AtomicU64::new(1);

    struct TempLog(PathBuf);

    impl TempLog {
        fn new() -> Self {
            let id = NEXT_PATH.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "seed-core-event-store-{}-{id}.log",
                std::process::id()
            ));
            let _ = fs::remove_file(&path);
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempLog {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    fn event(sequence: u64, space: u8) -> (RootIdentity, DeviceIdentity, crate::identity::DeviceAuthorization, Event) {
        let root = RootIdentity::generate().unwrap();
        let device = DeviceIdentity::generate().unwrap();
        let authorization = root.authorize_device(&device, 1, 0);
        let header = EventHeader {
            protocol_version: PROTOCOL_VERSION,
            space: SpaceId::from_bytes([space; 32]),
            author: root.document().id(),
            device: device.id(),
            sequence,
            timestamp_ms: 0,
            schema: "test/v1".to_owned(),
        };

        let event = Event::sign(
            root.document(),
            &authorization,
            &device,
            header,
            vec![sequence as u8],
        )
        .unwrap();
        (root, device, authorization, event)
    }

    #[test]
    fn duplicate_event_is_idempotent_in_memory() {
        let (_, _, _, event) = event(1, 9);
        let duplicate = event.clone();
        let mut store = InMemoryEventStore::default();

        assert_eq!(store.append(event), AppendOutcome::Inserted);
        assert_eq!(store.append(duplicate), AppendOutcome::Duplicate);
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn memory_query_is_scoped_to_space() {
        let (_, _, _, first) = event(1, 9);
        let (_, _, _, second) = event(2, 8);
        let mut store = InMemoryEventStore::default();
        store.append(first);
        store.append(second);

        assert_eq!(
            store.events_for_space(&SpaceId::from_bytes([9; 32])).len(),
            1
        );
    }

    #[test]
    fn file_store_survives_reopen_and_reverification() {
        let log = TempLog::new();
        let (root, _device, authorization, event) = event(1, 9);
        let event_id = event.id();

        {
            let mut store = FileEventStore::open(log.path()).unwrap();
            assert_eq!(
                store.append(event).unwrap(),
                AppendOutcome::Inserted
            );
            assert_eq!(store.len(), 1);
        }

        let reopened = FileEventStore::open(log.path()).unwrap();
        assert_eq!(reopened.len(), 1);
        let restored = reopened.events_for_space(&SpaceId::from_bytes([9; 32]));
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].id(), event_id);
        restored[0]
            .verify(root.document(), &authorization)
            .unwrap();
    }

    #[test]
    fn duplicate_file_append_does_not_grow_log() {
        let log = TempLog::new();
        let (_, _, _, event) = event(1, 9);
        let duplicate = event.clone();

        let mut store = FileEventStore::open(log.path()).unwrap();
        assert_eq!(store.append(event).unwrap(), AppendOutcome::Inserted);
        let first_len = fs::metadata(log.path()).unwrap().len();

        assert_eq!(
            store.append(duplicate).unwrap(),
            AppendOutcome::Duplicate
        );
        let second_len = fs::metadata(log.path()).unwrap().len();
        assert_eq!(first_len, second_len);
    }

    #[test]
    fn truncated_tail_is_rejected() {
        let log = TempLog::new();
        let (_, _, _, event) = event(1, 9);

        {
            let mut store = FileEventStore::open(log.path()).unwrap();
            store.append(event).unwrap();
        }

        let mut file = OpenOptions::new().append(true).open(log.path()).unwrap();
        file.write_all(&[0, 0]).unwrap();
        file.sync_data().unwrap();

        assert!(matches!(
            FileEventStore::open(log.path()),
            Err(StorageError::TruncatedRecord)
        ));
    }

    #[test]
    fn record_checksum_detects_corruption() {
        let log = TempLog::new();
        let (_, _, _, event) = event(1, 9);

        {
            let mut store = FileEventStore::open(log.path()).unwrap();
            store.append(event).unwrap();
        }

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(log.path())
            .unwrap();

        file.seek(SeekFrom::Start(10)).unwrap();
        let mut byte = [0u8; 1];
        file.read_exact(&mut byte).unwrap();
        byte[0] ^= 0x01;
        file.seek(SeekFrom::Start(10)).unwrap();
        file.write_all(&byte).unwrap();
        file.sync_data().unwrap();

        assert!(matches!(
            FileEventStore::open(log.path()),
            Err(StorageError::ChecksumMismatch)
        ));
    }
}
