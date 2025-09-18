use {
    crate::{
        codec::{Codec, Decoder, Encoder},
        storage::TermPart,
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
        path::Path,
        sync::atomic::{self, AtomicU64},
    },
    tracing::debug,
    wordbase_api::{Record, RecordId, Term},
};

const DATABASE_PATH: &str = "database.redb";
const RECORDS: TableDefinition<u64, &[u8]> = TableDefinition::new("records");
const HEADWORDS: MultimapTableDefinition<&str, u64> = MultimapTableDefinition::new("headwords");
const READINGS: MultimapTableDefinition<&str, u64> = MultimapTableDefinition::new("readings");

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
        let db_path = data_dir.join(DATABASE_PATH);
        Ok(ImportStorage {
            db: Database::create(&db_path)
                .wrap_err_with(|| eyre!("failed to create database at {db_path:?}"))?,
            next_record_id: AtomicU64::new(0),
            codec: self.codec.clone(),
        })
    }

    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn open(&self, data_dir: &Path) -> Result<Lookups<C>> {
        let db_path = data_dir.join(DATABASE_PATH);
        let db = ReadOnlyDatabase::open(&db_path).wrap_err("failed to open database")?;
        let txn = db
            .begin_read()
            .wrap_err("failed to begin read transaction")?;

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
            codec: self.codec.clone(),
            _txn: txn,
            _db: db,
        })
    }
}

pub struct ImportStorage<C> {
    db: Database,
    next_record_id: AtomicU64,
    codec: C,
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
            codec: self.codec.clone(),
        })
    }
}

pub struct ImportTransaction<'s, C> {
    txn: WriteTransaction,
    next_record_id: &'s AtomicU64,
    codec: C,
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
            encoder: self.codec.encoder(),
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
        let id = self.next_record_id.fetch_add(1, atomic::Ordering::SeqCst);
        let blob = self
            .encoder
            .encode(record)
            .wrap_err("failed to encode record")?;
        self.records
            .insert(id, blob.as_ref())
            .wrap_err("failed to insert record")?;
        Ok(RecordId(id))
    }
}

#[derive(Debug)]
pub struct Lookups<C> {
    #[debug(skip)]
    records: ReadOnlyTable<u64, &'static [u8]>,
    #[debug(skip)]
    headwords: ReadOnlyMultimapTable<&'static str, u64>,
    #[debug(skip)]
    readings: ReadOnlyMultimapTable<&'static str, u64>,
    codec: C,
    _txn: ReadTransaction,
    #[debug(skip)]
    _db: ReadOnlyDatabase,
}

impl<C: Codec> super::Lookups for Lookups<C> {
    fn lookup_lemma(&self, lemma: &str) -> Result<Vec<(TermPart, Record)>> {
        self.lookup_lemma_(lemma).collect()
    }
}

impl<C: Codec> Lookups<C> {
    fn lookup_lemma_(&self, lemma: &str) -> impl Iterator<Item = Result<(TermPart, Record)>> {
        let mut decoder = self.codec.decoder();

        let get_ids = |part: TermPart, table: &ReadOnlyMultimapTable<_, _>| {
            match table.get(lemma) {
                Ok(values) => Either::Left(values.map(move |id| {
                    let id = id.wrap_err("failed to get single record ID")?;
                    eyre::Ok((part, RecordId(id.value())))
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
