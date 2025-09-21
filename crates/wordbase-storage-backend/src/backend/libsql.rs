//! See [`Libsql`].

use {
    derive_more::Debug,
    eyre::{Context, ContextCompat, Result, eyre},
    futures::executor::block_on,
    libsql::{Builder, Connection, Statement, Transaction, TransactionBehavior},
    std::path::Path,
    tokio::sync::{Mutex, MutexGuard},
    wordbase_api::{RecordId, Term},
    wordbase_storage_api::{
        backend::{self, RecordRow},
        codec::Decoder,
    },
};

const DATABASE_PATH: &str = "database.db";

const SETUP: &str = "
CREATE TABLE record (
    id   INTEGER PRIMARY KEY,
    data BLOB    NOT NULL
);

CREATE TABLE term (
    headword TEXT,
    reading  TEXT,
    record   INTEGER NOT NULL REFERENCES record(id)
);

CREATE INDEX term_headword ON term(headword);
CREATE INDEX term_reading ON term(reading);
";

const INSERT_RECORD: &str = "
INSERT INTO record (id, data)
VALUES (?1, ?2)";

const INSERT_TERM: &str = "
INSERT INTO term (headword, reading, record)
VALUES (?1, ?2, ?3)";

// don't use `WHERE term.headword = ? OR term.reading = ?`
// because that will not use our indexes
const GET_RECORDS: &str = "
SELECT term.headword, term.reading, record.id, record.data
FROM record
JOIN term INDEXED BY term_headword ON term.record = record.id
WHERE term.headword = ?1

UNION ALL

SELECT term.headword, term.reading, record.id, record.data
FROM record
JOIN term INDEXED BY term_reading ON term.record = record.id
WHERE term.reading = ?1";

/// [`backend::Backend`] implementation which uses the [`libsql`] wrapper around
/// [SQLite](https://sqlite.org/index.html).
#[derive(Debug)]
pub struct Libsql;

impl backend::Backend for Libsql {
    type Lookups = LookupStorage;

    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn create_import_storage(data_dir: &Path) -> Result<ImportStorage> {
        block_on(async {
            let conn = connect(data_dir).await?;
            conn.execute_batch(SETUP)
                .await
                .wrap_err("failed to setup database")?;

            Ok(ImportStorage {
                statements: Mutex::new(Statements {
                    insert_record: conn
                        .prepare(INSERT_RECORD)
                        .await
                        .wrap_err("failed to prepare insert record statement")?,
                    insert_term: conn
                        .prepare(INSERT_TERM)
                        .await
                        .wrap_err("failed to prepare insert term statement")?,
                }),
                conn,
            })
        })
    }

    fn open(data_dir: &Path) -> Result<Self::Lookups> {
        block_on(async {
            let conn = connect(data_dir).await?;
            Ok(LookupStorage {
                get_records: conn
                    .prepare(GET_RECORDS)
                    .await
                    .wrap_err("failed to prepare get records statement")?,
            })
        })
    }
}

async fn connect(data_dir: &Path) -> Result<Connection> {
    let db_path = data_dir.join(DATABASE_PATH);
    let db_path = db_path
        .to_str()
        .ok_or_else(|| eyre!("path {db_path:?} is not UTF-8"))?;
    let db = Builder::new_local(db_path)
        .build()
        .await
        .wrap_err_with(|| eyre!("failed to create database at {db_path:?}"))?;
    db.connect().wrap_err("failed to connect to database")
}

/// [`backend::ImportStorage`] for [`Libsql`].
#[derive(Debug)]
pub struct ImportStorage {
    statements: Mutex<Statements>,
    conn: Connection,
}

#[derive(Debug)]
struct Statements {
    #[debug(skip)]
    insert_record: Statement,
    #[debug(skip)]
    insert_term: Statement,
}

impl backend::ImportStorage for ImportStorage {
    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn transaction(&mut self) -> Result<ImportTransaction<'_>> {
        let txn = block_on(
            self.conn
                .transaction_with_behavior(TransactionBehavior::Exclusive),
        )
        .wrap_err("failed to start transaction")?;
        Ok(ImportTransaction {
            txn,
            statements: &self.statements,
        })
    }

    fn commit(self) -> Result<()> {
        Ok(())
    }
}

/// [`backend::ImportTransaction`] for [`Libsql`].
#[derive(Debug)]
pub struct ImportTransaction<'stg> {
    #[debug(skip)]
    txn: Transaction,
    statements: &'stg Mutex<Statements>,
}

impl<'stg> backend::ImportTransaction for ImportTransaction<'stg> {
    type Batch<'txn>
        = ImportBatch<'stg>
    where
        Self: 'txn;

    fn batch(&self) -> Result<Self::Batch<'_>> {
        Ok(ImportBatch {
            statements: self.statements.blocking_lock(),
        })
    }

    fn commit(self) -> Result<()> {
        block_on(self.txn.commit())?;
        Ok(())
    }
}

/// [`backend::ImportBatch`] for [`Libsql`].
#[derive(Debug)]
pub struct ImportBatch<'stg> {
    statements: MutexGuard<'stg, Statements>,
}

impl backend::ImportBatch for ImportBatch<'_> {
    fn insert_record(&mut self, record_id: RecordId, record: &[u8]) -> Result<()> {
        block_on(async {
            self.statements
                .insert_record
                .execute((record_id.0, record))
                .await?;
            self.statements.insert_record.reset();
            eyre::Ok(())
        })
    }

    fn insert_term(&mut self, term: &Term, record_id: RecordId) -> Result<()> {
        block_on(async {
            let headword = term.headword().map(|s| s.as_str());
            let reading = term.reading().map(|s| s.as_str());
            self.statements
                .insert_term
                .execute((headword, reading, record_id.0))
                .await?;
            self.statements.insert_term.reset();
            eyre::Ok(())
        })
    }
}

/// [`backend::LookupStorage`] for [`Libsql`].
#[derive(Debug)]
pub struct LookupStorage {
    #[debug(skip)]
    get_records: Statement,
}

impl backend::LookupStorage for LookupStorage {
    fn lookup<D: Decoder>(
        &self,
        make_decoder: impl Fn() -> D,
        lemma: &str,
    ) -> Result<Vec<RecordRow>> {
        block_on(async {
            let mut decoder = make_decoder();
            let mut records = Vec::new();

            let mut rows = self
                .get_records
                .query([lemma])
                .await
                .wrap_err("failed to get records")?;

            while let Some(row) = rows.next().await.wrap_err("failed to get row")? {
                let headword = row
                    .get::<Option<String>>(0)
                    .wrap_err("failed to get column `headword`")?;
                let reading = row
                    .get::<Option<String>>(1)
                    .wrap_err("failed to get column `reading`")?;
                let record_id = row
                    .get::<u64>(2)
                    .map(RecordId)
                    .wrap_err("failed to get column `record_id`")?;
                let record_blob = row.get_value(3).wrap_err("failed to get column `data`")?;

                let term = Term::from_parts(headword, reading)
                    .wrap_err_with(|| eyre!("failed to get record for {record_id:?}"))?;

                (|| {
                    let record_blob = record_blob
                        .as_blob()
                        .wrap_err("column `data` is not a blob")?;
                    let record = decoder
                        .decode(record_blob)
                        .wrap_err("failed to decode record")?;

                    records.push(RecordRow {
                        term: term.clone(),
                        record_id,
                        record,
                    });
                    eyre::Ok(())
                })()
                .wrap_err_with(|| eyre!("failed to get record for {term} ({record_id:?})"))?;
            }

            eyre::Ok(records)
        })
    }
}
