use {
    either::Either,
    eyre::{Context, Result, eyre},
    redb::{
        Database, MultimapTable, MultimapTableDefinition, ReadOnlyDatabase, ReadOnlyMultimapTable,
        ReadOnlyTable, ReadTransaction, Table, TableDefinition, WriteTransaction,
    },
    std::{
        fs, iter,
        path::PathBuf,
        sync::atomic::{self, AtomicU64},
    },
    tracing::debug,
    wordbase_api::{DictionaryId, Record, RecordId, Term},
};

const RECORDS: TableDefinition<u64, &[u8]> = TableDefinition::new("records");
const HEADWORDS: MultimapTableDefinition<&str, u64> = MultimapTableDefinition::new("headwords");
const READINGS: MultimapTableDefinition<&str, u64> = MultimapTableDefinition::new("readings");

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
        let db_path = dictionary_dir.join("database.redb");

        Ok(ImportStorage {
            db: Database::create(&db_path)
                .wrap_err_with(|| eyre!("failed to create database at {dictionary_dir:?}"))?,
            next_record_id: AtomicU64::new(0),
        })
    }
}

pub struct ImportStorage {
    db: Database,
    next_record_id: AtomicU64,
}

impl ImportStorage {
    pub fn from_db(db: Database) -> Self {
        Self {
            db,
            next_record_id: AtomicU64::new(0),
        }
    }
}

impl super::ImportStorage for ImportStorage {
    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn begin_write(&mut self) -> Result<ImportTransaction<'_>> {
        Ok(ImportTransaction {
            txn: self
                .db
                .begin_write()
                .wrap_err("failed to begin write transaction")?,
            next_record_id: &self.next_record_id,
        })
    }
}

pub struct ImportTransaction<'s> {
    txn: WriteTransaction,
    next_record_id: &'s AtomicU64,
}

impl super::ImportTransaction for ImportTransaction<'_> {
    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn open_tables(&mut self) -> Result<ImportTables<'_>> {
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
        })
    }

    fn commit(self) -> Result<()> {
        debug!("Committing");
        self.txn.commit().wrap_err("failed to commit transaction")?;
        // compacting appears to be useless

        Ok(())
    }
}

pub struct ImportTables<'t> {
    records: Table<'t, u64, &'static [u8]>,
    headwords: MultimapTable<'t, &'static str, u64>,
    readings: MultimapTable<'t, &'static str, u64>,
    next_record_id: &'t AtomicU64,
}

impl super::ImportTables for ImportTables<'_> {
    fn insert_record(&mut self, record: impl Into<Record>) -> Result<RecordId> {
        self._insert_record(record.into())
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

impl ImportTables<'_> {
    fn _insert_record(&mut self, record: Record) -> Result<RecordId> {
        let record_id = self.next_record_id.fetch_add(1, atomic::Ordering::SeqCst);

        // TODO codec
        let buf = rmp_serde::to_vec(&record).unwrap();

        self.records
            .insert(record_id, buf.as_slice())
            .wrap_err("failed to insert record")?;
        Ok(RecordId(record_id))
    }
}

pub struct Lookups {
    records: ReadOnlyTable<u64, &'static [u8]>,
    headwords: ReadOnlyMultimapTable<&'static str, u64>,
    readings: ReadOnlyMultimapTable<&'static str, u64>,
    _txn: ReadTransaction,
    _db: ReadOnlyDatabase,
}

impl super::Lookups for Lookups {
    fn lookup_lemma(&self, lemma: &str) -> impl Iterator<Item = Result<Record>> {
        match self.lookup_record_ids(lemma) {
            Ok(record_ids) => Either::Left(record_ids.filter_map(|id| {
                match self.records.get(id.0) {
                    Ok(Some(blob)) => Some(
                        // TODO codec
                        rmp_serde::from_slice::<Record>(blob.value())
                            .wrap_err_with(|| eyre!("failed to deserialize {id:?}")),
                    ),
                    Ok(None) => None,
                    Err(err) => Some(Err(err.into())),
                }
            })),
            Err(err) => Either::Right(iter::once(Err(err))),
        }
        .into_iter()
    }
}

impl Lookups {
    fn lookup_record_ids(&self, lemma: &str) -> Result<impl Iterator<Item = RecordId>> {
        let query_ids = |table: &ReadOnlyMultimapTable<_, _>, err| {
            eyre::Ok(
                table
                    .get(lemma)
                    .wrap_err(err)?
                    .filter_map(|record_id| record_id.ok().map(|id| RecordId(id.value()))),
            )
        };

        Ok(iter::empty()
            .chain(query_ids(&self.headwords, "failed to query headwords")?)
            .chain(query_ids(&self.readings, "failed to query readings")?))
    }
}
