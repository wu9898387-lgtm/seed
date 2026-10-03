use std::{
    collections::HashSet,
    convert::Infallible,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};

use crate::{
    event::{EncodeError as EventEncodeError, Event, EventDecodeError},
    id::{EventId, SpaceId},
};

const LOG_MAGIC: &[u8; 4] = b"SELG";
const LOG_VERSION: u16 = 1;
const LOG_HEADER_BYTES: usize = 6;
const MAX_EVENT_RECORD_BYTES: usize = 17 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppendOutcome {
    Inserted,
    Duplicate,
}

pub trait EventStore {
    type Error;

    fn append(&mut self, event: Event) -> Result<AppendOutcome, Self::Error>;
    fn has_event(&self, id: &EventId) -> bool;
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
    type Error = Infallible;

    fn append(&mut self, event: Event) -> Result<AppendOutcome, Self::Error> {
        if !self.seen.insert(event.id()) {
            return Ok(AppendOutcome::Duplicate);
        }
        self.events.push(event);
        Ok(AppendOutcome::Inserted)
    }

    fn has_event(&self, id: &EventId) -> bool {
        self.seen.contains(id)
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
pub enum FileStoreError {
    Io(std::io::Error),
    InvalidHeader,
    UnsupportedVersion,
    RecordTooLarge,
    CorruptEvent {
        offset: u64,
        reason: EventDecodeError,
    },
    EventIdMismatch {
        offset: u64,
    },
    DuplicateRecord {
        offset: u64,
    },
    EventEncode(EventEncodeError),
}

impl From<std::io::Error> for FileStoreError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

/// Append-only Event Store spike.
///
/// This type only accepts Event objects that the caller has already decided are
/// valid. It does not replace the Identity/Capability/Governance acceptance
/// pipeline. On open, it validates structural event framing and the persisted
/// EventId, then rebuilds the in-memory index.
///
/// A partial final record is treated as an interrupted append and is truncated
/// back to the last complete record. A partial initial header that matches the
/// expected header prefix is repaired. Structural corruption in a complete
/// record is rejected rather than silently skipped.
#[derive(Debug)]
pub struct FileEventStore {
    file: File,
    seen: HashSet<EventId>,
    events: Vec<Event>,
}

impl FileEventStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, FileStoreError> {
        let path = path.as_ref();
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path)?;

        let file_len = file.metadata()?.len();
        let mut seen = HashSet::new();
        let mut events = Vec::new();

        if file_len == 0 {
            write_log_header(&mut file)?;
        } else if file_len < LOG_HEADER_BYTES as u64 {
            let expected = expected_log_header();
            let mut actual = [0u8; LOG_HEADER_BYTES];
            file.seek(SeekFrom::Start(0))?;
            file.read_exact(&mut actual[..file_len as usize])?;

            if actual[..file_len as usize] != expected[..file_len as usize] {
                return Err(FileStoreError::InvalidHeader);
            }

            file.set_len(0)?;
            write_log_header(&mut file)?;
        } else {
            file.seek(SeekFrom::Start(0))?;
            let mut header = [0u8; LOG_HEADER_BYTES];
            file.read_exact(&mut header)?;

            if &header[..4] != LOG_MAGIC {
                return Err(FileStoreError::InvalidHeader);
            }

            let version = u16::from_be_bytes([header[4], header[5]]);
            if version != LOG_VERSION {
                return Err(FileStoreError::UnsupportedVersion);
            }

            let mut offset = LOG_HEADER_BYTES as u64;
            while offset < file_len {
                let record_start = offset;
                let remaining = file_len - offset;

                if remaining < 4 {
                    recover_partial_tail(&mut file, record_start)?;
                    break;
                }

                let mut len_bytes = [0u8; 4];
                file.read_exact(&mut len_bytes)?;
                let event_len = u32::from_be_bytes(len_bytes) as usize;
                offset += 4;

                if event_len > MAX_EVENT_RECORD_BYTES {
                    return Err(FileStoreError::RecordTooLarge);
                }

                let needed = 32u64
                    .checked_add(event_len as u64)
                    .ok_or(FileStoreError::RecordTooLarge)?;

                if file_len - offset < needed {
                    recover_partial_tail(&mut file, record_start)?;
                    break;
                }

                let mut expected_id_bytes = [0u8; 32];
                file.read_exact(&mut expected_id_bytes)?;
                let expected_id = EventId::from_bytes(expected_id_bytes);
                offset += 32;

                let mut event_bytes = vec![0u8; event_len];
                file.read_exact(&mut event_bytes)?;
                offset += event_len as u64;

                let event = Event::from_canonical_bytes(&event_bytes).map_err(|reason| {
                    FileStoreError::CorruptEvent {
                        offset: record_start,
                        reason,
                    }
                })?;

                if event.id() != expected_id {
                    return Err(FileStoreError::EventIdMismatch {
                        offset: record_start,
                    });
                }

                if !seen.insert(event.id()) {
                    return Err(FileStoreError::DuplicateRecord {
                        offset: record_start,
                    });
                }
                events.push(event);
            }
        }

        file.seek(SeekFrom::End(0))?;

        Ok(Self { file, seen, events })
    }

    pub fn sync(&mut self) -> Result<(), FileStoreError> {
        self.file.flush()?;
        self.file.sync_data()?;
        Ok(())
    }
}

impl EventStore for FileEventStore {
    type Error = FileStoreError;

    fn append(&mut self, event: Event) -> Result<AppendOutcome, Self::Error> {
        if self.seen.contains(&event.id()) {
            return Ok(AppendOutcome::Duplicate);
        }

        let encoded = event
            .canonical_bytes()
            .map_err(FileStoreError::EventEncode)?;
        if encoded.len() > MAX_EVENT_RECORD_BYTES {
            return Err(FileStoreError::RecordTooLarge);
        }
        let encoded_len =
            u32::try_from(encoded.len()).map_err(|_| FileStoreError::RecordTooLarge)?;

        let start = self.file.seek(SeekFrom::End(0))?;
        let write_result = (|| -> Result<(), std::io::Error> {
            self.file.write_all(&encoded_len.to_be_bytes())?;
            self.file.write_all(event.id().as_bytes())?;
            self.file.write_all(&encoded)?;
            self.file.flush()?;
            self.file.sync_data()
        })();

        if let Err(error) = write_result {
            let _ = self.file.set_len(start);
            let _ = self.file.seek(SeekFrom::End(0));
            return Err(FileStoreError::Io(error));
        }

        self.seen.insert(event.id());
        self.events.push(event);
        Ok(AppendOutcome::Inserted)
    }

    fn has_event(&self, id: &EventId) -> bool {
        self.seen.contains(id)
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

fn expected_log_header() -> [u8; LOG_HEADER_BYTES] {
    let version = LOG_VERSION.to_be_bytes();
    [
        LOG_MAGIC[0],
        LOG_MAGIC[1],
        LOG_MAGIC[2],
        LOG_MAGIC[3],
        version[0],
        version[1],
    ]
}

fn write_log_header(file: &mut File) -> Result<(), FileStoreError> {
    file.seek(SeekFrom::Start(0))?;
    file.write_all(&expected_log_header())?;
    file.flush()?;
    file.sync_data()?;
    Ok(())
}

fn recover_partial_tail(file: &mut File, valid_len: u64) -> Result<(), FileStoreError> {
    file.set_len(valid_len)?;
    file.seek(SeekFrom::End(0))?;
    file.sync_data()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        fs::OpenOptions,
        io::Write,
        path::{Path, PathBuf},
        process,
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

    fn temp_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!("seed-{label}-{}.eventlog", process::id()))
    }

    fn cleanup(path: &Path) {
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn duplicate_event_is_idempotent() {
        let event = event(1, 9);
        let event_id = event.id();
        let duplicate = event.clone();
        let mut store = InMemoryEventStore::default();

        assert_eq!(store.append(event).unwrap(), AppendOutcome::Inserted);
        assert!(store.has_event(&event_id));
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
    fn file_store_round_trip_and_duplicate_suppression() {
        let path = temp_path("round-trip");
        cleanup(&path);

        let first = event(1, 9);
        let first_id = first.id();
        let duplicate = first.clone();

        {
            let mut store = FileEventStore::open(&path).unwrap();
            assert_eq!(store.append(first).unwrap(), AppendOutcome::Inserted);
            assert!(store.has_event(&first_id));
            assert_eq!(store.len(), 1);
        }

        {
            let mut reopened = FileEventStore::open(&path).unwrap();
            assert_eq!(reopened.len(), 1);
            assert!(reopened.has_event(&first_id));
            assert_eq!(
                reopened.append(duplicate).unwrap(),
                AppendOutcome::Duplicate
            );
        }

        cleanup(&path);
    }

    #[test]
    fn reopen_rebuilds_index_without_loading_whole_file() {
        let path = temp_path("streaming-reopen");
        cleanup(&path);

        {
            let mut store = FileEventStore::open(&path).unwrap();
            for sequence in 0..128 {
                store.append(event(sequence, (sequence % 4) as u8)).unwrap();
            }
            assert_eq!(store.len(), 128);
        }

        let reopened = FileEventStore::open(&path).unwrap();
        assert_eq!(reopened.len(), 128);
        assert_eq!(
            reopened
                .events_for_space(&SpaceId::from_bytes([2; 32]))
                .len(),
            32
        );

        cleanup(&path);
    }

    #[test]
    fn partial_header_is_repaired_on_reopen() {
        let path = temp_path("partial-header");
        cleanup(&path);

        std::fs::write(&path, &expected_log_header()[..3]).unwrap();

        let reopened = FileEventStore::open(&path).unwrap();
        assert!(reopened.is_empty());
        assert_eq!(
            std::fs::read(&path).unwrap().as_slice(),
            expected_log_header().as_slice()
        );

        cleanup(&path);
    }

    #[test]
    fn partial_tail_is_truncated_on_reopen() {
        let path = temp_path("partial-tail");
        cleanup(&path);

        {
            let mut store = FileEventStore::open(&path).unwrap();
            store.append(event(1, 9)).unwrap();
        }
        let good_len = std::fs::metadata(&path).unwrap().len();

        {
            let mut file = OpenOptions::new().append(true).open(&path).unwrap();
            file.write_all(&100u32.to_be_bytes()).unwrap();
            file.write_all(&[7u8; 5]).unwrap();
            file.sync_data().unwrap();
        }
        assert!(std::fs::metadata(&path).unwrap().len() > good_len);

        let reopened = FileEventStore::open(&path).unwrap();
        assert_eq!(reopened.len(), 1);
        assert_eq!(std::fs::metadata(&path).unwrap().len(), good_len);

        cleanup(&path);
    }

    #[test]
    fn corrupted_persisted_event_id_is_rejected() {
        let path = temp_path("corrupt-id");
        cleanup(&path);

        {
            let mut store = FileEventStore::open(&path).unwrap();
            store.append(event(1, 9)).unwrap();
        }

        let mut bytes = std::fs::read(&path).unwrap();
        let event_id_offset = LOG_HEADER_BYTES + 4;
        bytes[event_id_offset] ^= 0xff;
        std::fs::write(&path, bytes).unwrap();

        assert!(matches!(
            FileEventStore::open(&path),
            Err(FileStoreError::EventIdMismatch { .. })
        ));

        cleanup(&path);
    }
}
