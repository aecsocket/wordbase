use {
    crate::{backend::TermPart, codec::Decoder},
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
    wordbase_api::{Record, RecordId, Term},
};

const DATABASE_PATH: &str = "database.redb";
const RECORDS: TableDefinition<u64, &[u8]> = TableDefinition::new("records");
const HEADWORDS: MultimapTableDefinition<&str, u64> = MultimapTableDefinition::new("headwords");
const READINGS: MultimapTableDefinition<&str, u64> = MultimapTableDefinition::new("readings");

#[derive(Debug)]
pub struct Backend;

impl super::Backend for Backend {
    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn import(data_dir: &Path) -> Result<ImportTransaction> {
        let db_path = data_dir.join(DATABASE_PATH);
        let db = Database::create(&db_path)
            .wrap_err_with(|| eyre!("failed to create database at {db_path:?}"))?;
        Ok(ImportTransaction {
            txn: db.begin_write().wrap_err("failed to begin write")?,
            _db: db,
        })
    }

    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn open(data_dir: &Path) -> Result<Lookups> {
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
            _txn: txn,
            _db: db,
        })
    }
}

#[derive(Debug)]
pub struct ImportTransaction {
    #[debug(skip)]
    txn: WriteTransaction,
    _db: Database,
}

impl super::ImportStorage for ImportTransaction {
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

#[derive(Debug)]
pub struct ImportTables<'txn> {
    tables: Mutex<Tables<'txn>>,
}

#[derive(Debug)]
struct Tables<'txn> {
    #[debug(skip)]
    records: Table<'txn, u64, &'static [u8]>,
    #[debug(skip)]
    headwords: MultimapTable<'txn, &'static str, u64>,
    #[debug(skip)]
    readings: MultimapTable<'txn, &'static str, u64>,
}

impl<'txn> super::ImportTransaction for ImportTables<'txn> {
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

#[derive(Debug)]
pub struct ImportBatch<'txn, 'tbl> {
    tables: MutexGuard<'tbl, Tables<'txn>>,
}

impl super::ImportBatch for ImportBatch<'_, '_> {
    fn insert_record(&mut self, record_id: RecordId, record: &[u8]) -> Result<()> {
        self.tables
            .records
            .insert(record_id.0, record)
            .wrap_err("failed to insert record")?;
        Ok(())
    }

    fn insert_term(&mut self, term: &Term, record_id: RecordId) -> Result<()> {
        if let Some(headword) = term.headword() {
            self.tables
                .headwords
                .insert(headword.as_str(), record_id.0)
                .wrap_err("failed to insert headword")?;
        }
        if let Some(reading) = term.reading() {
            self.tables
                .readings
                .insert(reading.as_str(), record_id.0)
                .wrap_err("failed to insert reading")?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Lookups {
    #[debug(skip)]
    records: ReadOnlyTable<u64, &'static [u8]>,
    #[debug(skip)]
    headwords: ReadOnlyMultimapTable<&'static str, u64>,
    #[debug(skip)]
    readings: ReadOnlyMultimapTable<&'static str, u64>,
    _txn: ReadTransaction,
    #[debug(skip)]
    _db: ReadOnlyDatabase,
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
