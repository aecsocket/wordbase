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
CREATE TABLE records (
    id   INTEGER PRIMARY KEY,
    data BLOB    NOT NULL
)";

const CREATE_HEADWORDS: &str = "
CREATE TABLE headwords (
    key    TEXT    PRIMARY KEY,
    record INTEGER NOT NULL REFERENCES records(id)
)";

const CREATE_READINGS: &str = "
CREATE TABLE readings (
    key    TEXT    PRIMARY KEY,
    record INTEGER NOT NULL REFERENCES records(id)
)";

const INSERT_RECORD: &str = "
INSERT INTO records (id, data)
VALUES (?, ?)";

const INSERT_HEADWORD: &str = "
INSERT INTO headwords (key, record)
VALUES (?, ?)";

const INSERT_READING: &str = "
INSERT INTO readings (key, record)
VALUES (?, ?)";

const GET_RECORDS: &str = "
SELECT id, data FROM records
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
            insert_record: &mut self.insert_record,
            insert_headword: &mut self.insert_headword,
            insert_reading: &mut self.insert_reading,
        })
    }
}

pub struct ImportTransaction<'stg> {
    txn: Transaction,
    insert_record: &'stg mut Statement,
    insert_headword: &'stg mut Statement,
    insert_reading: &'stg mut Statement,
}

impl super::ImportTransaction for ImportTransaction<'_> {
    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn open_tables(&mut self) -> Result<&mut Self> {
        Ok(self)
    }

    fn commit(self) -> Result<()> {
        block_on(self.txn.commit())?;
        Ok(())
    }
}

impl super::ImportTables for &mut ImportTransaction<'_> {
    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn batch(&self) -> &Self {
        self
    }
}

impl super::ImportBatch for &&mut ImportTransaction<'_> {
    fn insert_record(&mut self, record_id: RecordId, record: &[u8]) -> Result<()> {
        block_on(self.insert_record.execute((record_id.0, record)))
            .wrap_err("failed to insert record")?;
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
