use {
    crate::{backend::TermPart, codec::Decoder},
    derive_more::Debug,
    either::Either,
    eyre::{Context, Result, eyre},
    heed::{
        Database, DatabaseFlags, DatabaseOpenOptions, Env, EnvFlags, EnvOpenOptions, RoTxn, RwTxn,
        WithTls, byteorder, types::Str,
    },
    std::{
        iter,
        path::Path,
        sync::{Mutex, MutexGuard},
    },
    wordbase_api::{Record, RecordId, Term},
};

type RecordIdTy = heed::types::U64<byteorder::LE>;
type RecordTy = heed::types::Bytes;

const RECORDS: &str = "records";
const HEADWORDS: &str = "headwords";
const READINGS: &str = "readings";
const NUM_DBS: u32 = 3;
const MAP_SIZE: usize = 128 * 1024 * 1024 * 1024; // TODO is this the max db size?

#[derive(Debug, Clone)]
pub struct Backend;

impl super::Backend for Backend {
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
        let env = unsafe { env_open_options().flags(EnvFlags::READ_ONLY).open(data_dir) }
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

fn env_open_options() -> EnvOpenOptions {
    let mut opts = EnvOpenOptions::new();
    opts.map_size(MAP_SIZE);
    opts.max_dbs(NUM_DBS);
    opts
}

pub struct ImportStorage {
    env: Env,
}

impl super::ImportStorage for ImportStorage {
    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn transaction(&mut self) -> Result<ImportTransaction<'_>> {
        Ok(ImportTransaction {
            txn: self
                .env
                .write_txn()
                .wrap_err("failed to begin write transaction")?,
            env: &self.env,
        })
    }
}

pub struct ImportTransaction<'stg> {
    txn: RwTxn<'stg>,
    env: &'stg Env,
}

impl<'stg> super::ImportTransaction for ImportTransaction<'stg> {
    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn open_tables(&mut self) -> Result<ImportTables<'stg, '_>> {
        Ok(ImportTables {
            records: records_db_options(self.env)
                .create(&mut self.txn)
                .wrap_err_with(|| eyre!("failed to create database `{RECORDS}`"))?,
            headwords: term_db_options(self.env, HEADWORDS)
                .create(&mut self.txn)
                .wrap_err_with(|| eyre!("failed to create database `{HEADWORDS}`"))?,
            readings: term_db_options(self.env, READINGS)
                .create(&mut self.txn)
                .wrap_err_with(|| eyre!("failed to create database `{READINGS}`"))?,
            txn: Mutex::new(&mut self.txn),
        })
    }

    fn commit(self) -> Result<()> {
        self.txn.commit()?;
        Ok(())
    }
}

fn records_db_options<'env: 'name, 'name>(
    env: &'env Env,
) -> DatabaseOpenOptions<'env, 'name, WithTls, RecordIdTy, RecordTy> {
    env.database_options()
        .name(RECORDS)
        .types::<RecordIdTy, RecordTy>()
}

fn term_db_options<'env: 'name, 'name>(
    env: &'env Env,
    name: &'name str,
) -> DatabaseOpenOptions<'env, 'name, WithTls, Str, RecordIdTy> {
    env.database_options()
        .name(name)
        .flags(DatabaseFlags::DUP_SORT)
        .types::<Str, RecordIdTy>()
}

pub struct ImportTables<'stg, 'txn> {
    txn: Mutex<&'txn mut RwTxn<'stg>>,
    records: Database<RecordIdTy, RecordTy>,
    headwords: Database<Str, RecordIdTy>,
    readings: Database<Str, RecordIdTy>,
}

impl<'stg, 'txn> super::ImportTables for ImportTables<'stg, 'txn> {
    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn batch(&self) -> ImportBatch<'stg, 'txn, '_> {
        ImportBatch {
            txn: self.txn.lock().expect("mutex poisoned"),
            records: &self.records,
            headwords: &self.headwords,
            readings: &self.readings,
        }
    }
}

pub struct ImportBatch<'stg, 'txn, 'tbl> {
    txn: MutexGuard<'tbl, &'txn mut RwTxn<'stg>>,
    records: &'tbl Database<RecordIdTy, RecordTy>,
    headwords: &'tbl Database<Str, RecordIdTy>,
    readings: &'tbl Database<Str, RecordIdTy>,
}

impl super::ImportBatch for ImportBatch<'_, '_, '_> {
    fn insert_record(&mut self, record_id: RecordId, record: &[u8]) -> Result<()> {
        self.records
            .put(&mut self.txn, &record_id.0, record)
            .wrap_err("failed to insert record")?;
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
    records: Database<RecordIdTy, RecordTy>,
    headwords: Database<Str, RecordIdTy>,
    readings: Database<Str, RecordIdTy>,
    #[debug(skip)]
    txn: RoTxn<'static, WithTls>,
}

impl super::Lookups for Lookups {
    fn lookup_lemma<D: Decoder>(
        &self,
        make_decoder: impl Fn() -> D,
        lemma: &str,
    ) -> Result<Vec<(TermPart, Record)>> {
        self.lookup_lemma_(make_decoder(), lemma).collect()
    }
}

impl Lookups {
    fn lookup_lemma_(
        &self,
        mut decoder: impl Decoder,
        lemma: &str,
    ) -> impl Iterator<Item = Result<(TermPart, Record)>> {
        let get_ids = |part: TermPart, db: &Database<Str, RecordIdTy>| {
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

        ids.map(move |id| id.and_then(|(part, id)| get_record(id).map(|record| (part, record))))
    }
}
