use {
    crate::codec::{Codec, Decoder, Encoder},
    either::Either,
    eyre::{Context, Result, eyre},
    heed::{
        Database, DatabaseFlags, DatabaseOpenOptions, Env, EnvFlags, EnvOpenOptions, RoTxn, RwTxn,
        WithTls, byteorder, types::Str,
    },
    std::{
        fs, iter,
        marker::PhantomData,
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
const NUM_DBS: u32 = 3;

pub struct Storage<C> {
    dictionaries_dir: PathBuf,
    _phantom: PhantomData<C>,
}

impl<C: Codec> Storage<C> {
    pub fn new(dictionaries_dir: impl Into<PathBuf>) -> Self {
        Self {
            dictionaries_dir: dictionaries_dir.into(),
            _phantom: PhantomData,
        }
    }
}

impl<C: Codec> super::Storage for Storage<C> {
    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn begin_import(&self, dictionary_id: DictionaryId) -> Result<ImportStorage<C>> {
        let db_path = self
            .dictionaries_dir
            .join(dictionary_id.0.as_hyphenated().to_string());
        fs::create_dir_all(&db_path)
            .wrap_err_with(|| eyre!("failed to create directory {db_path:?}"))?;

        Ok(ImportStorage {
            // TODO safety comment
            env: unsafe { env_open_options().open(&db_path) }
                .wrap_err_with(|| eyre!("failed to open database env at {db_path:?}"))?,
            db_path,
            next_record_id: AtomicU64::new(0),
            _phantom: PhantomData,
        })
    }
}

fn env_open_options() -> EnvOpenOptions {
    let mut opts = EnvOpenOptions::new();
    opts.map_size(128 * 1024 * 1024 * 1024); // TODO is this the max db size?
    opts.max_dbs(NUM_DBS);
    opts
}

pub struct ImportStorage<C> {
    env: Env,
    db_path: PathBuf,
    next_record_id: AtomicU64,
    _phantom: PhantomData<C>,
}

impl<C: Codec> super::ImportStorage for ImportStorage<C> {
    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn begin_write(&mut self) -> Result<ImportTransaction<'_, C>> {
        Ok(ImportTransaction {
            txn: self
                .env
                .write_txn()
                .wrap_err("failed to begin write transaction")?,
            next_record_id: &self.next_record_id,
            env: &self.env,
            _phantom: PhantomData,
        })
    }

    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn open_lookups(self) -> Result<LookupStorage<C>> {
        drop(self.env);

        // TODO safety comment
        let env = unsafe {
            env_open_options()
                .flags(EnvFlags::READ_ONLY)
                .open(&self.db_path)
        }
        .wrap_err("failed to re-open database env for reading")?;

        Ok(LookupStorage {
            env,
            _phantom: PhantomData,
        })
    }
}

pub struct ImportTransaction<'s, C: Codec> {
    txn: RwTxn<'s>,
    next_record_id: &'s AtomicU64,
    env: &'s Env,
    _phantom: PhantomData<C>,
}

impl<'s, C: Codec> super::ImportTransaction for ImportTransaction<'s, C> {
    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn open_tables(&mut self) -> Result<ImportTables<'s, '_, C::Encoder>> {
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
            txn: &mut self.txn,
            next_record_id: self.next_record_id,
            encoder: C::encoder(),
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

pub struct ImportTables<'s, 't, E> {
    txn: &'t mut RwTxn<'s>,
    records: Database<RecordIdTy, RecordTy>,
    headwords: Database<Str, RecordIdTy>,
    readings: Database<Str, RecordIdTy>,
    next_record_id: &'t AtomicU64,
    encoder: E,
}

impl<E: Encoder> super::ImportTables for ImportTables<'_, '_, E> {
    fn insert_record(&mut self, record: impl Into<Record>) -> Result<RecordId> {
        self.insert_record_(&record.into())
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

impl<E: Encoder> ImportTables<'_, '_, E> {
    fn insert_record_(&mut self, record: &Record) -> Result<RecordId> {
        let record_id = self.next_record_id.fetch_add(1, atomic::Ordering::SeqCst);
        let blob = self
            .encoder
            .encode(record)
            .wrap_err("failed to encode record")?;
        self.records
            .put(self.txn, &record_id, blob.as_ref())
            .wrap_err("failed to insert record")?;
        Ok(RecordId(record_id))
    }
}

pub struct LookupStorage<C> {
    env: Env,
    _phantom: PhantomData<C>,
}

impl<C: Codec> super::LookupStorage for LookupStorage<C> {
    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn lookups(&self) -> Result<Lookups<'_, C>> {
        let txn = self
            .env
            .read_txn()
            .wrap_err("failed to begin read transaction")?;

        Ok(Lookups {
            records: records_db_options(&self.env)
                .open(&txn)
                .wrap_err_with(|| eyre!("failed to create database `{RECORDS}`"))?
                .ok_or_else(|| eyre!("no database `{RECORDS}`"))?,
            headwords: term_db_options(&self.env, HEADWORDS)
                .open(&txn)
                .wrap_err_with(|| eyre!("failed to create database `{HEADWORDS}`"))?
                .ok_or_else(|| eyre!("no database `{HEADWORDS}`"))?,
            readings: term_db_options(&self.env, READINGS)
                .open(&txn)
                .wrap_err_with(|| eyre!("failed to create database `{READINGS}`"))?
                .ok_or_else(|| eyre!("no database `{READINGS}`"))?,
            txn,
            _phantom: PhantomData,
        })
    }
}

pub struct Lookups<'e, C> {
    records: Database<RecordIdTy, RecordTy>,
    headwords: Database<Str, RecordIdTy>,
    readings: Database<Str, RecordIdTy>,
    txn: RoTxn<'e, WithTls>,
    _phantom: PhantomData<C>,
}

impl<C: Codec> super::Lookups for Lookups<'_, C> {
    fn lookup_lemma(&self, lemma: &str) -> Result<Vec<Record>> {
        self.lookup_lemma_(lemma).collect()
    }
}

impl<C: Codec> Lookups<'_, C> {
    fn lookup_lemma_(&self, lemma: &str) -> impl Iterator<Item = Result<Record>> {
        let mut decoder = C::decoder();

        let get_ids = |db: &Database<Str, RecordIdTy>| {
            match db.get_duplicates(&self.txn, lemma) {
                Ok(Some(id_results)) => Either::Left(id_results.map(move |result| {
                    let (_, record_id) =
                        result.wrap_err_with(|| eyre!("failed to get single record ID"))?;
                    eyre::Ok(RecordId(record_id))
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
            .chain(get_ids(&self.headwords).map(|r| r.wrap_err("failed to query headwords")))
            .chain(get_ids(&self.readings).map(|r| r.wrap_err("failed to query readings")));

        #[expect(
            clippy::redundant_closure,
            reason = "without the closure, `get_record` becomes a `FnOnce`"
        )]
        ids.map(move |id| id.and_then(|id| get_record(id)))
    }
}
