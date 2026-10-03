use std::{
    collections::HashSet,
    fmt,
    fs::{File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    path::Path,
};

use crate::{
    crypto::{Signature, SIGNATURE_LENGTH},
    event::{Event, EventHeader},
    id::{DeviceId, EventId, IdentityId, SpaceId},
};

const FILE_MAGIC: &[u8; 8] = b"SEEDLOG1";
const RECORD_VERSION: u8 = 1;
const MAX_EVENT_RECORD_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppendOutcome {
    Inserted,
    Duplicate,
}

#[derive(Debug)]
pub enum StorageError {
    Io(io::Error),
    InvalidHeader,
    EventTooLarge,
    RecordTooLarge { offset: u64, len: u32 },
    CorruptRecord { offset: u64 },
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "storage I/O error: {error}"),
            Self::InvalidHeader => f.write_str("invalid Seed event-log header"),
            Self::EventTooLarge => {
                f.write_str("event is too large for the event-log record format")
            }
            Self::RecordTooLarge { offset, len } => {
                write!(
                    f,
                    "event-log record at offset {offset} is too large: {len} bytes"
                )
            }
            Self::CorruptRecord { offset } => {
                write!(f, "event-log record at offset {offset} is corrupt")
            }
        }
    }
}

impl std::error::Error for StorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::InvalidHeader
            | Self::EventTooLarge
            | Self::RecordTooLarge { .. }
            | Self::CorruptRecord { .. } => None,
        }
    }
}

impl From<io::Error> for StorageError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

pub trait EventStore {
    fn append(&mut self, event: Event) -> Result<AppendOutcome, StorageError>;
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

    fn events_for_space(&self, space: &SpaceId) -> Vec<&Event> {
        self.events
            .iter()
            .filter(|event| &event.header().space == space)
            .collect()
    }
}

/// Minimal append-only persistent Event Store spike.
///
/// The on-disk framing is an implementation detail, not the Seed network wire format.
/// Complete records are content-ID checked while loading. A torn final record is
/// truncated back to the last complete record, while a complete corrupt record fails
/// closed instead of being silently discarded.
#[derive(Debug)]
pub struct FileEventStore {
    file: File,
    seen: HashSet<EventId>,
    events: Vec<Event>,
}

impl FileEventStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path)?;

        if file.metadata()?.len() == 0 {
            file.write_all(FILE_MAGIC)?;
            file.sync_data()?;
        } else {
            let mut magic = [0u8; FILE_MAGIC.len()];
            match read_exact_state(&mut file, &mut magic)? {
                ReadState::Complete if &magic == FILE_MAGIC => {}
                ReadState::Complete | ReadState::Eof | ReadState::Partial => {
                    return Err(StorageError::InvalidHeader);
                }
            }
        }

        let mut seen = HashSet::new();
        let mut events = Vec::new();

        loop {
            let record_offset = file.stream_position()?;
            let mut length_bytes = [0u8; 4];

            match read_exact_state(&mut file, &mut length_bytes)? {
                ReadState::Eof => break,
                ReadState::Partial => {
                    truncate_torn_tail(&mut file, record_offset)?;
                    break;
                }
                ReadState::Complete => {}
            }

            let record_len = u32::from_be_bytes(length_bytes);
            let record_len_usize =
                usize::try_from(record_len).map_err(|_| StorageError::RecordTooLarge {
                    offset: record_offset,
                    len: record_len,
                })?;

            if record_len_usize == 0 || record_len_usize > MAX_EVENT_RECORD_BYTES {
                return Err(StorageError::RecordTooLarge {
                    offset: record_offset,
                    len: record_len,
                });
            }

            let mut record = vec![0u8; record_len_usize];
            match read_exact_state(&mut file, &mut record)? {
                ReadState::Complete => {}
                ReadState::Eof | ReadState::Partial => {
                    truncate_torn_tail(&mut file, record_offset)?;
                    break;
                }
            }

            let event = decode_event_record(&record).map_err(|_| StorageError::CorruptRecord {
                offset: record_offset,
            })?;

            if seen.insert(event.id()) {
                events.push(event);
            }
        }

        file.seek(SeekFrom::End(0))?;

        Ok(Self { file, seen, events })
    }

    pub fn sync(&mut self) -> Result<(), StorageError> {
        self.file.sync_data()?;
        Ok(())
    }
}

impl EventStore for FileEventStore {
    fn append(&mut self, event: Event) -> Result<AppendOutcome, StorageError> {
        if self.seen.contains(&event.id()) {
            return Ok(AppendOutcome::Duplicate);
        }

        let record = encode_event_record(&event)?;
        let record_len = u32::try_from(record.len()).map_err(|_| StorageError::EventTooLarge)?;
        let record_offset = self.file.seek(SeekFrom::End(0))?;

        let write_result = (|| -> io::Result<()> {
            self.file.write_all(&record_len.to_be_bytes())?;
            self.file.write_all(&record)?;
            self.file.sync_data()
        })();

        if let Err(error) = write_result {
            let _ = self.file.set_len(record_offset);
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

    fn events_for_space(&self, space: &SpaceId) -> Vec<&Event> {
        self.events
            .iter()
            .filter(|event| &event.header().space == space)
            .collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReadState {
    Complete,
    Eof,
    Partial,
}

fn read_exact_state(reader: &mut File, buffer: &mut [u8]) -> io::Result<ReadState> {
    let mut read = 0usize;

    while read < buffer.len() {
        match reader.read(&mut buffer[read..])? {
            0 if read == 0 => return Ok(ReadState::Eof),
            0 => return Ok(ReadState::Partial),
            count => read += count,
        }
    }

    Ok(ReadState::Complete)
}

fn truncate_torn_tail(file: &mut File, valid_len: u64) -> Result<(), StorageError> {
    file.set_len(valid_len)?;
    file.sync_data()?;
    file.seek(SeekFrom::End(0))?;
    Ok(())
}

fn encode_event_record(event: &Event) -> Result<Vec<u8>, StorageError> {
    let schema = event.header().schema.as_bytes();
    let payload = event.payload();

    let schema_len = u32::try_from(schema.len()).map_err(|_| StorageError::EventTooLarge)?;
    let payload_len = u32::try_from(payload.len()).map_err(|_| StorageError::EventTooLarge)?;

    let estimated = 1usize
        .saturating_add(32)
        .saturating_add(2)
        .saturating_add(32)
        .saturating_add(32)
        .saturating_add(32)
        .saturating_add(8)
        .saturating_add(8)
        .saturating_add(4)
        .saturating_add(schema.len())
        .saturating_add(4)
        .saturating_add(payload.len())
        .saturating_add(SIGNATURE_LENGTH);

    if estimated > MAX_EVENT_RECORD_BYTES {
        return Err(StorageError::EventTooLarge);
    }

    let mut record = Vec::with_capacity(estimated);
    record.push(RECORD_VERSION);
    record.extend_from_slice(event.id().as_bytes());
    record.extend_from_slice(&event.header().protocol_version.to_be_bytes());
    record.extend_from_slice(event.header().space.as_bytes());
    record.extend_from_slice(event.header().author.as_bytes());
    record.extend_from_slice(event.header().device.as_bytes());
    record.extend_from_slice(&event.header().sequence.to_be_bytes());
    record.extend_from_slice(&event.header().timestamp_ms.to_be_bytes());
    record.extend_from_slice(&schema_len.to_be_bytes());
    record.extend_from_slice(schema);
    record.extend_from_slice(&payload_len.to_be_bytes());
    record.extend_from_slice(payload);
    record.extend_from_slice(event.signature().as_bytes());

    Ok(record)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RecordDecodeError {
    UnexpectedEof,
    TrailingBytes,
    UnsupportedVersion,
    InvalidUtf8,
    InvalidEventId,
}

fn decode_event_record(bytes: &[u8]) -> Result<Event, RecordDecodeError> {
    let mut cursor = RecordCursor::new(bytes);

    if cursor.u8()? != RECORD_VERSION {
        return Err(RecordDecodeError::UnsupportedVersion);
    }

    let id = EventId::from_bytes(cursor.fixed()?);
    let protocol_version = cursor.u16()?;
    let space = SpaceId::from_bytes(cursor.fixed()?);
    let author = IdentityId::from_bytes(cursor.fixed()?);
    let device = DeviceId::from_bytes(cursor.fixed()?);
    let sequence = cursor.u64()?;
    let timestamp_ms = cursor.i64()?;

    let schema_bytes = cursor.bytes()?;
    let schema = std::str::from_utf8(schema_bytes)
        .map_err(|_| RecordDecodeError::InvalidUtf8)?
        .to_owned();
    let payload = cursor.bytes()?.to_vec();
    let signature = Signature::from_bytes(cursor.fixed()?);

    cursor.finish()?;

    let header = EventHeader {
        protocol_version,
        space,
        author,
        device,
        sequence,
        timestamp_ms,
        schema,
    };

    Event::from_stored_parts(id, header, payload, signature)
        .map_err(|_| RecordDecodeError::InvalidEventId)
}

struct RecordCursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> RecordCursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], RecordDecodeError> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or(RecordDecodeError::UnexpectedEof)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(RecordDecodeError::UnexpectedEof)?;
        self.offset = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, RecordDecodeError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, RecordDecodeError> {
        Ok(u16::from_be_bytes(self.fixed()?))
    }

    fn u32(&mut self) -> Result<u32, RecordDecodeError> {
        Ok(u32::from_be_bytes(self.fixed()?))
    }

    fn u64(&mut self) -> Result<u64, RecordDecodeError> {
        Ok(u64::from_be_bytes(self.fixed()?))
    }

    fn i64(&mut self) -> Result<i64, RecordDecodeError> {
        Ok(i64::from_be_bytes(self.fixed()?))
    }

    fn fixed<const N: usize>(&mut self) -> Result<[u8; N], RecordDecodeError> {
        let mut value = [0u8; N];
        value.copy_from_slice(self.take(N)?);
        Ok(value)
    }

    fn bytes(&mut self) -> Result<&'a [u8], RecordDecodeError> {
        let len = self.u32()? as usize;
        self.take(len)
    }

    fn finish(self) -> Result<(), RecordDecodeError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(RecordDecodeError::TrailingBytes)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    use crate::{
        event::{Event, EventHeader},
        id::SpaceId,
        identity::{DeviceIdentity, RootIdentity},
        PROTOCOL_VERSION,
    };

    use super::*;

    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

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

    fn temp_path(label: &str) -> std::path::PathBuf {
        let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "seed-event-store-{}-{label}-{counter}.log",
            std::process::id()
        ))
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
    fn file_store_survives_reopen() {
        let path = temp_path("reopen");
        let original = event(1, 9);
        let original_id = original.id();

        {
            let mut store = FileEventStore::open(&path).unwrap();
            assert_eq!(store.append(original).unwrap(), AppendOutcome::Inserted);
            assert_eq!(store.len(), 1);
        }

        {
            let store = FileEventStore::open(&path).unwrap();
            assert_eq!(store.len(), 1);
            assert_eq!(
                store.events_for_space(&SpaceId::from_bytes([9; 32])).len(),
                1
            );
            assert_eq!(
                store.events_for_space(&SpaceId::from_bytes([9; 32]))[0].id(),
                original_id
            );
        }

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn file_store_duplicate_append_does_not_grow_log() {
        let path = temp_path("duplicate");
        let original = event(1, 9);
        let duplicate = original.clone();

        let mut store = FileEventStore::open(&path).unwrap();
        assert_eq!(store.append(original).unwrap(), AppendOutcome::Inserted);
        let first_len = fs::metadata(&path).unwrap().len();

        assert_eq!(store.append(duplicate).unwrap(), AppendOutcome::Duplicate);
        assert_eq!(fs::metadata(&path).unwrap().len(), first_len);

        drop(store);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn file_store_truncates_torn_tail() {
        let path = temp_path("torn-tail");

        {
            let mut store = FileEventStore::open(&path).unwrap();
            store.append(event(1, 9)).unwrap();
        }

        let clean_len = fs::metadata(&path).unwrap().len();

        {
            let mut file = OpenOptions::new().append(true).open(&path).unwrap();
            file.write_all(&[0, 0]).unwrap();
            file.sync_data().unwrap();
        }

        assert!(fs::metadata(&path).unwrap().len() > clean_len);

        {
            let store = FileEventStore::open(&path).unwrap();
            assert_eq!(store.len(), 1);
        }

        assert_eq!(fs::metadata(&path).unwrap().len(), clean_len);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn file_store_rejects_complete_corrupt_record() {
        let path = temp_path("corrupt");

        {
            let mut store = FileEventStore::open(&path).unwrap();
            store.append(event(1, 9)).unwrap();
        }

        {
            let mut file = OpenOptions::new().append(true).open(&path).unwrap();
            file.write_all(&1u32.to_be_bytes()).unwrap();
            file.write_all(&[0xff]).unwrap();
            file.sync_data().unwrap();
        }

        let error = FileEventStore::open(&path).unwrap_err();
        assert!(matches!(error, StorageError::CorruptRecord { .. }));

        fs::remove_file(path).unwrap();
    }
}
