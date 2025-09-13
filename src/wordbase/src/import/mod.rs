use {
    crate::db::{HEADWORDS, READINGS, RECORDS, RecordId},
    eyre::{Context, Result, eyre},
    redb::{
        Database, MultimapTable, MultimapTableHandle as _, Table, TableHandle, WriteTransaction,
    },
    rkyv::rancor::Panic,
    std::{
        path::PathBuf,
        sync::atomic::{self, AtomicU64},
    },
    tracing::debug,
    wordbase_api::{Record, Term},
};

pub mod yomitan;

pub trait OpenArchive: Send + Sync + Clone + 'static {
    fn open_archive(&self) -> Result<Box<dyn Archive>>;
}

impl<F: Fn() -> Result<Box<dyn Archive>> + Send + Sync + Clone + 'static> OpenArchive for F {
    fn open_archive(&self) -> Result<Box<dyn Archive>> {
        (self)()
    }
}

pub trait Archive: Send + Sync + std::io::Read + std::io::Seek + Unpin {}

impl<T: Send + Sync + Unpin + std::io::Read + std::io::Seek> Archive for T {}

pub trait FinishImport {
    fn finish(self: Box<Self>, storage: Storage) -> Result<()>;
}

impl<F: FnOnce(Storage) -> Result<()>> FinishImport for F {
    fn finish(self: Box<Self>, storage: Storage) -> Result<()> {
        (*self)(storage)
    }
}

//

#[derive(Debug)]
pub struct Storage {
    pub data_dir: PathBuf,
}

pub struct Transaction {
    // <https://github.com/cberner/redb/issues/1072>
    _db: Database,
    txn: WriteTransaction,
    next_record_id: AtomicU64,
}

pub struct Tables<'txn> {
    records: Table<'txn, RecordId, &'static [u8]>,
    headwords: MultimapTable<'txn, &'static str, RecordId>,
    readings: MultimapTable<'txn, &'static str, RecordId>,
    next_record_id: &'txn AtomicU64,
}

impl Storage {
    fn begin_write(self) -> Result<Transaction> {
        let path = self.data_dir.join("dictionary.redb");

        debug!("Creating dictionary database at {path:?}");
        let db = Database::create(&path).wrap_err("failed to create database")?;

        debug!("Beginning write transaction");
        let txn = db.begin_write().wrap_err("failed to begin write")?;

        Ok(Transaction {
            _db: db,
            txn,
            next_record_id: AtomicU64::new(0),
        })
    }
}

impl Transaction {
    pub fn open_tables(&self) -> Result<Tables<'_>> {
        Ok(Tables {
            records: self
                .txn
                .open_table(RECORDS)
                .wrap_err_with(|| eyre!("failed to open table `{}`", RECORDS.name()))?,
            headwords: self
                .txn
                .open_multimap_table(HEADWORDS)
                .wrap_err_with(|| eyre!("failed to open table `{}`", HEADWORDS.name()))?,
            readings: self
                .txn
                .open_multimap_table(READINGS)
                .wrap_err_with(|| eyre!("failed to open table `{}`", READINGS.name()))?,
            next_record_id: &self.next_record_id,
        })
    }

    pub fn commit(self) -> Result<()> {
        self.txn.commit()?;
        Ok(())
    }
}

impl Tables<'_> {
    pub fn insert_record(&mut self, record: impl Into<Record>) -> Result<RecordId> {
        self._insert_record(record.into())
    }

    fn _insert_record(&mut self, record: Record) -> Result<RecordId> {
        let record_id = RecordId(self.next_record_id.fetch_add(1, atomic::Ordering::SeqCst));

        // TODO arena and better alloc stuff
        // let bytes = rkyv::api::high::to_bytes::<Panic>(&record).unwrap();
        let bytes = rmp_serde::to_vec(&record).unwrap();

        tracing::info!("record size = {}", bytes.len());

        self.records
            .insert(&record_id, bytes.as_slice())
            .wrap_err("failed to insert record")?;
        Ok(record_id)
    }

    pub fn insert_term(&mut self, term: &Term, record_id: RecordId) -> Result<()> {
        if let Some(headword) = term.headword() {
            self.headwords
                .insert(headword.as_str(), record_id)
                .wrap_err("failed to insert headword")?;
        }
        if let Some(reading) = term.reading() {
            self.readings
                .insert(reading.as_str(), record_id)
                .wrap_err("failed to insert reading")?;
        }
        Ok(())
    }
}
