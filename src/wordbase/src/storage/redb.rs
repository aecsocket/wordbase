use {
    crate::{
        codec::{Codec, Decoder, Encoder},
        storage::StorageKind,
    },
    derive_more::Debug,
    either::Either,
    eyre::{Context, Result, eyre},
    redb::{
        Database, MultimapTable, MultimapTableDefinition, ReadOnlyDatabase, ReadOnlyMultimapTable,
        ReadOnlyTable, ReadTransaction, ReadableDatabase, Table, TableDefinition, WriteTransaction,
    },
    std::{
        iter,
        marker::PhantomData,
        path::{Path, PathBuf},
        sync::atomic::{self, AtomicU64},
    },
    tracing::debug,
    wordbase_api::{Record, RecordId, Term},
};

const RECORDS: TableDefinition<u64, &[u8]> = TableDefinition::new("records");
const HEADWORDS: MultimapTableDefinition<&str, u64> = MultimapTableDefinition::new("headwords");
const READINGS: MultimapTableDefinition<&str, u64> = MultimapTableDefinition::new("readings");

#[derive(Debug)]
pub struct Storage<C> {
    #[debug(ignore)]
    _phantom: PhantomData<C>,
}

impl<C: Codec> Storage<C> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<C: Codec> Default for Storage<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: Codec> Clone for Storage<C> {
    fn clone(&self) -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<C: Codec> super::Storage for Storage<C> {
    type Codec = C;

    fn kind() -> StorageKind {
        StorageKind::Redb
    }

    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn begin_import(&self, data_dir: &Path) -> Result<ImportStorage<C>> {
        let db_path = data_dir.join("database.redb");
        Ok(ImportStorage {
            db: Database::create(&db_path)
                .wrap_err_with(|| eyre!("failed to create database at {data_dir:?}"))?,
            db_path,
            next_record_id: AtomicU64::new(0),
            _phantom: PhantomData,
        })
    }
}

pub struct ImportStorage<C> {
    db: Database,
    db_path: PathBuf,
    next_record_id: AtomicU64,
    _phantom: PhantomData<C>,
}

impl<C: Codec> super::ImportStorage for ImportStorage<C> {
    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn begin_write(&mut self) -> Result<ImportTransaction<'_, C>> {
        Ok(ImportTransaction {
            txn: self
                .db
                .begin_write()
                .wrap_err("failed to begin write transaction")?,
            next_record_id: &self.next_record_id,
            _phantom: PhantomData,
        })
    }

    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn open_lookups(self) -> Result<Lookups<C>> {
        drop(self.db);

        let db = ReadOnlyDatabase::open(&self.db_path)
            .wrap_err("failed to re-open database for reading")?;
        let txn = db
            .begin_read()
            .wrap_err("failed to open read transaction")?;

        Ok(Lookups {
            records: txn
                .open_table(RECORDS)
                .wrap_err("failed to open records table")?,
            headwords: txn
                .open_multimap_table(HEADWORDS)
                .wrap_err("failed to open headwords table")?,
            readings: txn
                .open_multimap_table(READINGS)
                .wrap_err("failed to open readings table")?,
            _txn: txn,
            _db: db,
            _phantom: PhantomData,
        })
    }
}

pub struct ImportTransaction<'s, C> {
    txn: WriteTransaction,
    next_record_id: &'s AtomicU64,
    _phantom: PhantomData<C>,
}

impl<C: Codec> super::ImportTransaction for ImportTransaction<'_, C> {
    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn open_tables(&mut self) -> Result<ImportTables<'_, C::Encoder>> {
        Ok(ImportTables {
            records: self
                .txn
                .open_table(RECORDS)
                .wrap_err("failed to open records table")?,
            headwords: self
                .txn
                .open_multimap_table(HEADWORDS)
                .wrap_err("failed to open headwords table")?,
            readings: self
                .txn
                .open_multimap_table(READINGS)
                .wrap_err("failed to open readings table")?,
            next_record_id: self.next_record_id,
            encoder: C::encoder(),
        })
    }

    fn commit(self) -> Result<()> {
        debug!("Committing");
        self.txn.commit().wrap_err("failed to commit transaction")?;
        // compacting appears to be useless

        Ok(())
    }
}

pub struct ImportTables<'t, E> {
    records: Table<'t, u64, &'static [u8]>,
    headwords: MultimapTable<'t, &'static str, u64>,
    readings: MultimapTable<'t, &'static str, u64>,
    next_record_id: &'t AtomicU64,
    encoder: E,
}

impl<E: Encoder> super::ImportTables for ImportTables<'_, E> {
    fn insert_record(&mut self, record: impl Into<Record>) -> Result<RecordId> {
        self.insert_record_(&record.into())
    }

    fn insert_term(&mut self, term: &Term, record_id: RecordId) -> Result<()> {
        if let Some(headword) = term.headword() {
            self.headwords
                .insert(headword.as_str(), record_id.0)
                .wrap_err("failed to insert headword")?;
        }
        if let Some(reading) = term.reading() {
            self.readings
                .insert(reading.as_str(), record_id.0)
                .wrap_err("failed to insert reading")?;
        }
        Ok(())
    }
}

impl<E: Encoder> ImportTables<'_, E> {
    fn insert_record_(&mut self, record: &Record) -> Result<RecordId> {
        let record_id = self.next_record_id.fetch_add(1, atomic::Ordering::SeqCst);
        let record_blob = self
            .encoder
            .encode(record)
            .wrap_err("failed to encode record")?;
        self.records
            .insert(record_id, record_blob.as_ref())
            .wrap_err("failed to insert record")?;
        Ok(RecordId(record_id))
    }
}

pub struct Lookups<C> {
    records: ReadOnlyTable<u64, &'static [u8]>,
    headwords: ReadOnlyMultimapTable<&'static str, u64>,
    readings: ReadOnlyMultimapTable<&'static str, u64>,
    _txn: ReadTransaction,
    _db: ReadOnlyDatabase,
    _phantom: PhantomData<C>,
}

impl<C: Codec> super::LookupStorage for Lookups<C> {
    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn lookups(&self) -> Result<&Self> {
        Ok(self)
    }
}

impl<C: Codec> super::Lookups for &Lookups<C> {
    fn lookup_lemma(&self, lemma: &str) -> Result<Vec<Record>> {
        self.lookup_lemma_(lemma).collect()
    }
}

impl<C: Codec> Lookups<C> {
    fn lookup_lemma_(&self, lemma: &str) -> impl Iterator<Item = Result<Record>> {
        let mut decoder = C::decoder();

        let get_ids = |table: &ReadOnlyMultimapTable<_, _>| {
            match table.get(lemma) {
                Ok(values) => Either::Left(values.map(|id| {
                    let id = id.wrap_err("failed to get single record ID")?;
                    eyre::Ok(RecordId(id.value()))
                })),
                Err(err) => {
                    Either::Right(Err(err).wrap_err("failed to get all record IDs for key"))
                }
            }
            .into_iter()
        };

        let mut get_record = move |id: RecordId| -> Result<Record> {
            let blob = self
                .records
                .get(id.0)
                .wrap_err_with(|| eyre!("failed to get {id:?}"))?
                .ok_or_else(|| eyre!("no record {id:?}"))?;
            let record = decoder
                .decode(blob.value())
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
