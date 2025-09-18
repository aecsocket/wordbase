use {
    crate::codec::Codec,
    eyre::Result,
    std::path::Path,
    wordbase_api::{Record, RecordId, Term},
};

#[cfg(feature = "storage-heed")]
pub mod heed;
#[cfg(feature = "storage-heed")]
pub type Heed<C> = heed::Storage<C>;

// #[cfg(feature = "storage-libsql")]
// pub mod libsql;
// #[cfg(feature = "storage-libsql")]
// pub type LibSql<C> = libsql::Storage<C>;

#[cfg(feature = "storage-redb")]
pub mod redb;
#[cfg(feature = "storage-redb")]
pub type Redb<C> = redb::Storage<C>;

#[cfg(feature = "storage-rocksdb")]
pub mod rocksdb;
#[cfg(feature = "storage-rocksdb")]
pub type RocksDb<C> = rocksdb::Storage<C>;

// #[cfg(feature = "storage-rusqlite")]
// pub mod rusqlite;
// #[cfg(feature = "storage-rusqlite")]
// pub type Rusqlite<C> = rusqlite::Storage<C>;

// #[cfg(feature = "storage-turso")]
// pub mod turso;
// #[cfg(feature = "storage-turso")]
// pub type Turso<C> = turso::Storage<C>;

pub trait Storage: Send + Sync + Clone + 'static {
    type Codec: Codec;

    fn with_codec(codec: Self::Codec) -> Result<Self>;

    fn new() -> Result<Self>
    where
        Self::Codec: Default,
    {
        Self::with_codec(Self::Codec::default())
    }

    fn create_import_storage(&self, data_dir: &Path) -> Result<impl ImportStorage + use<Self>>;

    fn open(&self, data_dir: &Path) -> Result<impl Lookups>;
}

pub trait ImportStorage: Send {
    fn begin_write(&mut self) -> Result<impl ImportTransaction>;
}

pub trait ImportTransaction: Send {
    fn open_tables(&mut self) -> Result<impl ImportTables + '_>;

    fn commit(self) -> Result<()>;
}

pub trait ImportTables: Send {
    fn insert_record(&mut self, record: impl Into<Record>) -> Result<RecordId>;

    fn insert_term(&mut self, term: &Term, record_id: RecordId) -> Result<()>;
}

pub trait Lookups {
    fn lookup_lemma(&self, lemma: &str) -> Result<Vec<(TermPart, Record)>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermPart {
    Headword,
    Reading,
}
