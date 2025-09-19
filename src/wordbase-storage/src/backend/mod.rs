use {
    crate::codec::Decoder,
    eyre::Result,
    std::path::Path,
    wordbase_api::{Record, RecordId, Term},
};

#[cfg(feature = "backend-heed")]
pub mod heed;
#[cfg(feature = "backend-heed")]
pub type Heed = heed::Backend;

#[cfg(feature = "backend-libsql")]
pub mod libsql;
#[cfg(feature = "backend-libsql")]
pub type Libsql = libsql::Backend;

#[cfg(feature = "backend-redb")]
pub mod redb;
#[cfg(feature = "backend-redb")]
pub type Redb = redb::Backend;

#[cfg(feature = "backend-rocksdb")]
pub mod rocksdb;
#[cfg(feature = "backend-rocksdb")]
pub type RocksDb = rocksdb::Backend;

// #[cfg(feature = "backend-rusqlite")]
// pub mod rusqlite;
// #[cfg(feature = "backend-rusqlite")]
// pub type Rusqlite = rusqlite::Storage;

// #[cfg(feature = "backend-turso")]
// pub mod turso;
// #[cfg(feature = "backend-turso")]
// pub type Turso = turso::Storage;

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

pub trait Lookups {
    fn lookup_lemma<D: Decoder>(
        &self,
        make_decoder: impl Fn() -> D,
        lemma: &str,
    ) -> Result<Vec<(TermPart, Record)>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermPart {
    Headword,
    Reading,
}
