use {
    crate::codec::Codec,
    eyre::Result,
    std::path::Path,
    wordbase_api::{Record, RecordId, Term},
};

pub trait Storage: Send + Sync + Clone + 'static {
    type Codec: Codec;

    fn kind() -> StorageKind;

    fn begin_import(&self, data_dir: &Path) -> Result<impl ImportStorage + use<Self>>;
}

pub trait ImportStorage: Send {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum StorageKind {
    Heed,
    Redb,
    RocksDb,
}

pub enum InbuiltLookupStorage<C> {
    Heed(heed::Storage<C>),
    Redb(redb::Storage<C>),
    RocksDb(rocksdb::Storage<C>),
}

impl<C: Codec> LookupStorage for InbuiltLookupStorage<C> {
    fn lookups(&self) -> Result<InbuiltLookups<'_, C>> {}
}

pub enum InbuiltLookups<'e, C> {
    Heed(heed::Lookups<'e, C>),
    Redb(redb::Lookups<C>),
    RocksDb(rocksdb::Lookups<'e, C>),
}

impl<C: Codec> Lookups for InbuiltLookups<'_, C> {
    fn lookup_lemma(&self, lemma: &str) -> Result<Vec<Record>> {
        match self {
            Self::Heed(this) => this.lookup_lemma(lemma),
            Self::Redb(this) => this.lookup_lemma(lemma),
            Self::RocksDb(this) => this.lookup_lemma(lemma),
        }
    }
}
