use {
    either::Either,
    eyre::{Context, Result, eyre},
    heed::{Database, Env, EnvOpenOptions, RoTxn, RwTxn, byteorder, types::Str},
    std::{
        fs, iter,
        path::PathBuf,
        sync::atomic::{self, AtomicU64},
    },
    wordbase_api::{DictionaryId, Record, RecordId, Term},
};

type RecordIdTy = heed::types::U64<byteorder::LE>;
type RecordTy = heed::types::Bytes;

const RECORDS: &str = "records";
const HEADWORDS: &str = "headwords";
const READINGS: &str = "readings";

pub struct Storage {
    pub dictionaries_dir: PathBuf,
}

impl Storage {
    pub fn new(dictionaries_dir: impl Into<PathBuf>) -> Self {
        Self {
            dictionaries_dir: dictionaries_dir.into(),
        }
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

        Ok(ImportStorage {
            // TODO safety comment
            env: unsafe {
                EnvOpenOptions::new()
                    .map_size(1024 * 1024 * 1024)
                    .max_dbs(3)
                    .open(&dictionary_dir)
            }
            .wrap_err_with(|| eyre!("failed to open database env at {dictionary_dir:?}"))?,
            next_record_id: AtomicU64::new(0),
        })
    }
}

pub struct ImportStorage {
    env: Env,
    next_record_id: AtomicU64,
}

impl ImportStorage {
    pub fn from_env(env: Env) -> Self {
        Self {
            env,
            next_record_id: AtomicU64::new(0),
        }
    }
}

impl super::ImportStorage for ImportStorage {
    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn begin_write(&mut self) -> Result<ImportTransaction<'_>> {
        Ok(ImportTransaction {
            txn: self
                .env
                .write_txn()
                .wrap_err("failed to begin write transaction")?,
            next_record_id: &self.next_record_id,
            env: &self.env,
        })
    }
}

pub struct ImportTransaction<'s> {
    txn: RwTxn<'s>,
    next_record_id: &'s AtomicU64,
    env: &'s Env,
}

impl<'s> super::ImportTransaction for ImportTransaction<'s> {
    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn open_tables(&mut self) -> Result<ImportTables<'s, '_>> {
        let records = self
            .env
            .create_database(&mut self.txn, Some(RECORDS))
            .wrap_err("failed to create table `{RECORDS}`")?;
        let headwords = self
            .env
            .create_database(&mut self.txn, Some(HEADWORDS))
            .wrap_err("failed to create table `{HEADWORDS}`")?;
        let readings = self
            .env
            .create_database(&mut self.txn, Some(READINGS))
            .wrap_err("failed to create table `{READINGS}`")?;

        Ok(ImportTables {
            records,
            headwords,
            readings,
            txn: &mut self.txn,
            next_record_id: &self.next_record_id,
        })
    }

    fn commit(self) -> Result<()> {
        self.txn.commit()?;
        Ok(())
    }
}

pub struct ImportTables<'s, 't> {
    txn: &'t mut RwTxn<'s>,
    records: Database<RecordIdTy, RecordTy>,
    headwords: Database<Str, RecordIdTy>,
    readings: Database<Str, RecordIdTy>,
    next_record_id: &'t AtomicU64,
}

impl super::ImportTables for ImportTables<'_, '_> {
    fn insert_record(&mut self, record: impl Into<Record>) -> Result<RecordId> {
        self._insert_record(record.into())
    }

    fn insert_term(&mut self, term: &Term, record_id: RecordId) -> Result<()> {
        if let Some(headword) = term.headword() {
            self.headwords
                .put(self.txn, headword.as_str(), &record_id.0)
                .wrap_err("failed to insert headword")?;
        }
        if let Some(reading) = term.reading() {
            self.readings
                .put(self.txn, reading.as_str(), &record_id.0)
                .wrap_err("failed to insert reading")?;
        }
        Ok(())
    }
}

impl ImportTables<'_, '_> {
    fn _insert_record(&mut self, record: Record) -> Result<RecordId> {
        let record_id = self.next_record_id.fetch_add(1, atomic::Ordering::SeqCst);

        // TODO codec
        let buf = rmp_serde::to_vec(&record).unwrap();

        self.records
            .put(self.txn, &record_id, &buf)
            .wrap_err("failed to insert record")?;
        Ok(RecordId(record_id))
    }
}

pub struct Lookups<'s> {
    txn: RoTxn<'s>,
    records: Database<RecordIdTy, RecordTy>,
    headwords: Database<Str, RecordIdTy>,
    readings: Database<Str, RecordIdTy>,
}

impl super::Lookups for Lookups<'_> {
    fn lookup_lemma(&self, lemma: &str) -> impl Iterator<Item = Result<Record>> {
        let ids_for_db = |db: &Database<Str, RecordIdTy>| {
            match db.get_duplicates(&self.txn, lemma) {
                Ok(Some(id_results)) => Either::Left(id_results.map(|result| {
                    let (_, record_id) = result?;
                    eyre::Ok(record_id)
                })),
                Ok(None) => Either::Right(None),
                Err(err) => Either::Right(Some(Err(err.into()))),
            }
            .into_iter()
        };

        let record_ids = iter::empty()
            .chain(ids_for_db(&self.headwords))
            .chain(ids_for_db(&self.readings));

        record_ids.map(|record_id| {
            let record_id = record_id?;
            let blob = self
                .records
                .get(&self.txn, &record_id)
                .wrap_err_with(|| eyre!("failed to get record {record_id:?}"))?
                .ok_or_else(|| eyre!("no record {record_id:?}"))?;
            // TODO codec
            let record = rmp_serde::from_slice(blob)
                .wrap_err_with(|| eyre!("failed to deserialize record {record_id:?}"))?;
            Ok(record)
        })
    }
}
