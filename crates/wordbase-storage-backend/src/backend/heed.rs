//! See [`Heed`].

use {
    derive_more::Debug,
    either::Either,
    eyre::{Context, OptionExt, Result, eyre},
    heed::{
        BytesDecode, BytesEncode, Database, DatabaseFlags, DatabaseOpenOptions, Env, EnvFlags,
        EnvOpenOptions, RoTxn, RwTxn, WithoutTls, byteorder,
        types::{Bytes, Str},
    },
    std::{
        borrow::Cow,
        iter,
        path::Path,
        sync::{Mutex, MutexGuard},
    },
    wordbase_api::{NoHeadwordOrReading, Record, RecordId, Term},
    wordbase_storage_api::{
        backend::{self, RecordRow},
        codec::Decoder,
    },
};

/// [`backend::Backend`] implementation which uses the [`heed`] wrapper around
/// [LMDB](https://en.wikipedia.org/wiki/Lightning_Memory-Mapped_Database).
#[derive(Debug, Clone)]
pub struct Heed;

impl backend::Backend for Heed {
    type Lookups = LookupStorage;

    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn create_import_storage(data_dir: &Path) -> Result<ImportStorage> {
        Ok(ImportStorage {
            // TODO safety comment
            env: unsafe { env_open_options().open(data_dir) }
                .wrap_err("failed to open database env")?,
        })
    }

    fn open(data_dir: &Path) -> Result<Self::Lookups> {
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

        Ok(LookupStorage {
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

const RECORDS: &str = "records";
const HEADWORDS: &str = "headwords";
const READINGS: &str = "readings";
const NUM_DBS: u32 = 3;
const MAP_SIZE: usize = 128 * 1024 * 1024 * 1024; // TODO is this the max db size?

type U64LE = heed::types::U64<byteorder::LE>;

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
) -> DatabaseOpenOptions<'env, 'name, T, Str, TermEntry<'static>> {
    env.database_options()
        .name(name)
        .flags(DatabaseFlags::DUP_SORT)
        .types::<Str, TermEntry<'static>>()
}

/// [`backend::ImportStorage`] for [`Heed`].
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

/// [`backend::ImportTransaction`] for [`Heed`].
#[derive(Debug)]
pub struct ImportTransaction<'stg> {
    records: Database<U64LE, Bytes>,
    headwords: Database<Str, TermEntry<'static>>,
    readings: Database<Str, TermEntry<'static>>,
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

/// [`backend::ImportBatch`] for [`Heed`].
#[derive(Debug)]
pub struct ImportBatch<'stg, 'txn> {
    #[debug(skip)]
    txn: MutexGuard<'txn, RwTxn<'stg>>,
    records: &'txn Database<U64LE, Bytes>,
    headwords: &'txn Database<Str, TermEntry<'static>>,
    readings: &'txn Database<Str, TermEntry<'static>>,
}

impl backend::ImportBatch for ImportBatch<'_, '_> {
    fn insert_record(&mut self, record_id: RecordId, record: &[u8]) -> Result<()> {
        self.records.put(&mut self.txn, &record_id.0, record)?;
        Ok(())
    }

    fn insert_term(&mut self, term: &Term, record_id: RecordId) -> Result<()> {
        let headword = term.headword().map(|s| s.as_str());
        let reading = term.reading().map(|s| s.as_str());

        if let Some(headword) = headword {
            self.headwords
                .put(
                    &mut self.txn,
                    headword,
                    &TermEntry {
                        record_id,
                        secondary: reading.unwrap_or(""),
                    },
                )
                .wrap_err("failed to insert headword")?;
        }
        if let Some(reading) = reading {
            self.readings
                .put(
                    &mut self.txn,
                    reading,
                    &TermEntry {
                        record_id,
                        secondary: headword.unwrap_or(""),
                    },
                )
                .wrap_err("failed to insert reading")?;
        }
        Ok(())
    }
}

#[derive(Debug)]
struct TermEntry<'a> {
    record_id: RecordId,
    secondary: &'a str,
}

const RECORD_ID_SIZE: usize = size_of::<RecordId>();

impl<'a> BytesEncode<'a> for TermEntry<'a> {
    type EItem = Self;

    fn bytes_encode(item: &'a Self::EItem) -> Result<Cow<'a, [u8]>, heed::BoxedError> {
        let mut buf = vec![0; RECORD_ID_SIZE + item.secondary.len()];
        buf[..RECORD_ID_SIZE].copy_from_slice(&item.record_id.0.to_le_bytes());
        buf[RECORD_ID_SIZE..].copy_from_slice(item.secondary.as_bytes());

        Ok(Cow::Owned(buf))
    }
}

impl<'a> BytesDecode<'a> for TermEntry<'a> {
    type DItem = Self;

    fn bytes_decode(bytes: &'a [u8]) -> Result<Self::DItem, heed::BoxedError> {
        let (id_bytes, rest) = bytes
            .split_first_chunk::<RECORD_ID_SIZE>()
            .ok_or_eyre("missing record ID")?;
        let record_id = RecordId(u64::from_le_bytes(*id_bytes));
        let secondary = str::from_utf8(rest).wrap_err("invalid term secondary")?;
        Ok(Self {
            record_id,
            secondary,
        })
    }
}

/// [`backend::LookupStorage`] for [`Heed`].
#[derive(Debug)]
pub struct LookupStorage {
    records: Database<U64LE, Bytes>,
    headwords: Database<Str, TermEntry<'static>>,
    readings: Database<Str, TermEntry<'static>>,
    #[debug(skip)]
    txn: RoTxn<'static, WithoutTls>,
}

impl backend::LookupStorage for LookupStorage {
    fn lookup<D: Decoder>(
        &self,
        make_decoder: impl Fn() -> D,
        lemma: &str,
    ) -> Result<Vec<RecordRow>> {
        self.lookup_lemma_(make_decoder(), lemma).collect()
    }
}

impl LookupStorage {
    fn get_ids(
        &self,
        primary: &str,
        db: &Database<Str, TermEntry<'static>>,
        make_term: impl Fn(&str) -> Result<Term, NoHeadwordOrReading>,
    ) -> impl Iterator<Item = Result<(Term, RecordId)>> {
        match db.get_duplicates(&self.txn, primary) {
            Ok(Some(rows)) => Either::Left(rows.map(move |row| {
                let (_, entry) = row.wrap_err("failed to get single record ID")?;
                let term = make_term(entry.secondary)?;
                eyre::Ok((term, entry.record_id))
            })),
            Ok(None) => Either::Right(None),
            Err(err) => Either::Right(Some(
                Err(err).wrap_err("failed to get all record IDs for key"),
            )),
        }
        .into_iter()
    }

    fn lookup_lemma_(
        &self,
        mut decoder: impl Decoder,
        lemma: &str,
    ) -> impl Iterator<Item = Result<RecordRow>> {
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
                self.get_ids(lemma, &self.headwords, move |reading| {
                    Term::from_full(lemma, reading)
                })
                .map(|r| r.wrap_err("failed to query headwords")),
            )
            .chain(
                self.get_ids(lemma, &self.readings, move |headword| {
                    Term::from_full(headword, lemma)
                })
                .map(|r| r.wrap_err("failed to query readings")),
            );

        ids.map(move |id| {
            id.and_then(|(term, record_id)| {
                get_record(record_id).map(|record| RecordRow {
                    term,
                    record_id,
                    record,
                })
            })
        })
    }
}
