use {
    eyre::Result,
    wordbase_api::{DictionaryId, Record, RecordId, Term},
};

#[cfg(feature = "heed")]
pub mod heed;
#[cfg(feature = "redb")]
pub mod redb;
pub mod rkyv;
#[cfg(feature = "rocksdb")]
pub mod rocksdb;

pub trait Storage {
    fn begin_import(&self, dictionary_id: DictionaryId) -> Result<impl ImportStorage>;

    // fn lookups(&self) -> impl Lookups;
}

pub trait ImportStorage {
    fn begin_write(&mut self) -> Result<impl ImportTransaction>;
}

pub trait ImportTransaction {
    fn open_tables(&mut self) -> Result<impl ImportTables + '_>;

    fn commit(self) -> Result<()>;
}

pub trait ImportTables: Send {
    fn insert_record(&mut self, record: impl Into<Record>) -> Result<RecordId>;

    fn insert_term(&mut self, term: &Term, record_id: RecordId) -> Result<()>;
}

pub trait Lookups {
    fn lookup_lemma(&self, lemma: &str) -> impl Iterator<Item = Result<Record>>;
}
