//! See [`Storage`].
#![expect(
    clippy::missing_errors_doc,
    reason = "all errors are implementation-specific"
)]

use {
    crate::codec::Decoder,
    eyre::Result,
    std::path::Path,
    wordbase_types::{Record, RecordId, Term},
};

/// Allows inserting [`Record`]s into a persistent storage on disk, and opening
/// the storage for reading.
pub trait Storage: Send + Sync + 'static {
    /// Type of [`Storage::open`].
    type Lookups: LookupStorage;

    /// Creates storage for importing a dictionary.
    fn create_import_storage(
        data_dir: &Path,
    ) -> impl Future<Output = Result<impl ImportStorage + 'static>>;

    /// Opens an existing storage for reading records.
    fn open(data_dir: &Path) -> impl Future<Output = Result<Self::Lookups>>;
}

/// Persistent [`Record`] storage which is open for writing.
pub trait ImportStorage: Send + Sync {
    /// Starts a transaction.
    fn transaction(&mut self) -> Result<impl ImportTransaction>;

    /// Commits this storage to disk.
    fn commit(self) -> Result<()>;
}

/// Write transaction for an [`ImportStorage`].
pub trait ImportTransaction: Send + Sync {
    /// Type of [`ImportTransaction::batch`].
    type Batch<'txn>: ImportBatch
    where
        Self: 'txn;

    /// Starts writing a batch of records.
    ///
    /// [`ImportBatch`] may take a lock, so keep it live for the shortest
    /// possible time.
    fn batch(&self) -> Result<Self::Batch<'_>>;

    /// Commits this transaction to disk.
    fn commit(self) -> Result<()>;
}

/// Write batch for an [`ImportStorage`].
pub trait ImportBatch {
    /// Inserts [encoded] record data into storage.
    ///
    /// [encoded]: crate::codec::Encoder
    fn insert_record(&mut self, record_id: RecordId, record: &[u8]) -> Result<()>;

    /// Inserts a term linked to a record into storage.
    fn insert_term(&mut self, term: &Term, record_id: RecordId) -> Result<()>;
}

/// Persistent [`Record`] storage which is open for reading.
pub trait LookupStorage: Sized + Send {
    /// Finds record IDs which are linked to the given lemma text (either as a
    /// headword or reading), and finds the corresponding record.
    fn lookup<D: Decoder>(
        &self,
        make_decoder: impl Fn() -> D,
        lemma: &str,
    ) -> Result<Vec<RecordRow>>;
}

/// [`Record`] and its metadata returned by a [`LookupStorage::lookup`].
#[derive(Debug)]
pub struct RecordRow {
    /// Term which points to this record.
    pub term: Term,
    /// Record ID.
    pub record_id: RecordId,
    /// Record data.
    pub record: Record,
}
