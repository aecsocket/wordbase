//! See [`Rocksdb`].

use {
    derive_more::Debug,
    either::Either,
    eyre::{Context, Result, eyre},
    rocksdb::{
        ColumnFamily, DB, DBPinnableSlice, Env, Options, WaitForCompactOptions, WriteOptions,
    },
    std::{iter, path::Path, sync::LazyLock},
    wordbase_api::{Record, RecordId, Term, TermPart},
    wordbase_storage_api::{
        backend::{self, RecordRow},
        codec::Decoder,
    },
};

const RECORDS: &str = "records";
const HEADWORDS: &str = "headwords";
const READINGS: &str = "readings";

#[derive(Debug)]
pub struct Rocksdb;

impl backend::Backend for Rocksdb {
    type Lookups = LookupStorage;

    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn create_import_storage(data_dir: &Path) -> Result<ImportStorage> {
        let env = Env::new().wrap_err("failed to create database env")?;

        let options = db_open_options(&env);
        let mut db = DB::open(&options, data_dir)
            .wrap_err_with(|| eyre!("failed to create database at {data_dir:?}"))?;
        db.create_cf(RECORDS, &options)
            .wrap_err_with(|| eyre!("failed to create column family `{RECORDS}`"))?;

        let options = term_cf_options(&env);
        db.create_cf(HEADWORDS, &options)
            .wrap_err_with(|| eyre!("failed to create column family `{HEADWORDS}`"))?;
        db.create_cf(READINGS, &options)
            .wrap_err_with(|| eyre!("failed to create column family `{RECORDS}`"))?;

        Ok(ImportStorage { db })
    }

    fn open(data_dir: &Path) -> Result<Self::Lookups> {
        let env = Env::new().wrap_err("failed to create database env")?;
        let column_families = [
            (RECORDS, db_open_options(&env)),
            (HEADWORDS, term_cf_options(&env)),
            (READINGS, term_cf_options(&env)),
        ];
        let db = DB::open_cf_with_opts_for_read_only(
            &db_open_options(&env),
            data_dir,
            column_families,
            false, // error_if_log_file_exist
        )
        .wrap_err("failed to open database")?;
        Ok(LookupStorage { db })
    }
}

fn db_open_options(env: &Env) -> Options {
    let mut options = Options::default();
    options.set_env(env);
    options.create_if_missing(true);
    // <https://github.com/facebook/rocksdb/wiki/RocksDB-FAQ>
    // "What's the fastest way to load data into RocksDB?"
    options.prepare_for_bulk_load();
    options
}

fn term_cf_options(env: &Env) -> Options {
    let mut options = db_open_options(env);
    options.set_merge_operator_associative("concat", |_new_key, existing_val, operands| {
        let mut result = Vec::with_capacity(operands.len());
        if let Some(val) = existing_val {
            result.extend_from_slice(val);
        }
        for op in operands {
            result.extend_from_slice(op);
        }
        Some(result)
    });
    options
}

#[derive(Debug)]
pub struct ImportStorage {
    db: DB,
}

impl backend::ImportStorage for ImportStorage {
    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn transaction(&mut self) -> Result<ImportTransaction<'_>> {
        Ok(ImportTransaction { db: &self.db })
    }

    fn commit(self) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct ImportTransaction<'stg> {
    db: &'stg DB,
}

impl<'stg> backend::ImportTransaction for ImportTransaction<'stg> {
    type Batch<'txn>
        = ImportBatch<'stg>
    where
        Self: 'txn;

    fn batch(&self) -> Result<Self::Batch<'_>> {
        Ok(ImportBatch {
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
        })
    }

    fn commit(self) -> Result<()> {
        self.db
            .flush()
            .wrap_err("failed to flush memtables to SST files")?;
        // <https://github.com/facebook/rocksdb/wiki/RocksDB-FAQ>
        // "What's the fastest way to load data into RocksDB?"
        self.db.compact_range(None::<&[u8]>, None::<&[u8]>);
        self.db
            .wait_for_compact(&WaitForCompactOptions::default())
            .wrap_err("failed to wait for compaction jobs")?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct ImportBatch<'stg> {
    #[debug(skip)]
    records: &'stg ColumnFamily,
    #[debug(skip)]
    headwords: &'stg ColumnFamily,
    #[debug(skip)]
    readings: &'stg ColumnFamily,
    db: &'stg DB,
}

impl backend::ImportBatch for ImportBatch<'_> {
    fn insert_record(&mut self, record_id: RecordId, record: &[u8]) -> Result<()> {
        put(self.db, self.records, &id_to_bytes(record_id), record)
            .wrap_err("failed to insert record")?;
        Ok(())
    }

    fn insert_term(&mut self, term: &Term, record_id: RecordId) -> Result<()> {
        if let Some(headword) = term.headword() {
            merge(
                self.db,
                self.headwords,
                headword.as_bytes(),
                &id_to_bytes(record_id),
            )
            .wrap_err("failed to insert headword")?;
        }
        if let Some(reading) = term.reading() {
            merge(
                self.db,
                self.readings,
                reading.as_bytes(),
                &id_to_bytes(record_id),
            )
            .wrap_err("failed to insert reading")?;
        }
        Ok(())
    }
}

static WRITE_OPTIONS: LazyLock<WriteOptions> = LazyLock::new(|| {
    let mut opts = WriteOptions::new();
    opts.disable_wal(true);
    opts
});

fn put(db: &DB, cf: &ColumnFamily, key: &[u8], value: &[u8]) -> Result<()> {
    db.put_cf_opt(cf, key, value, &WRITE_OPTIONS)?;
    Ok(())
}

fn merge(db: &DB, cf: &ColumnFamily, key: &[u8], value: &[u8]) -> Result<()> {
    db.merge_cf_opt(cf, key, value, &WRITE_OPTIONS)?;
    Ok(())
}

#[derive(Debug)]
pub struct LookupStorage {
    db: DB,
}

impl backend::LookupStorage for LookupStorage {
    fn lookup<D: Decoder>(
        &self,
        make_decoder: impl Fn() -> D,
        lemma: &str,
    ) -> Result<Vec<RecordRow>> {
        let records = get_cf(&self.db, RECORDS)?;
        let headwords = get_cf(&self.db, HEADWORDS)?;
        let readings = get_cf(&self.db, READINGS)?;

        let headword_ids_blob = self
            .db
            .get_pinned_cf(headwords, lemma.as_bytes())
            .wrap_err("failed to get record IDs for headword")?;
        let reading_ids_blob = self
            .db
            .get_pinned_cf(readings, lemma.as_bytes())
            .wrap_err("failed to get record IDs for reading")?;

        let records = iter::empty()
            .chain(
                headword_ids_blob
                    .iter()
                    .map(|blob| (TermPart::Headword, blob)),
            )
            .chain(
                reading_ids_blob
                    .iter()
                    .map(|blob| (TermPart::Reading, blob)),
            )
            .flat_map(move |(term_part, blob)| {
                self.get_by_ids(make_decoder(), records, blob)
                    .map(move |r| {
                        r.map(|(record_id, record)| RecordRow {
                            term_part,
                            record_id,
                            record,
                        })
                        .wrap_err_with(|| eyre!("failed to query {term_part:?}"))
                    })
            });

        records.collect::<Result<Vec<_>, _>>()
    }
}

impl LookupStorage {
    fn get_by_ids<'db, 'blob: 'db>(
        &'db self,
        mut decoder: impl Decoder,
        records: &'db ColumnFamily,
        ids_blob: &'blob DBPinnableSlice<'db>,
    ) -> impl Iterator<Item = Result<(RecordId, Record)>> + 'db {
        let get_record = move |id: RecordId| {
            let blob = self
                .db
                .get_pinned_cf(records, id_to_bytes(id))
                .wrap_err_with(|| eyre!("failed to get {id:?}"))?
                .ok_or_else(|| eyre!("no record {id:?}"))?;
            let record = decoder
                .decode(&blob)
                .wrap_err_with(|| eyre!("failed to decode {id:?}"))?;
            Ok((id, record))
        };

        match ids_blob.as_chunks::<{ size_of::<RecordId>() }>() {
            (ids_chunks, []) => Either::Left({
                let ids = ids_chunks.iter().map(|chunk| bytes_to_id(*chunk));
                ids.map(get_record)
            }),
            _ => Either::Right(iter::once(Err(eyre!(
                "value is of length {}, which is not a multiple of {}",
                ids_blob.len(),
                size_of::<RecordId>()
            )))),
        }
        .into_iter()
    }
}

fn get_cf<'db>(db: &'db DB, name: &str) -> Result<&'db ColumnFamily> {
    db.cf_handle(name)
        .ok_or_else(|| eyre!("no column family `{name}"))
}

fn id_to_bytes(record_id: RecordId) -> [u8; size_of::<u64>()] {
    record_id.0.to_le_bytes()
}

fn bytes_to_id(bytes: [u8; size_of::<u64>()]) -> RecordId {
    RecordId(u64::from_le_bytes(bytes))
}
