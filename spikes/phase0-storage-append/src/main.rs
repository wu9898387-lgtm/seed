use seed_core::codec::{encode_event_for_signing, SignableEvent};
use seed_core::event::EventKind;
use seed_core::identity::{DeviceId, IdentityId};
use seed_core::space::SpaceId;
use seed_core::storage::RecordStore;
use std::fs::{File, OpenOptions};
use std::io::{self, ErrorKind, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

struct AppendLog {
    file: File,
}

impl AppendLog {
    fn open(path: &Path) -> io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(path)?;
        Ok(Self { file })
    }

    fn sync(&self) -> io::Result<()> {
        self.file.sync_data()
    }
}

impl RecordStore for AppendLog {
    type Error = io::Error;

    fn append(&mut self, record: &[u8]) -> Result<(), Self::Error> {
        let len = u32::try_from(record.len())
            .map_err(|_| io::Error::new(ErrorKind::InvalidInput, "record exceeds u32 length"))?;
        self.file.write_all(&len.to_be_bytes())?;
        self.file.write_all(record)?;
        Ok(())
    }

    fn scan(&mut self, visitor: &mut dyn FnMut(&[u8])) -> Result<(), Self::Error> {
        self.file.seek(SeekFrom::Start(0))?;

        loop {
            let mut len_bytes = [0u8; 4];
            match self.file.read_exact(&mut len_bytes) {
                Ok(()) => {}
                Err(error) if error.kind() == ErrorKind::UnexpectedEof => return Ok(()),
                Err(error) => return Err(error),
            }

            let len = u32::from_be_bytes(len_bytes) as usize;
            let mut record = vec![0u8; len];
            self.file.read_exact(&mut record)?;
            visitor(&record);
        }
    }
}

fn temp_path() -> PathBuf {
    std::env::temp_dir().join(format!("seed-phase0-append-{}.log", std::process::id()))
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

fn exercise_storage() -> io::Result<()> {
    let path = temp_path();
    let _ = std::fs::remove_file(&path);
    let event = canonical_event();

    {
        let mut store = AppendLog::open(&path)?;
        store.append(&event)?;
        store.append(b"second-record")?;
        store.sync()?;
    }

    let mut records = 0usize;
    let mut first_matches = false;
    {
        let mut reopened = AppendLog::open(&path)?;
        reopened.scan(&mut |record| {
            if records == 0 {
                first_matches = record == event;
            }
            records += 1;
        })?;
    }

    std::fs::remove_file(&path)?;
    assert_eq!(records, 2);
    assert!(first_matches);
    Ok(())
}

fn main() -> io::Result<()> {
    exercise_storage()?;
    println!("Seed Phase 0 append-only storage spike");
    println!("reopen_replay=ok");
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn append_sync_reopen_and_replay() {
        super::exercise_storage().unwrap();
    }
}
