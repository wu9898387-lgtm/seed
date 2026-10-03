/// Minimal persistent record store boundary for Phase 0.
///
/// Core owns the record semantics; adapters own files, SQLite, key-value
/// stores, durability policy, encryption-at-rest integration, and indexing.
pub trait RecordStore {
    type Error;

    /// Append one complete opaque protocol record.
    fn append(&mut self, record: &[u8]) -> Result<(), Self::Error>;

    /// Visit stored records in durable order.
    ///
    /// The borrowed slice is only valid for the duration of the callback.
    fn scan(&mut self, visitor: &mut dyn FnMut(&[u8])) -> Result<(), Self::Error>;
}
