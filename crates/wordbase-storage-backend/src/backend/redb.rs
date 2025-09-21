//! See [`Redb`].

use {
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
        sync::{Mutex, MutexGuard},
    },
    tracing::debug,
    wordbase_api::{NoHeadwordOrReading, Record, RecordId, Term},
    wordbase_storage_api::{
        backend::{self, RecordRow},
        codec::Decoder,
    },
};

const DATABASE_PATH: &str = "database.redb";
const RECORDS: TableDefinition<u64, &[u8]> = TableDefinition::new("records");
const HEADWORDS: MultimapTableDefinition<&str, (Option<&str>, u64)> =
    MultimapTableDefinition::new("headwords");
const READINGS: MultimapTableDefinition<&str, (Option<&str>, u64)> =
    MultimapTableDefinition::new("readings");

/// [`backend::Backend`] implementation which uses [`redb`] written in pure
/// Rust.
#[derive(Debug)]
pub struct Redb;

impl backend::Backend for Redb {
    type Lookups = LookupStorage;

    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn create_import_storage(data_dir: &Path) -> Result<ImportTransaction> {
        let db_path = data_dir.join(DATABASE_PATH);
        let db = Database::create(&db_path)
            .wrap_err_with(|| eyre!("failed to create database at {db_path:?}"))?;
        Ok(ImportTransaction {
            txn: db.begin_write().wrap_err("failed to begin write")?,
            _db: db,
        })
    }

    fn open(data_dir: &Path) -> Result<Self::Lookups> {
        let db_path = data_dir.join(DATABASE_PATH);
        let db = ReadOnlyDatabase::open(&db_path).wrap_err("failed to open database")?;
        let txn = db
            .begin_read()
            .wrap_err("failed to begin read transaction")?;

        Ok(LookupStorage {
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
        })
    }
}

/// [`backend::ImportTransaction`] for [`Redb`].
#[derive(Debug)]
pub struct ImportTransaction {
    #[debug(skip)]
    txn: WriteTransaction,
    _db: Database,
}

impl backend::ImportStorage for ImportTransaction {
    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn transaction(&mut self) -> Result<ImportTables<'_>> {
        Ok(ImportTables {
            tables: Mutex::new(Tables {
                records: self
                    .txn
                    .open_table(RECORDS)
                    .wrap_err("failed to create records table")?,
                headwords: self
                    .txn
                    .open_multimap_table(HEADWORDS)
                    .wrap_err("failed to create headwords table")?,
                readings: self
                    .txn
                    .open_multimap_table(READINGS)
                    .wrap_err("failed to create readings table")?,
            }),
        })
    }

    fn commit(self) -> Result<()> {
        self.txn.commit()?;
        Ok(())
    }
}

/// [`backend::ImportTables`] for [`Redb`].
#[derive(Debug)]
pub struct ImportTables<'txn> {
    tables: Mutex<Tables<'txn>>,
}

#[derive(Debug)]
struct Tables<'txn> {
    #[debug(skip)]
    records: Table<'txn, u64, &'static [u8]>,
    #[debug(skip)]
    headwords: MultimapTable<'txn, &'static str, (Option<&'static str>, u64)>,
    #[debug(skip)]
    readings: MultimapTable<'txn, &'static str, (Option<&'static str>, u64)>,
}

impl<'txn> backend::ImportTransaction for ImportTables<'txn> {
    type Batch<'tbl>
        = ImportBatch<'txn, 'tbl>
    where
        Self: 'tbl;

    fn batch(&self) -> Result<Self::Batch<'_>> {
        Ok(ImportBatch {
            tables: self.tables.lock().expect("tables poisoned"),
        })
    }

    fn commit(self) -> Result<()> {
        Ok(())
    }
}

/// [`backend::ImportBatch`] for [`Redb`].
#[derive(Debug)]
pub struct ImportBatch<'txn, 'tbl> {
    tables: MutexGuard<'tbl, Tables<'txn>>,
}

impl backend::ImportBatch for ImportBatch<'_, '_> {
    fn insert_record(&mut self, record_id: RecordId, record: &[u8]) -> Result<()> {
        self.tables
            .records
            .insert(record_id.0, record)
            .wrap_err("failed to insert record")?;
        Ok(())
    }

    fn insert_term(&mut self, term: &Term, record_id: RecordId) -> Result<()> {
        let headword = term.headword().map(|s| s.as_str());
        let reading = term.reading().map(|s| s.as_str());

        if let Some(headword) = headword {
            self.tables
                .headwords
                .insert(headword, (reading, record_id.0))
                .wrap_err("failed to insert headword")?;
        }
        if let Some(reading) = reading {
            self.tables
                .readings
                .insert(reading, (headword, record_id.0))
                .wrap_err("failed to insert reading")?;
        }
        Ok(())
    }
}

/// [`backend::LookupStorage`] for [`Redb`].
#[derive(Debug)]
pub struct LookupStorage {
    #[debug(skip)]
    records: ReadOnlyTable<u64, &'static [u8]>,
    #[debug(skip)]
    headwords: ReadOnlyMultimapTable<&'static str, (Option<&'static str>, u64)>,
    #[debug(skip)]
    readings: ReadOnlyMultimapTable<&'static str, (Option<&'static str>, u64)>,
    _txn: ReadTransaction,
    #[debug(skip)]
    _db: ReadOnlyDatabase,
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
        term_primary: &str,
        table: &ReadOnlyMultimapTable<&str, (Option<&str>, u64)>,
        make_term: impl Fn(Option<&str>) -> Result<Term, NoHeadwordOrReading>,
    ) -> impl Iterator<Item = Result<(Term, RecordId)>> {
        match table.get(term_primary) {
            Ok(rows) => Either::Left(rows.map(move |row| {
                let row = row.wrap_err("failed to get single record ID")?;
                let (term_secondary, id) = row.value();
                let term = make_term(term_secondary)?;
                eyre::Ok((term, RecordId(id)))
            })),
            Err(err) => Either::Right(Err(err).wrap_err("failed to get all record IDs for key")),
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
                self.get_ids(lemma, &self.headwords, move |reading| {
                    Term::from_parts(Some(lemma), reading)
                })
                .map(|r| r.wrap_err("failed to query headwords")),
            )
            .chain(
                self.get_ids(lemma, &self.readings, move |headword| {
                    Term::from_parts(headword, Some(lemma))
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
