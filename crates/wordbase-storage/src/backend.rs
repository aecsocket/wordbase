use {
    crate::codec::Decoder,
    eyre::Result,
    std::path::Path,
    wordbase_api::{Record, RecordId, Term, TermPart},
};

pub trait Backend: Send + Sync + 'static {
    fn import(data_dir: &Path) -> Result<impl ImportStorage + use<Self>>;

    fn open(data_dir: &Path) -> Result<impl Lookups + use<Self>>;
}

pub trait ImportStorage: Send + Sync {
    fn transaction(&mut self) -> Result<impl ImportTransaction>;

    fn commit(self) -> Result<()>;
}

pub trait ImportTransaction: Send + Sync {
    type Batch<'txn>: ImportBatch
    where
        Self: 'txn;

    fn batch(&self) -> Result<Self::Batch<'_>>;

    fn commit(self) -> Result<()>;
}

pub trait ImportBatch {
    fn insert_record(&mut self, record_id: RecordId, record: &[u8]) -> Result<()>;

    fn insert_term(&mut self, term: &Term, record_id: RecordId) -> Result<()>;
}

pub trait Lookups: Send {
    fn lookup_lemma<D: Decoder>(
        &self,
        make_decoder: impl Fn() -> D,
        lemma: &str,
    ) -> Result<Vec<RecordRow>>;
}

#[derive(Debug)]
pub struct RecordRow {
    pub term_part: TermPart,
    pub record_id: RecordId,
    pub record: Record,
}
