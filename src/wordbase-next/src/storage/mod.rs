use {
    eyre::Result,
    std::path::Path,
    wordbase_api::{Record, Term},
};

pub mod redb;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordId(pub u64);

pub trait Storage {
    type DictionaryWriteStorage: DictionaryWriteStorage;

    fn begin_dictionary_write(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<Self::DictionaryWriteStorage>;
}

pub trait DictionaryWriteStorage: Send + Sync + Sized {
    type Tx: DictionaryWriteTx;

    fn begin(self) -> Result<Self::Tx>;
}

pub trait DictionaryWriteTx: Send + Sync {
    type Write<'tx>: DictionaryWrite
    where
        Self: 'tx;

    fn writer(&self) -> Result<Self::Write<'_>>;

    fn commit(self) -> Result<()>;
}

pub trait DictionaryWrite: Send + Sync {
    fn insert_record(&mut self, record: &Record) -> Result<RecordId>;

    fn insert_term(&mut self, term: &Term, record_id: RecordId) -> Result<()>;
}

// impl<T: DictionaryImport + ?Sized> DictionaryImport for &mut T {
//     fn insert_record(&mut self, record: &Record) -> Result<RecordId> {
//         (*self).insert_record(record)
//     }

//     fn insert_term(&mut self, term: &Term, record_id: RecordId) -> Result<()>
// {         (*self).insert_term(term, record_id)
//     }
// }
