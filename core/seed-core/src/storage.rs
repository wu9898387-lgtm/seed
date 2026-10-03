use std::{
    collections::HashSet,
    fs::{File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    path::Path,
};

use crate::{
    crypto::hash32,
    event::{EncodeError, Event, EventDecodeError, MAX_EVENT_WIRE_BYTES},
    id::{EventId, SpaceId},
};

const STORE_MAGIC: &[u8; 8] = b"SEEDLOG1";
const FRAME_CHECKSUM_DOMAIN: &[u8] = b"seed:event-store-frame:v1\0";
const FRAME_CHECKSUM_BYTES: usize = 32;
const FRAME_PREFIX_BYTES: usize = 4 + FRAME_CHECKSUM_BYTES;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppendOutcome {
    Inserted,
    Duplicate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryOutcome {
    Clean,
    TruncatedTail { offset: u64, removed_bytes: u64 },
}

#[derive(Debug)]
pub enum StorageError {
    Io(io::Error),
    EventEncode(EncodeError),
    EventDecode {
        offset: u64,
        error: EventDecodeError,
    },
    InvalidHeader,
    FrameTooLarge {
        offset: u64,
        length: u32,
    },
    CorruptFrame {
        offset: u64,
    },
    TruncatedTail {
        offset: u64,
        file_len: u64,
    },
    DuplicateEventInLog {
        offset: u64,
        id: EventId,
    },
}

impl From<io::Error> for StorageError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Accepted-event storage.
///
/// Callers are responsible for completing identity/signature/governance/state
/// validation before appending. The store provides durability, deduplication,
/// corruption detection, and space-scoped history access; it is not an
/// authorization bypass.
pub trait EventStore {
    fn append(&mut self, event: Event) -> Result<AppendOutcome, StorageError>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn has_event(&self, id: &EventId) -> bool;
    fn events_for_space(&self, space: &SpaceId) -> Vec<&Event>;
    fn checkpoint(&mut self) -> Result<(), StorageError> {
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct InMemoryEventStore {
    seen: HashSet<EventId>,
    events: Vec<Event>,
}

impl EventStore for InMemoryEventStore {
    fn append(&mut self, event: Event) -> Result<AppendOutcome, StorageError> {
        if !self.seen.insert(event.id()) {
            return Ok(AppendOutcome::Duplicate);
        }
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
}

/// Minimal append-only persistent Event Store used by the Phase-0 storage spike.
///
/// File layout:
///
/// ```text
/// "SEEDLOG1"
/// repeat {
///   event_len: u32 big-endian
///   checksum:  32-byte SHA-256 domain-separated digest of event_bytes
///   event_bytes: canonical Event wire record
/// }
/// ```
///
/// The checksum detects accidental/local file corruption; Event authenticity is
/// still provided by the Device signature and must be validated by the normal
/// acceptance/rebuild pipeline.
#[derive(Debug)]
pub struct FileEventStore {
    file: File,
    seen: HashSet<EventId>,
    events: Vec<Event>,
}

impl FileEventStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        Self::open_inner(path, false).map(|(store, _)| store)
    }

    /// Open the log and repair only an incomplete final frame.
    ///
    /// A checksum mismatch, invalid Event encoding, duplicate persisted Event,
    /// invalid header, or oversized frame remains a hard error and is never
    /// silently truncated.
    pub fn open_recover_tail(
        path: impl AsRef<Path>,
    ) -> Result<(Self, RecoveryOutcome), StorageError> {
        Self::open_inner(path, true)
    }

    fn open_inner(
        path: impl AsRef<Path>,
        recover_tail: bool,
    ) -> Result<(Self, RecoveryOutcome), StorageError> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;

        if file.metadata()?.len() == 0 {
            file.write_all(STORE_MAGIC)?;
            file.sync_data()?;
        }

        file.seek(SeekFrom::Start(0))?;
        let mut header = [0u8; STORE_MAGIC.len()];
        match file.read_exact(&mut header) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => {
                return Err(StorageError::InvalidHeader);
            }
            Err(error) => return Err(StorageError::Io(error)),
        }
        if &header != STORE_MAGIC {
            return Err(StorageError::InvalidHeader);
        }

        let mut store = Self {
            file,
            seen: HashSet::new(),
            events: Vec::new(),
        };
        let recovery = store.rebuild_index(recover_tail)?;
        store.file.seek(SeekFrom::End(0))?;
        Ok((store, recovery))
    }

    fn rebuild_index(&mut self, recover_tail: bool) -> Result<RecoveryOutcome, StorageError> {
        let file_len = self.file.metadata()?.len();
        let mut offset = STORE_MAGIC.len() as u64;

        while offset < file_len {
            let remaining = file_len - offset;
            if remaining < 4 {
                return self.handle_truncated_tail(recover_tail, offset, file_len);
            }

            self.file.seek(SeekFrom::Start(offset))?;
            let mut len_bytes = [0u8; 4];
            self.file.read_exact(&mut len_bytes)?;
            let frame_len = u32::from_be_bytes(len_bytes);

            if frame_len as usize > MAX_EVENT_WIRE_BYTES {
                return Err(StorageError::FrameTooLarge {
                    offset,
                    length: frame_len,
                });
            }

            let total_frame_len = FRAME_PREFIX_BYTES as u64 + frame_len as u64;
            if remaining < total_frame_len {
                return self.handle_truncated_tail(recover_tail, offset, file_len);
            }

            let mut stored_checksum = [0u8; FRAME_CHECKSUM_BYTES];
            self.file.read_exact(&mut stored_checksum)?;

            let mut event_bytes = vec![0u8; frame_len as usize];
            self.file.read_exact(&mut event_bytes)?;

            let expected_checksum = hash32(FRAME_CHECKSUM_DOMAIN, &[&event_bytes]);
            if stored_checksum != expected_checksum {
                return Err(StorageError::CorruptFrame { offset });
            }

            let event =
                Event::from_canonical_bytes(&event_bytes).map_err(|error| StorageError::EventDecode {
                    offset,
                    error,
                })?;

            if !self.seen.insert(event.id()) {
                return Err(StorageError::DuplicateEventInLog {
                    offset,
                    id: event.id(),
                });
            }
            self.events.push(event);
            offset += total_frame_len;
        }

        Ok(RecoveryOutcome::Clean)
    }

    fn handle_truncated_tail(
        &mut self,
        recover_tail: bool,
        offset: u64,
        file_len: u64,
    ) -> Result<RecoveryOutcome, StorageError> {
        if !recover_tail {
            return Err(StorageError::TruncatedTail { offset, file_len });
        }

        self.file.set_len(offset)?;
        self.file.sync_data()?;
        self.file.seek(SeekFrom::Start(offset))?;

        Ok(RecoveryOutcome::TruncatedTail {
            offset,
            removed_bytes: file_len - offset,
        })
    }
}

impl EventStore for FileEventStore {
    fn append(&mut self, event: Event) -> Result<AppendOutcome, StorageError> {
        if self.seen.contains(&event.id()) {
            return Ok(AppendOutcome::Duplicate);
        }

        let event_bytes = event
            .canonical_bytes()
            .map_err(StorageError::EventEncode)?;
        if event_bytes.len() > MAX_EVENT_WIRE_BYTES {
            return Err(StorageError::EventEncode(EncodeError::BodyTooLarge));
        }

        let frame_len = u32::try_from(event_bytes.len())
            .map_err(|_| StorageError::EventEncode(EncodeError::FieldTooLarge))?;
        let checksum = hash32(FRAME_CHECKSUM_DOMAIN, &[&event_bytes]);
        let start = self.file.seek(SeekFrom::End(0))?;

        let write_result = (|| -> io::Result<()> {
            self.file.write_all(&frame_len.to_be_bytes())?;
            self.file.write_all(&checksum)?;
            self.file.write_all(&event_bytes)?;
            self.file.sync_data()?;
            Ok(())
        })();

        if let Err(error) = write_result {
            let _ = self.file.set_len(start);
            let _ = self.file.seek(SeekFrom::End(0));
            return Err(StorageError::Io(error));
        }

        self.seen.insert(event.id());
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
        self.file.sync_data()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        io::{Read, Seek, Write},
        path::PathBuf,
    };

    use crate::{
        event::{Event, EventHeader},
        id::SpaceId,
        identity::{DeviceIdentity, RootIdentity},
        PROTOCOL_VERSION,
    };

    use super::*;

    fn event(sequence: u64, space: u8) -> Event {
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

        Event::sign(
            root.document(),
            &authorization,
            &device,
            header,
            vec![sequence as u8],
        )
        .unwrap()
    }

    fn test_path(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("seed-core-{name}-{}.eventlog", std::process::id()));
        let _ = fs::remove_file(&path);
        path
    }

    #[test]
    fn duplicate_event_is_idempotent() {
        let event = event(1, 9);
        let duplicate = event.clone();
        let mut store = InMemoryEventStore::default();

        assert_eq!(store.append(event).unwrap(), AppendOutcome::Inserted);
        assert_eq!(store.append(duplicate).unwrap(), AppendOutcome::Duplicate);
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn query_is_scoped_to_space() {
        let mut store = InMemoryEventStore::default();
        store.append(event(1, 9)).unwrap();
        store.append(event(2, 8)).unwrap();

        assert_eq!(
            store.events_for_space(&SpaceId::from_bytes([9; 32])).len(),
            1
        );
    }

    #[test]
    fn persistent_store_survives_reopen_and_rebuilds_index() {
        let path = test_path("reopen");
        let first = event(1, 9);
        let second = event(2, 8);
        let first_id = first.id();

        {
            let mut store = FileEventStore::open(&path).unwrap();
            assert_eq!(store.append(first).unwrap(), AppendOutcome::Inserted);
            assert_eq!(store.append(second).unwrap(), AppendOutcome::Inserted);
            assert_eq!(store.len(), 2);
            store.checkpoint().unwrap();
        }

        {
            let store = FileEventStore::open(&path).unwrap();
            assert_eq!(store.len(), 2);
            assert!(store.has_event(&first_id));
            assert_eq!(
                store.events_for_space(&SpaceId::from_bytes([9; 32])).len(),
                1
            );
        }

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn persistent_duplicate_is_not_rewritten() {
        let path = test_path("duplicate");
        let event = event(1, 9);
        let duplicate = event.clone();
        let mut store = FileEventStore::open(&path).unwrap();

        assert_eq!(store.append(event).unwrap(), AppendOutcome::Inserted);
        let after_insert = fs::metadata(&path).unwrap().len();
        assert_eq!(store.append(duplicate).unwrap(), AppendOutcome::Duplicate);
        let after_duplicate = fs::metadata(&path).unwrap().len();

        assert_eq!(after_insert, after_duplicate);
        drop(store);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn truncated_tail_is_rejected_or_explicitly_recovered() {
        let path = test_path("truncated-tail");

        {
            let mut store = FileEventStore::open(&path).unwrap();
            store.append(event(1, 9)).unwrap();
        }

        {
            let mut file = OpenOptions::new().append(true).open(&path).unwrap();
            file.write_all(&[0x00, 0x01]).unwrap();
            file.sync_data().unwrap();
        }

        assert!(matches!(
            FileEventStore::open(&path),
            Err(StorageError::TruncatedTail { .. })
        ));

        let (store, recovery) = FileEventStore::open_recover_tail(&path).unwrap();
        assert_eq!(store.len(), 1);
        assert!(matches!(
            recovery,
            RecoveryOutcome::TruncatedTail {
                removed_bytes: 2,
                ..
            }
        ));

        drop(store);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn checksum_corruption_is_never_silently_recovered() {
        let path = test_path("corrupt-checksum");

        {
            let mut store = FileEventStore::open(&path).unwrap();
            store.append(event(1, 9)).unwrap();
        }

        {
            let mut file = OpenOptions::new().read(true).write(true).open(&path).unwrap();
            let checksum_offset = STORE_MAGIC.len() as u64 + 4;
            file.seek(SeekFrom::Start(checksum_offset)).unwrap();
            let mut byte = [0u8; 1];
            file.read_exact(&mut byte).unwrap();
            byte[0] ^= 0xff;
            file.seek(SeekFrom::Start(checksum_offset)).unwrap();
            file.write_all(&byte).unwrap();
            file.sync_data().unwrap();
        }

        assert!(matches!(
            FileEventStore::open_recover_tail(&path),
            Err(StorageError::CorruptFrame { .. })
        ));

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn oversized_frame_length_is_rejected_before_allocation() {
        let path = test_path("oversized-frame");

        {
            let mut file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&path)
                .unwrap();
            file.write_all(STORE_MAGIC).unwrap();
            let oversized = (MAX_EVENT_WIRE_BYTES as u32) + 1;
            file.write_all(&oversized.to_be_bytes()).unwrap();
            file.sync_data().unwrap();
        }

        assert!(matches!(
            FileEventStore::open(&path),
            Err(StorageError::FrameTooLarge { .. })
        ));

        fs::remove_file(path).unwrap();
    }
}
