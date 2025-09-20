use {
    crate::archive::OpenArchive,
    eyre::Result,
    wordbase_api::{DictionaryMeta, Record, RecordId, Term},
};

pub trait StartImport: Send + Sync + 'static {
    fn start(open_archive: impl OpenArchive) -> Result<(DictionaryMeta, impl FinishImport)>;
}

pub trait FinishImport: Send {
    fn finish(
        self,
        txn: &impl ImportTransaction,
        tx_progress: async_channel::Sender<ImportProgress>,
    ) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct ImportProgress {
    pub progress: f64,
}

pub trait ImportTransaction: Send + Sync {
    type Batch<'txn>: ImportBatch
    where
        Self: 'txn;

    fn batch(&self) -> Result<Self::Batch<'_>>;
}

pub trait ImportBatch {
    fn insert_record(&mut self, record: impl Into<Record>) -> Result<RecordId>;

    fn insert_term(&mut self, term: &Term, record_id: RecordId) -> Result<()>;
}

pub mod imp {
    use {
        crate::{
            backend,
            codec::{Codec, Encoder},
        },
        eyre::{Context as _, Result, eyre},
        std::sync::atomic::{self, AtomicU64},
        tracing::trace_span,
        wordbase_api::{Record, RecordId, Term},
    };

    #[derive(Debug)]
    pub struct ImportTransaction<'a, T, C> {
        txn: &'a T,
        codec: &'a C,
        record_id: AtomicU64,
    }

    impl<'a, T: backend::ImportTransaction, C: Codec> ImportTransaction<'a, T, C> {
        pub fn new(txn: &'a T, codec: &'a C) -> Self {
            Self {
                txn,
                codec,
                record_id: AtomicU64::default(),
            }
        }
    }

    impl<T: backend::ImportTransaction, C: Codec> super::ImportTransaction
        for ImportTransaction<'_, T, C>
    {
        type Batch<'txn>
            = ImportBatch<'txn, T::Batch<'txn>, C::Encoder>
        where
            Self: 'txn;

        fn batch(&self) -> Result<ImportBatch<'_, T::Batch<'_>, C::Encoder>> {
            Ok(ImportBatch {
                batch: self.txn.batch()?,
                encoder: self.codec.encoder(),
                record_id: &self.record_id,
            })
        }
    }

    #[derive(Debug)]
    pub struct ImportBatch<'txn, B, E> {
        batch: B,
        encoder: E,
        record_id: &'txn AtomicU64,
    }

    impl<B: backend::ImportBatch, E: Encoder> super::ImportBatch for ImportBatch<'_, B, E> {
        fn insert_record(&mut self, record: impl Into<Record>) -> Result<RecordId> {
            self.insert_record_(&record.into())
        }

        fn insert_term(&mut self, term: &Term, record_id: RecordId) -> Result<()> {
            let _span = trace_span!("insert_term", ?term, ?record_id).entered();
            self.batch.insert_term(term, record_id)
        }
    }

    impl<B: backend::ImportBatch, E: Encoder> ImportBatch<'_, B, E> {
        fn insert_record_(&mut self, record: &Record) -> Result<RecordId> {
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
    }
}
