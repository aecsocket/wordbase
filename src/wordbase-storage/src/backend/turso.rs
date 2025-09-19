use {
    crate::{
        codec::{Codec, Encoder},
        storage::TermPart,
    },
    derive_more::Debug,
    eyre::{Context, Result, eyre},
    futures::executor::block_on,
    std::{
        path::Path,
        sync::atomic::{self, AtomicU64},
    },
    tokio::sync::Mutex,
    turso::{
        Builder, Connection, Statement,
        transaction::{Transaction, TransactionBehavior},
    },
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
                next_record_id: AtomicU64::new(0),
                codec: self.codec.clone(),
            })
        })
    }

    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn open(&self, data_dir: &Path) -> Result<Lookups<C>> {
        block_on(async {
            let conn = connect(data_dir).await?;
            Ok(Lookups {
                get_records: Mutex::new(
                    conn.prepare(GET_RECORDS)
                        .await
                        .wrap_err("failed to prepare get records statement")?,
                ),
                codec: self.codec.clone(),
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

pub struct ImportStorage<C> {
    insert_record: Statement,
    insert_headword: Statement,
    insert_reading: Statement,
    conn: Connection,
    next_record_id: AtomicU64,
    codec: C,
}

impl<C: Codec> super::ImportStorage for ImportStorage<C> {
    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn begin_write(&mut self) -> Result<ImportTransaction<'_, C::Encoder>> {
        let txn = block_on(
            // TODO what behavior is fastest?
            self.conn
                .transaction_with_behavior(TransactionBehavior::Exclusive),
        )
        .wrap_err("failed to start transaction")?;
        Ok(ImportTransaction {
            txn,
            insert_record: &mut self.insert_record,
            insert_headword: &mut self.insert_headword,
            insert_reading: &mut self.insert_reading,
            next_record_id: &self.next_record_id,
            encoder: self.codec.encoder(),
        })
    }
}

pub struct ImportTransaction<'s, E> {
    txn: Transaction<'s>,
    insert_record: &'s mut Statement,
    insert_headword: &'s mut Statement,
    insert_reading: &'s mut Statement,
    next_record_id: &'s AtomicU64,
    encoder: E,
}

impl<E: Encoder> super::ImportTransaction for ImportTransaction<'_, E> {
    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn open_tables(&mut self) -> Result<&mut Self> {
        Ok(self)
    }

    fn commit(self) -> Result<()> {
        block_on(self.txn.commit())?;
        Ok(())
    }
}

impl<E: Encoder> super::ImportTables for &mut ImportTransaction<'_, E> {
    fn insert_record(&mut self, record: impl Into<Record>) -> Result<RecordId> {
        self.insert_record_(&record.into())
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

impl<E: Encoder> ImportTransaction<'_, E> {
    fn insert_record_(&mut self, record: &Record) -> Result<RecordId> {
        let id = RecordId(self.next_record_id.fetch_add(1, atomic::Ordering::Relaxed));
        let blob = self
            .encoder
            .encode(record)
            .wrap_err("failed to encode record")?;
        block_on(self.insert_record.execute((id.0, blob.as_ref())))
            .wrap_err("failed to insert record")?;
        Ok(id)
    }
}

#[derive(Debug)]
pub struct Lookups<C> {
    #[debug(skip)]
    get_records: Mutex<Statement>,
    codec: C,
}

impl<C: Codec> super::Lookups for Lookups<C> {
    fn lookup_lemma(&self, lemma: &str) -> Result<Vec<(TermPart, Record)>> {
        block_on(async {
            let rows = {
                let mut get_records = self.get_records.lock().await;
                get_records
                    .query((lemma,))
                    .await
                    .wrap_err("failed to get records")?;
            };
            eyre::Ok(())
        });

        Ok(vec![])
    }
}
