use {
    crate::codec::{Codec, Decoder, Encoder},
    either::Either,
    eyre::{Context, Result, eyre},
    rocksdb::{
        ColumnFamily, DB, DBPinnableSlice, Env, Options, WaitForCompactOptions, WriteOptions,
    },
    std::{
        fs, iter,
        marker::PhantomData,
        path::PathBuf,
        sync::{
            LazyLock,
            atomic::{self, AtomicU64},
        },
    },
    wordbase_api::{DictionaryId, Record, RecordId, Term},
};

const RECORDS: &str = "records";
const HEADWORDS: &str = "headwords";
const READINGS: &str = "readings";
const COLUMN_FAMILIES: &[&str] = &[RECORDS, HEADWORDS, READINGS];

pub struct Storage<C> {
    env: Env,
    dictionaries_dir: PathBuf,
    _phantom: PhantomData<C>,
}

impl<C: Codec> Storage<C> {
    pub fn new(dictionaries_dir: impl Into<PathBuf>) -> Result<Self> {
        Ok(Self {
            env: Env::new().wrap_err("failed to create env")?,
            dictionaries_dir: dictionaries_dir.into(),
            _phantom: PhantomData,
        })
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

        let options = db_open_options(&self.env);
        let mut db = DB::open(&options, &db_path)
            .wrap_err_with(|| eyre!("failed to create database at {db_path:?}"))?;
        db.create_cf(RECORDS, &options)
            .wrap_err_with(|| eyre!("failed to create column family `{RECORDS}`"))?;

        let options = record_id_cf_open_options(&self.env);
        db.create_cf(HEADWORDS, &options)
            .wrap_err_with(|| eyre!("failed to create column family `{HEADWORDS}`"))?;
        db.create_cf(READINGS, &options)
            .wrap_err_with(|| eyre!("failed to create column family `{RECORDS}`"))?;

        Ok(ImportStorage {
            env: self.env.clone(),
            db,
            db_path,
            next_record_id: AtomicU64::new(0),
            _phantom: PhantomData,
        })
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

fn record_id_cf_open_options(env: &Env) -> Options {
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

pub struct ImportStorage<C> {
    env: Env,
    db: DB,
    db_path: PathBuf,
    next_record_id: AtomicU64,
    _phantom: PhantomData<C>,
}

impl<C: Codec> super::ImportStorage for ImportStorage<C> {
    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn begin_write(&mut self) -> Result<ImportTransaction<'_, C>> {
        Ok(ImportTransaction {
            db: &self.db,
            next_record_id: &self.next_record_id,
            _phantom: PhantomData,
        })
    }

    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn open_lookups(self) -> Result<LookupStorage<C>> {
        self.db
            .flush()
            .wrap_err("failed to flush memtables to SST files")?;
        // <https://github.com/facebook/rocksdb/wiki/RocksDB-FAQ>
        // "What's the fastest way to load data into RocksDB?"
        self.db.compact_range(None::<&[u8]>, None::<&[u8]>);
        self.db
            .wait_for_compact(&WaitForCompactOptions::default())
            .wrap_err("failed to wait for compaction jobs")?;
        drop(self.db);

        let db = DB::open_cf_for_read_only(
            &db_open_options(&self.env),
            &self.db_path,
            COLUMN_FAMILIES,
            false, // error_if_log_file_exist
        )
        .wrap_err_with(|| eyre!("failed to re-open database for reading"))?;

        Ok(LookupStorage {
            db,
            _phantom: PhantomData,
        })
    }
}

pub struct ImportTransaction<'s, C> {
    db: &'s DB,
    next_record_id: &'s AtomicU64,
    _phantom: PhantomData<C>,
}

impl<C: Codec> super::ImportTransaction for ImportTransaction<'_, C> {
    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn open_tables(&mut self) -> Result<ImportTables<'_, C::Encoder>> {
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
            next_record_id: self.next_record_id,
            encoder: C::encoder(),
        })
    }

    fn commit(self) -> Result<()> {
        Ok(())
    }
}

pub struct ImportTables<'s, E> {
    records: &'s ColumnFamily,
    headwords: &'s ColumnFamily,
    readings: &'s ColumnFamily,
    db: &'s DB,
    next_record_id: &'s AtomicU64,
    encoder: E,
}

impl<E: Encoder> super::ImportTables for ImportTables<'_, E> {
    fn insert_record(&mut self, record: impl Into<Record>) -> Result<RecordId> {
        self.insert_record_(&record.into())
    }

    fn insert_term(&mut self, term: &Term, record_id: RecordId) -> Result<()> {
        if let Some(headword) = term.headword() {
            put(
                self.db,
                self.headwords,
                headword.as_bytes(),
                &id_to_bytes(record_id),
            )
            .wrap_err("failed to insert headword")?;
        }
        if let Some(reading) = term.reading() {
            put(
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

impl<E: Encoder> ImportTables<'_, E> {
    fn insert_record_(&mut self, record: &Record) -> Result<RecordId> {
        let record_id = RecordId(self.next_record_id.fetch_add(1, atomic::Ordering::SeqCst));
        let blob = self
            .encoder
            .encode(record)
            .wrap_err("failed to encode record")?;
        put(
            self.db,
            self.records,
            &id_to_bytes(record_id),
            blob.as_ref(),
        )
        .wrap_err("failed to insert record")?;
        Ok(record_id)
    }
}

fn put(db: &DB, cf: &ColumnFamily, key: &[u8], value: &[u8]) -> Result<()> {
    static WRITE_OPTIONS: LazyLock<WriteOptions> = LazyLock::new(|| {
        let mut opts = WriteOptions::new();
        opts.disable_wal(true);
        opts
    });

    db.put_cf_opt(cf, key, value, &WRITE_OPTIONS)?;
    Ok(())
}

pub struct LookupStorage<C> {
    db: DB,
    _phantom: PhantomData<C>,
}

impl<C: Codec> super::LookupStorage for LookupStorage<C> {
    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn lookups(&self) -> Result<Lookups<'_, C>> {
        Ok(Lookups {
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
            db: &self.db,
            _phantom: PhantomData,
        })
    }
}

pub struct Lookups<'s, C> {
    records: &'s ColumnFamily,
    headwords: &'s ColumnFamily,
    readings: &'s ColumnFamily,
    db: &'s DB,
    _phantom: PhantomData<C>,
}

impl<C: Codec> super::Lookups for Lookups<'_, C> {
    fn lookup_lemma(&self, lemma: &str) -> Result<Vec<Record>> {
        let headword_ids_blob = self
            .db
            .get_pinned_cf(&self.headwords, lemma.as_bytes())
            .wrap_err("failed to get record IDs for headword")?;
        let reading_ids_blob = self
            .db
            .get_pinned_cf(&self.readings, lemma.as_bytes())
            .wrap_err("failed to get record IDs for reading")?;

        let records = iter::empty()
            .chain(
                headword_ids_blob
                    .iter()
                    .flat_map(|ids_blob| self.get_by_ids(ids_blob))
                    .map(|r| r.wrap_err("failed to query headwords")),
            )
            .chain(
                reading_ids_blob
                    .iter()
                    .flat_map(|ids_blob| self.get_by_ids(ids_blob))
                    .map(|r| r.wrap_err("failed to query readings")),
            );

        records.collect::<Result<Vec<_>, _>>()
    }
}

impl<C: Codec> Lookups<'_, C> {
    fn get_by_ids<'db, 'blob: 'db>(
        &'db self,
        ids_blob: &'blob DBPinnableSlice<'db>,
    ) -> impl Iterator<Item = Result<Record>> + 'db {
        let mut decoder = C::decoder();

        let get_record = move |id: RecordId| {
            let blob = self
                .db
                .get_pinned_cf(&self.records, id_to_bytes(id))
                .wrap_err_with(|| eyre!("failed to get {id:?}"))?
                .ok_or_else(|| eyre!("no record {id:?}"))?;
            let record = decoder
                .decode(&blob)
                .wrap_err_with(|| eyre!("failed to decode {id:?}"))?;
            Ok(record)
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

fn id_to_bytes(record_id: RecordId) -> [u8; size_of::<u64>()] {
    record_id.0.to_le_bytes()
}

fn bytes_to_id(bytes: [u8; size_of::<u64>()]) -> RecordId {
    RecordId(u64::from_le_bytes(bytes))
}
