use {
    eyre::{Context, Result, eyre},
    rocksdb::{ColumnFamily, DB},
    std::{
        fs,
        path::PathBuf,
        sync::atomic::{self, AtomicU64},
    },
    wordbase_api::{DictionaryId, Record, RecordId, Term},
};

const RECORDS: &str = "records";
const HEADWORDS: &str = "headwords";
const READINGS: &str = "readings";

pub struct Storage {
    pub env: rocksdb::Env,
    pub dictionaries_dir: PathBuf,
}

impl Storage {
    pub fn new(dictionaries_dir: impl Into<PathBuf>) -> Result<Self> {
        Ok(Self {
            env: rocksdb::Env::new().wrap_err("failed to create env")?,
            dictionaries_dir: dictionaries_dir.into(),
        })
    }
}

impl super::Storage for Storage {
    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn begin_import(&self, dictionary_id: DictionaryId) -> Result<ImportStorage> {
        let dictionary_dir = self
            .dictionaries_dir
            .join(dictionary_id.0.as_hyphenated().to_string());
        fs::create_dir_all(&dictionary_dir)
            .wrap_err_with(|| eyre!("failed to create directory {dictionary_dir:?}"))?;

        let mut options = rocksdb::Options::default();
        options.set_env(&self.env);
        options.create_if_missing(true);
        // <https://github.com/facebook/rocksdb/wiki/RocksDB-FAQ>
        // "What's the fastest way to load data into RocksDB?"
        options.prepare_for_bulk_load();

        let mut db = DB::open(&options, &dictionary_dir)
            .wrap_err_with(|| eyre!("failed to create database at {dictionary_dir:?}"))?;

        db.create_cf(RECORDS, &options)
            .wrap_err_with(|| eyre!("failed to create column family `{RECORDS}`"))?;
        db.create_cf(HEADWORDS, &options)
            .wrap_err_with(|| eyre!("failed to create column family `{HEADWORDS}`"))?;
        db.create_cf(READINGS, &options)
            .wrap_err_with(|| eyre!("failed to create column family `{READINGS}`"))?;

        Ok(ImportStorage {
            db,
            next_record_id: AtomicU64::new(0),
        })
    }
}

pub struct ImportStorage {
    db: DB,
    next_record_id: AtomicU64,
}

impl super::ImportStorage for ImportStorage {
    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn begin_write(&mut self) -> Result<ImportTransaction<'_>> {
        Ok(ImportTransaction {
            db: &self.db,
            next_record_id: &self.next_record_id,
        })
    }
}

pub struct ImportTransaction<'s> {
    db: &'s DB,
    next_record_id: &'s AtomicU64,
}

impl super::ImportTransaction for ImportTransaction<'_> {
    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn open_tables(&mut self) -> Result<ImportTables<'_>> {
        Ok(ImportTables {
            records: self
                .db
                .cf_handle(RECORDS)
                .ok_or_else(|| eyre!("no column family `{RECORDS}`"))?,
            headwords: self
                .db
                .cf_handle(HEADWORDS)
                .ok_or_else(|| eyre!("no column family `{HEADWORDS}`"))?,
            readings: self
                .db
                .cf_handle(READINGS)
                .ok_or_else(|| eyre!("no column family `{READINGS}`"))?,
            db: self.db,
            next_record_id: &self.next_record_id,
        })
    }

    fn commit(self) -> Result<()> {
        // <https://github.com/facebook/rocksdb/wiki/RocksDB-FAQ>
        // "What's the fastest way to load data into RocksDB?"
        self.db.compact_range(None::<&[u8]>, None::<&[u8]>);
        Ok(())
    }
}

pub struct ImportTables<'s> {
    records: &'s ColumnFamily,
    headwords: &'s ColumnFamily,
    readings: &'s ColumnFamily,
    db: &'s DB,
    next_record_id: &'s AtomicU64,
}

impl super::ImportTables for ImportTables<'_> {
    fn insert_record(&mut self, record: impl Into<Record>) -> Result<RecordId> {
        self._insert_record(record.into())
    }

    fn insert_term(&mut self, term: &Term, record_id: RecordId) -> Result<()> {
        if let Some(headword) = term.headword() {
            self.db
                .put_cf(self.headwords, headword.as_bytes(), id_to_bytes(record_id))
                .wrap_err("failed to insert headword")?;
        }
        if let Some(reading) = term.reading() {
            self.db
                .put_cf(self.readings, reading.as_bytes(), id_to_bytes(record_id))
                .wrap_err("failed to insert reading")?;
        }
        Ok(())
    }
}

impl ImportTables<'_> {
    fn _insert_record(&mut self, record: Record) -> Result<RecordId> {
        let record_id = RecordId(self.next_record_id.fetch_add(1, atomic::Ordering::SeqCst));

        // TODO codec
        let buf = rmp_serde::to_vec(&record).unwrap();

        self.db
            .put_cf(self.records, id_to_bytes(record_id), &buf)
            .wrap_err("failed to insert record")?;
        Ok(record_id)
    }
}

fn id_to_bytes(record_id: RecordId) -> [u8; size_of::<u64>()] {
    record_id.0.to_le_bytes()
}
