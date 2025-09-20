//! Types for dictionary import operations.
//!
//! # Why [`ImportTransaction`] instead of [`backend::ImportTransaction`]?
//!
//! When writing an importer for a dictionary format, the code must be generic
//! over the storage backend and codec used. The typical approach for this would
//! be to add `<T: backend::ImportTransaction, C: codec::Codec>` generic
//! parameters to the importer code. However, these generics are viral and would
//! make the logic more annoying to write; and the importer logic shouldn't even
//! care about the concrete transaction or codec types.
//!
//! To avoid this, we define our own `trait ImportTransaction` which is
//! effectively a dyn-compatible version of [`backend::ImportTransaction`], and
//! also has some extra logic like auto-incrementing record IDs and using a
//! codec for encoding internally instead of requiring the caller to encode the
//! record. This also means that importer code only has to monomorphize once on
//! `dyn ImportTransaction`, reducing codegen.

use {
    crate::{archive::OpenArchive, backend, codec},
    eyre::{Context, Result, eyre},
    std::sync::atomic::{self, AtomicU64},
    tracing::trace_span,
    wordbase_api::{DictionaryMeta, Record, RecordId, Term},
};

/// Importer which can read an [`OpenArchive`] of a specific format and insert
/// records into persistent storage.
pub trait StartImport {
    /// Begins importing an [`OpenArchive`].
    ///
    /// # Errors
    ///
    /// Errors if the archive is not valid for this importer, or some
    /// implementation-specific validation fails.
    fn start<'a>(
        &self,
        open_archive: &'a dyn OpenArchive,
    ) -> Result<(DictionaryMeta, Box<dyn FinishImport + 'a>)>;
}

impl<F> StartImport for F
where
    F: for<'a> Fn(&'a dyn OpenArchive) -> Result<(DictionaryMeta, Box<dyn FinishImport + 'a>)>,
{
    fn start<'a>(
        &self,
        open_archive: &'a dyn OpenArchive,
    ) -> Result<(DictionaryMeta, Box<dyn FinishImport + 'a>)> {
        (self)(open_archive)
    }
}

/// Continuation of [`StartImport::start`].
pub trait FinishImport {
    /// Continues the import process and finishes it.
    ///
    /// # Errors
    ///
    /// Errors if there was an invalid record, a record could not be inserted
    /// into storage, or some other implementation-specific error occurred.
    fn finish(
        self,
        txn: &dyn ImportTransaction,
        tx_progress: async_channel::Sender<ImportProgress>,
    ) -> Result<()>;
}

impl<F> FinishImport for F
where
    F: FnOnce(&dyn ImportTransaction, async_channel::Sender<ImportProgress>) -> Result<()>,
{
    fn finish(
        self,
        txn: &dyn ImportTransaction,
        tx_progress: async_channel::Sender<ImportProgress>,
    ) -> Result<()> {
        (self)(txn, tx_progress)
    }
}

/// Allows importing [`Record`]s into persistent storage.
///
/// Dyn-compatible version of [`backend::ImportTransaction`].
pub trait ImportTransaction: Send + Sync {
    /// Begins writing a batch of records into storage.
    ///
    /// [`ImportBatch`] may take a lock, so keep it live for the shortest
    /// possible time.
    ///
    /// # Errors
    ///
    /// Implementation-specific.
    fn batch(&self) -> Result<Box<dyn ImportBatch + '_>>;
}

/// Allows importing [`Record`]s into persistent storage.
///
/// Dyn-compatible version of [`backend::ImportBatch`], which automatically
/// assigns a unique ID to each record.
pub trait ImportBatch {
    /// Inserts a record into storage, and returns its newly-assigned ID.
    ///
    /// Prefer using [`ImportBatchExt::insert_record`].
    ///
    /// # Errors
    ///
    /// Implementation-specific.
    fn insert_record_ref(&mut self, record: &Record) -> Result<RecordId>;

    /// Inserts a term into storage, associating it with a previously-inserted
    /// record.
    ///
    /// # Errors
    ///
    /// Implementation-specific.
    fn insert_term(&mut self, term: &Term, record_id: RecordId) -> Result<()>;
}

/// Extension trait for [`ImportBatch`].
pub trait ImportBatchExt {
    /// Inserts a record into storage, and returns its newly-assigned ID.
    ///
    /// # Errors
    ///
    /// Implementation-specific.
    fn insert_record(&mut self, record: impl Into<Record>) -> Result<RecordId>;
}

impl<T: ?Sized + ImportBatch> ImportBatchExt for T {
    fn insert_record(&mut self, record: impl Into<Record>) -> Result<RecordId> {
        <Self as ImportBatch>::insert_record_ref(self, &record.into())
    }
}

/// Creates a new [`ImportTransaction`] based on an existing
/// [`backend::ImportTransaction`] and [`codec::Codec`].
pub fn import_transaction<'a>(
    txn: &'a impl backend::ImportTransaction,
    codec: &'a impl codec::Codec,
) -> impl ImportTransaction + 'a {
    struct Transaction<'a, T, C> {
        txn: &'a T,
        codec: &'a C,
        record_id: AtomicU64,
    }

    impl<T: backend::ImportTransaction, C: codec::Codec> ImportTransaction for Transaction<'_, T, C> {
        fn batch(&self) -> Result<Box<dyn ImportBatch + '_>> {
            Ok(Box::new(Batch {
                batch: self.txn.batch()?,
                encoder: self.codec.encoder(),
                record_id: &self.record_id,
            }))
        }
    }

    struct Batch<'txn, B, E> {
        batch: B,
        encoder: E,
        record_id: &'txn AtomicU64,
    }

    impl<B: backend::ImportBatch, E: codec::Encoder> ImportBatch for Batch<'_, B, E> {
        fn insert_record_ref(&mut self, record: &Record) -> Result<RecordId> {
            let record_id = RecordId(self.record_id.fetch_add(1, atomic::Ordering::SeqCst));
            let _span = trace_span!("insert_record", ?record_id).entered();

            let record_blob = self
                .encoder
                .encode(record)
                .wrap_err("failed to encode record")?;
            self.batch
                .insert_record(record_id, record_blob.as_ref())
                .wrap_err_with(|| eyre!("failed to insert {record_id:?}"))?;
            Ok(record_id)
        }

        fn insert_term(&mut self, term: &Term, record_id: RecordId) -> Result<()> {
            let _span = trace_span!("insert_term", ?term, ?record_id).entered();
            self.batch.insert_term(term, record_id)
        }
    }

    Transaction {
        txn,
        codec,
        record_id: AtomicU64::default(),
    }
}

/// Progress update on a dictionary import operation.
#[derive(Debug, Clone)]
pub struct ImportProgress {
    /// Estimate of how complete the import operation is.
    ///
    /// Expressed as a fraction between `0.0` and `1.0`.
    pub progress: f64,
}
