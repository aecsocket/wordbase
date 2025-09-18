use {
    crate::{
        codec::{Codec, Decoder, Encoder},
        storage::TermPart,
    },
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
        sync::atomic::{self, AtomicU64},
    },
    wordbase_api::{Record, RecordId, Term},
};

type RecordIdTy = heed::types::U64<byteorder::LE>;
type RecordTy = heed::types::Bytes;

const RECORDS: &str = "records";
const HEADWORDS: &str = "headwords";
const READINGS: &str = "readings";
const NUM_DBS: u32 = 3;

#[derive(Debug, Clone)]
pub struct Storage<C> {
    codec: C,
}

impl<C: Codec> super::Storage for Storage<C> {
    type Codec = C;

    fn with_codec(codec: Self::Codec) -> Result<Self> {
        Ok(Self { codec })
    }

    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn create_import_storage(&self, data_dir: &Path) -> Result<ImportStorage<C>> {
        Ok(ImportStorage {
            // TODO safety comment
            env: unsafe { env_open_options().open(data_dir) }
                .wrap_err("failed to open database env")?,
            next_record_id: AtomicU64::new(0),
            codec: self.codec.clone(),
        })
    }

    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn open(&self, data_dir: &Path) -> Result<Lookups<C>> {
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
            codec: self.codec.clone(),
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
    next_record_id: AtomicU64,
    codec: C,
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
            codec: self.codec.clone(),
        })
    }
}

pub struct ImportTransaction<'s, C> {
    txn: RwTxn<'s>,
    next_record_id: &'s AtomicU64,
    env: &'s Env,
    codec: C,
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
            encoder: self.codec.encoder(),
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
        let id = self.next_record_id.fetch_add(1, atomic::Ordering::SeqCst);
        let blob = self
            .encoder
            .encode(record)
            .wrap_err("failed to encode record")?;
        self.records
            .put(self.txn, &id, blob.as_ref())
            .wrap_err("failed to insert record")?;
        Ok(RecordId(id))
    }
}

#[derive(Debug)]
pub struct Lookups<C> {
    records: Database<RecordIdTy, RecordTy>,
    headwords: Database<Str, RecordIdTy>,
    readings: Database<Str, RecordIdTy>,
    #[debug(skip)]
    txn: RoTxn<'static, WithTls>,
    codec: C,
}

impl<C: Codec> super::Lookups for Lookups<C> {
    fn lookup_lemma(&self, lemma: &str) -> Result<Vec<(TermPart, Record)>> {
        self.lookup_lemma_(lemma).collect()
    }
}

impl<C: Codec> Lookups<C> {
    fn lookup_lemma_(&self, lemma: &str) -> impl Iterator<Item = Result<(TermPart, Record)>> {
        let mut decoder = self.codec.decoder();

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
