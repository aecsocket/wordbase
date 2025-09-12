use {
    crate::storage::RecordId,
    eyre::{Context, Result, eyre},
    redb::{Database, MultimapTableDefinition, TableDefinition, TypeName, WriteTransaction},
    std::{
        any::type_name,
        path::Path,
        sync::atomic::{self, AtomicU64},
    },
    wordbase_api::{Record, Term},
};

pub struct Storage;

impl super::Storage for Storage {
    type DictionaryWriteStorage = DictionaryWriteStorage;

    fn begin_dictionary_write(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<Self::DictionaryWriteStorage> {
        let mut path = path.as_ref().to_path_buf();
        path.set_extension("redb");
        Ok(DictionaryWriteStorage {
            db: Database::create(path)?,
        })
    }
}

pub struct DictionaryWriteStorage {
    db: Database,
}

impl super::DictionaryWriteStorage for DictionaryWriteStorage {
    type Tx = DictionaryWriteTx;

    fn begin(self) -> Result<Self::Tx> {
        Ok(DictionaryWriteTx {
            tx: self.db.begin_write()?,
            next_record_id: AtomicU64::new(0),
        })
    }
}

pub struct DictionaryWriteTx {
    tx: WriteTransaction,
    next_record_id: AtomicU64,
}

impl super::DictionaryWriteTx for DictionaryWriteTx {
    type Write<'tx> = DictionaryWrite<'tx>;

    fn writer(&self) -> Result<Self::Write<'_>> {
        Ok(DictionaryWrite {
            records: self
                .tx
                .open_table(RECORDS)
                .wrap_err_with(|| eyre!("failed to open table `{RECORDS}`"))?,
            terms: self
                .tx
                .open_multimap_table(TERMS)
                .wrap_err_with(|| eyre!("failed to open table `{TERMS}`"))?,
            next_record_id: &self.next_record_id,
        })
    }

    fn commit(self) -> Result<()> {
        self.tx.commit()?;
        Ok(())
    }
}

pub struct DictionaryWrite<'tx> {
    records: redb::Table<'tx, RecordId, &'static str>,
    terms: redb::MultimapTable<'tx, &'static str, RecordId>,
    next_record_id: &'tx AtomicU64,
}

impl super::DictionaryWrite for DictionaryWrite<'_> {
    fn insert_record(&mut self, record: &Record) -> Result<RecordId> {
        let record_id = RecordId(self.next_record_id.fetch_add(1, atomic::Ordering::SeqCst));
        self.records
            .insert(&record_id, &*format!("TODO {record_id:?}"))?;
        Ok(record_id)
    }

    fn insert_term(&mut self, term: &Term, record_id: RecordId) -> Result<()> {
        if let Some(headword) = term.headword() {
            self.terms
                .insert(headword.as_str(), record_id)
                .wrap_err("failed to insert headword")?;
        }
        if let Some(reading) = term.reading() {
            self.terms
                .insert(reading.as_str(), record_id)
                .wrap_err("failed to insert reading")?;
        }
        Ok(())
    }
}

const RECORDS: TableDefinition<RecordId, &str> = TableDefinition::new("records");
const TERMS: MultimapTableDefinition<&str, RecordId> = MultimapTableDefinition::new("terms");

impl redb::Value for RecordId {
    type SelfType<'a> = Self;
    type AsBytes<'a> = <u64 as redb::Value>::AsBytes<'a>;

    fn fixed_width() -> Option<usize> {
        <u64 as redb::Value>::fixed_width()
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        Self(<u64 as redb::Value>::from_bytes(data))
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        <u64 as redb::Value>::as_bytes(&value.0)
    }

    fn type_name() -> redb::TypeName {
        TypeName::new(type_name::<Self>())
    }
}

impl redb::Key for RecordId {
    fn compare(data1: &[u8], data2: &[u8]) -> std::cmp::Ordering {
        <u64 as redb::Key>::compare(data1, data2)
    }
}
