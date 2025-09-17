use {
    eyre::Result,
    wordbase_api::{DictionaryId, Record, RecordId, Term},
};

#[cfg(feature = "storage-heed")]
pub mod heed;
#[cfg(feature = "storage-heed")]
pub type Heed<C> = heed::Storage<C>;

#[cfg(feature = "storage-redb")]
pub mod redb;
#[cfg(feature = "storage-redb")]
pub type Redb<C> = redb::Storage<C>;

#[cfg(feature = "storage-rocksdb")]
pub mod rocksdb;
#[cfg(feature = "storage-rocksdb")]
pub type RocksDb<C> = rocksdb::Storage<C>;

pub trait Storage {
    fn begin_import(&self, dictionary_id: DictionaryId) -> Result<impl ImportStorage>;
}

pub trait ImportStorage {
    fn begin_write(&mut self) -> Result<impl ImportTransaction>;

    fn open_lookups(self) -> Result<impl LookupStorage>;
}

pub trait ImportTransaction {
    fn open_tables(&mut self) -> Result<impl ImportTables + '_>;

    fn commit(self) -> Result<()>;
}

pub trait ImportTables: Send {
    fn insert_record(&mut self, record: impl Into<Record>) -> Result<RecordId>;

    fn insert_term(&mut self, term: &Term, record_id: RecordId) -> Result<()>;
}

pub trait LookupStorage {
    fn lookups(&self) -> Result<impl Lookups>;
}

pub trait Lookups {
    fn lookup_lemma(&self, lemma: &str) -> Result<Vec<Record>>;
}
