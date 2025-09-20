//! See [`Libsql`].

use {
    derive_more::Debug,
    eyre::{Context, ContextCompat, Result, bail, eyre},
    futures::executor::block_on,
    libsql::{Builder, Connection, Statement, Transaction, TransactionBehavior},
    std::path::Path,
    tokio::sync::{Mutex, MutexGuard},
    wordbase_api::{RecordId, Term, TermPart},
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

CREATE TABLE headword (
    text   TEXT    NOT NULL,
    record INTEGER NOT NULL REFERENCES record(id)
);

CREATE TABLE reading (
    text   TEXT    NOT NULL,
    record INTEGER NOT NULL REFERENCES record(id)
);

CREATE INDEX headword_text ON headword(text);
CREATE INDEX reading_text ON reading(text);
";

const INSERT_RECORD: &str = "
INSERT INTO record (id, data)
VALUES (?1, ?2)";

const INSERT_HEADWORD: &str = "
INSERT INTO headword (text, record)
VALUES (?1, ?2)";

const INSERT_READING: &str = "
INSERT INTO reading (text, record)
VALUES (?1, ?2)";

// don't use `WHERE headword.text = ? OR reading.text = ?`
// because that will not use our indexes
const GET_RECORDS: &str = "
SELECT record.id, record.data, 0 as part
FROM record
JOIN headword INDEXED BY headword_text ON record.id = headword.record
WHERE headword.text = ?

UNION ALL

SELECT record.id, record.data, 1 as part
FROM record
JOIN reading INDEXED BY reading_text ON record.id = reading.record
WHERE reading.text = ?";

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
                    insert_headword: conn
                        .prepare(INSERT_HEADWORD)
                        .await
                        .wrap_err("failed to prepare insert headword statement")?,
                    insert_reading: conn
                        .prepare(INSERT_READING)
                        .await
                        .wrap_err("failed to prepare insert reading statement")?,
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
    insert_headword: Statement,
    #[debug(skip)]
    insert_reading: Statement,
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
        let insert_headword = async {
            if let Some(headword) = term.headword() {
                self.statements
                    .insert_headword
                    .execute((headword.as_str(), record_id.0))
                    .await
                    .wrap_err("failed to insert headword")?;
                self.statements.insert_headword.reset();
            }
            eyre::Ok(())
        };
        let insert_reading = async {
            if let Some(reading) = term.reading() {
                self.statements
                    .insert_reading
                    .execute((reading.as_str(), record_id.0))
                    .await
                    .wrap_err("failed to insert reading")?;
                self.statements.insert_reading.reset();
            }
            eyre::Ok(())
        };
        block_on(async move { tokio::try_join!(insert_headword, insert_reading) })?;
        Ok(())
    }
}

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
                let record_id = RecordId(row.get::<u64>(0).wrap_err("failed to get column `id`")?);

                (|| {
                    let record_blob = row.get_value(1).wrap_err("failed to get column `data`")?;
                    let record_blob = record_blob
                        .as_blob()
                        .wrap_err("column `data` is not a blob")?;
                    let record = decoder
                        .decode(record_blob)
                        .wrap_err("failed to decode record")?;

                    let part = row.get::<u32>(2).wrap_err("failed to get column `part`")?;
                    let term_part = match part {
                        0 => TermPart::Headword,
                        1 => TermPart::Reading,
                        _ => bail!("invalid term part `{part}`"),
                    };

                    records.push(RecordRow {
                        term_part,
                        record_id,
                        record,
                    });
                    eyre::Ok(())
                })()
                .wrap_err_with(|| eyre!("failed to get {record_id:?}"))?;
            }

            eyre::Ok(records)
        })
    }
}
