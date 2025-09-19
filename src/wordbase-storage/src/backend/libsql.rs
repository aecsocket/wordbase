use {
    crate::{backend::TermPart, codec::Decoder},
    derive_more::Debug,
    eyre::{Context, Result, eyre},
    futures::executor::block_on,
    libsql::{Builder, Connection, Statement, Transaction, TransactionBehavior},
    std::path::Path,
    wordbase_api::{Record, RecordId, Term},
};

const DATABASE_PATH: &str = "database.db";

const CREATE_RECORDS: &str = "
CREATE TABLE record (
    id   INTEGER PRIMARY KEY,
    data BLOB    NOT NULL
)";

const CREATE_HEADWORDS: &str = "
CREATE TABLE headword (
    text   TEXT    PRIMARY KEY,
    record INTEGER NOT NULL REFERENCES record(id)
)";

const CREATE_READINGS: &str = "
CREATE TABLE reading (
    text   TEXT    PRIMARY KEY,
    record INTEGER NOT NULL REFERENCES record(id)
)";

const INSERT_RECORD: &str = "
INSERT INTO record (id, data)
VALUES (?, ?)";

const INSERT_HEADWORD: &str = "
INSERT INTO headword (text, record)
VALUES (?, ?)";

const INSERT_READING: &str = "
INSERT INTO reading (text, record)
VALUES (?, ?)";

const GET_RECORDS: &str = "
SELECT id, data FROM record
LIMIT 1";

#[derive(Debug)]
pub struct Backend;

impl super::Backend for Backend {
    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn import(data_dir: &Path) -> Result<ImportStorage> {
        block_on(async {
            let conn = connect(data_dir).await?;

            conn.execute(CREATE_RECORDS, ())
                .await
                .wrap_err("failed to create records table")?;
            conn.execute(CREATE_HEADWORDS, ())
                .await
                .wrap_err("failed to create headwords table")?;
            conn.execute(CREATE_READINGS, ())
                .await
                .wrap_err("failed to create readings table")?;

            Ok(ImportStorage {
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
                conn,
            })
        })
    }

    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn open(data_dir: &Path) -> Result<Lookups> {
        block_on(async {
            let conn = connect(data_dir).await?;
            Ok(Lookups {
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

pub struct ImportStorage {
    insert_record: Statement,
    insert_headword: Statement,
    insert_reading: Statement,
    conn: Connection,
}

impl super::ImportStorage for ImportStorage {
    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn transaction(&mut self) -> Result<ImportTransaction<'_>> {
        let txn = block_on(
            self.conn
                .transaction_with_behavior(TransactionBehavior::Exclusive),
        )
        .wrap_err("failed to start transaction")?;
        Ok(ImportTransaction {
            txn,
            insert_record: &self.insert_record,
            insert_headword: &self.insert_headword,
            insert_reading: &self.insert_reading,
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
    #[debug(skip)]
    insert_record: &'stg Statement,
    #[debug(skip)]
    insert_headword: &'stg Statement,
    #[debug(skip)]
    insert_reading: &'stg Statement,
}

impl<'stg> super::ImportTransaction for ImportTransaction<'stg> {
    type Batch<'txn>
        = ImportBatch<'stg>
    where
        Self: 'txn;

    fn batch(&self) -> Result<Self::Batch<'_>> {
        Ok(ImportBatch {
            insert_record: self.insert_record,
            insert_headword: self.insert_headword,
            insert_reading: self.insert_reading,
        })
    }

    fn commit(self) -> Result<()> {
        block_on(self.txn.commit())?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct ImportBatch<'stg> {
    #[debug(skip)]
    insert_record: &'stg Statement,
    #[debug(skip)]
    insert_headword: &'stg Statement,
    #[debug(skip)]
    insert_reading: &'stg Statement,
}

impl super::ImportBatch for ImportBatch<'_> {
    fn insert_record(&mut self, record_id: RecordId, record: &[u8]) -> Result<()> {
        println!("inserting {record_id:?}");
        block_on(self.insert_record.execute((record_id.0, record)))?;
        println!("inserted {record_id:?}");
        Ok(())
    }

    fn insert_term(&mut self, term: &Term, record_id: RecordId) -> Result<()> {
        let insert_headword = async {
            if let Some(headword) = term.headword() {
                self.insert_headword
                    .execute((headword.as_str(), record_id.0))
                    .await
                    .wrap_err("failed to insert headword")?;
            }
            eyre::Ok(())
        };
        let insert_reading = async {
            if let Some(reading) = term.reading() {
                self.insert_reading
                    .execute((reading.as_str(), record_id.0))
                    .await
                    .wrap_err("failed to insert reading")?;
            }
            eyre::Ok(())
        };
        block_on(async move { tokio::try_join!(insert_headword, insert_reading) })?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct Lookups {
    #[debug(skip)]
    get_records: Statement,
}

impl super::Lookups for Lookups {
    fn lookup_lemma<D: Decoder>(
        &self,
        make_decoder: impl Fn() -> D,
        lemma: &str,
    ) -> Result<Vec<(TermPart, Record)>> {
        block_on(async {
            let rows = self
                .get_records
                .query(&[lemma])
                .await
                .wrap_err("failed to get records")?;
            eyre::Ok(())
        });

        Ok(vec![])
    }
}
