use {
    derive_more::Debug,
    either::Either,
    eyre::{Context, Result, eyre},
    heed::{
        Database, DatabaseFlags, DatabaseOpenOptions, Env, EnvFlags, EnvOpenOptions, RoTxn, RwTxn,
        WithoutTls, byteorder,
        types::{Bytes, Str},
    },
    std::{
        iter,
        path::Path,
        sync::{Mutex, MutexGuard},
    },
    wordbase_api::{Record, RecordId, Term, TermPart},
    wordbase_storage::{
        backend::{self, RecordRow},
        codec::Decoder,
    },
};

type U64LE = heed::types::U64<byteorder::LE>;

const RECORDS: &str = "records";
const HEADWORDS: &str = "headwords";
const READINGS: &str = "readings";
const NUM_DBS: u32 = 3;
const MAP_SIZE: usize = 128 * 1024 * 1024 * 1024; // TODO is this the max db size?

fn env_open_options() -> EnvOpenOptions {
    let mut opts = heed::EnvOpenOptions::new();
    opts.map_size(MAP_SIZE);
    opts.max_dbs(NUM_DBS);
    opts
}

fn records_db_options<'env: 'name, 'name, T>(
    env: &'env Env<T>,
) -> DatabaseOpenOptions<'env, 'name, T, U64LE, Bytes> {
    env.database_options().name(RECORDS).types::<U64LE, Bytes>()
}

fn term_db_options<'env: 'name, 'name, T>(
    env: &'env Env<T>,
    name: &'name str,
) -> DatabaseOpenOptions<'env, 'name, T, Str, U64LE> {
    env.database_options()
        .name(name)
        .flags(DatabaseFlags::DUP_SORT)
        .types::<Str, U64LE>()
}

#[derive(Debug, Clone)]
pub struct Backend;

impl backend::Backend for Backend {
    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn import(data_dir: &Path) -> Result<ImportStorage> {
        Ok(ImportStorage {
            // TODO safety comment
            env: unsafe { env_open_options().open(data_dir) }
                .wrap_err("failed to open database env")?,
        })
    }

    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn open(data_dir: &Path) -> Result<Lookups> {
        // TODO safety comment
        let env = unsafe {
            env_open_options()
                .read_txn_without_tls()
                .flags(EnvFlags::READ_ONLY)
                .open(data_dir)
        }
        .wrap_err("failed to open database env")?;
        let txn = env
            .clone()
            .static_read_txn()
            .wrap_err("failed to begin read transaction")?;

        Ok(Lookups {
            records: records_db_options(&env)
                .open(&txn)
                .wrap_err_with(|| eyre!("failed to open database `{RECORDS}`"))?
                .ok_or_else(|| eyre!("no database `{RECORDS}`"))?,
            headwords: term_db_options(&env, HEADWORDS)
                .open(&txn)
                .wrap_err_with(|| eyre!("failed to open database `{HEADWORDS}`"))?
                .ok_or_else(|| eyre!("no database `{HEADWORDS}`"))?,
            readings: term_db_options(&env, READINGS)
                .open(&txn)
                .wrap_err_with(|| eyre!("failed to open database `{READINGS}`"))?
                .ok_or_else(|| eyre!("no database `{READINGS}`"))?,
            txn,
        })
    }
}

#[derive(Debug)]
pub struct ImportStorage {
    env: Env,
}

impl backend::ImportStorage for ImportStorage {
    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn transaction(&mut self) -> Result<ImportTransaction<'_>> {
        let mut txn = self
            .env
            .write_txn()
            .wrap_err("failed to begin transaction")?;
        Ok(ImportTransaction {
            records: records_db_options(&self.env)
                .create(&mut txn)
                .wrap_err_with(|| eyre!("failed to create database `{RECORDS}`"))?,
            headwords: term_db_options(&self.env, HEADWORDS)
                .create(&mut txn)
                .wrap_err_with(|| eyre!("failed to create database `{HEADWORDS}`"))?,
            readings: term_db_options(&self.env, READINGS)
                .create(&mut txn)
                .wrap_err_with(|| eyre!("failed to create database `{READINGS}`"))?,
            txn: Mutex::new(txn),
        })
    }

    fn commit(self) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct ImportTransaction<'stg> {
    records: Database<U64LE, Bytes>,
    headwords: Database<Str, U64LE>,
    readings: Database<Str, U64LE>,
    #[debug(skip)]
    txn: Mutex<RwTxn<'stg>>,
}

impl<'stg> backend::ImportTransaction for ImportTransaction<'stg> {
    type Batch<'txn>
        = ImportBatch<'stg, 'txn>
    where
        Self: 'txn;

    fn batch(&self) -> Result<Self::Batch<'_>> {
        Ok(ImportBatch {
            txn: self.txn.lock().expect("txn poisoned"),
            records: &self.records,
            headwords: &self.headwords,
            readings: &self.readings,
        })
    }

    fn commit(self) -> Result<()> {
        self.txn.into_inner().expect("txn poisoned").commit()?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct ImportBatch<'stg, 'txn> {
    #[debug(skip)]
    txn: MutexGuard<'txn, RwTxn<'stg>>,
    records: &'txn Database<U64LE, Bytes>,
    headwords: &'txn Database<Str, U64LE>,
    readings: &'txn Database<Str, U64LE>,
}

impl backend::ImportBatch for ImportBatch<'_, '_> {
    fn insert_record(&mut self, record_id: RecordId, record: &[u8]) -> Result<()> {
        self.records.put(&mut self.txn, &record_id.0, record)?;
        Ok(())
    }

    fn insert_term(&mut self, term: &Term, record_id: RecordId) -> Result<()> {
        if let Some(headword) = term.headword() {
            self.headwords
                .put(&mut self.txn, headword.as_str(), &record_id.0)
                .wrap_err("failed to insert headword")?;
        }
        if let Some(reading) = term.reading() {
            self.readings
                .put(&mut self.txn, reading.as_str(), &record_id.0)
                .wrap_err("failed to insert reading")?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Lookups {
    records: Database<U64LE, Bytes>,
    headwords: Database<Str, U64LE>,
    readings: Database<Str, U64LE>,
    #[debug(skip)]
    txn: RoTxn<'static, WithoutTls>,
}

impl backend::Lookups for Lookups {
    fn lookup_lemma<D: Decoder>(
        &self,
        make_decoder: impl Fn() -> D,
        lemma: &str,
    ) -> Result<Vec<RecordRow>> {
        self.lookup_lemma_(make_decoder(), lemma).collect()
    }
}

impl Lookups {
    fn lookup_lemma_(
        &self,
        mut decoder: impl Decoder,
        lemma: &str,
    ) -> impl Iterator<Item = Result<RecordRow>> {
        let get_ids = |part: TermPart, db: &Database<Str, U64LE>| {
            match db.get_duplicates(&self.txn, lemma) {
                Ok(Some(id_results)) => Either::Left(id_results.map(move |result| {
                    let (_, record_id) =
                        result.wrap_err_with(|| eyre!("failed to get single record ID"))?;
                    eyre::Ok((part, RecordId(record_id)))
                })),
                Ok(None) => Either::Right(None),
                Err(err) => Either::Right(Some(
                    Err(err).wrap_err("failed to get all record IDs for key"),
                )),
            }
            .into_iter()
        };

        let mut get_record = move |id: RecordId| -> Result<Record> {
            let blob = self
                .records
                .get(&self.txn, &id.0)
                .wrap_err_with(|| eyre!("failed to get {id:?}"))?
                .ok_or_else(|| eyre!("no record {id:?}"))?;
            let record = decoder
                .decode(blob)
                .wrap_err_with(|| eyre!("failed to decode {id:?}"))?;
            Ok(record)
        };

        let ids = iter::empty()
            .chain(
                get_ids(TermPart::Headword, &self.headwords)
                    .map(|r| r.wrap_err("failed to query headwords")),
            )
            .chain(
                get_ids(TermPart::Reading, &self.readings)
                    .map(|r| r.wrap_err("failed to query readings")),
            );

        ids.map(move |id| {
            id.and_then(|(term_part, record_id)| {
                get_record(record_id).map(|record| RecordRow {
                    term_part,
                    record_id,
                    record,
                })
            })
        })
    }
}
