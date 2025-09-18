use {
    crate::codec::{Codec, Encoder},
    eyre::{Context, Result, eyre},
    rusqlite::{Connection, Transaction},
    std::{
        path::Path,
        sync::{
            Mutex, RwLock,
            atomic::{self, AtomicU64},
        },
    },
    wordbase_api::{Record, RecordId, Term},
};

const DATABASE_PATH: &str = "database.db";

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
        let conn = Connection::open(&db_path)
            .wrap_err_with(|| eyre!("failed to open connection to database at {db_path:?}"))?;

        conn.execute(
            "CREATE TABLE records (
                id   INTEGER PRIMARY KEY,
                data BLOB
            )",
            (),
        )
        .wrap_err("failed to create records table")?;

        conn.execute(
            "CREATE TABLE headwords (
                key TEXT    PRIMARY KEY,
                id  INTEGER NOT NULL REFERENCES records(id)
            )",
            (),
        )
        .wrap_err("failed to create headwords table")?;

        conn.execute(
            "CREATE TABLE readings (
                key TEXT    PRIMARY KEY,
                id  INTEGER NOT NULL REFERENCES records(id)
            )",
            (),
        )
        .wrap_err("failed to create readings table")?;

        Ok(ImportStorage {
            conn,
            next_record_id: AtomicU64::new(0),
            codec: self.codec.clone(),
        })
    }

    fn open(&self, data_dir: &Path) -> Result<impl super::Lookups> {
        todo!();
    }
}

pub struct ImportStorage<C> {
    conn: Connection,
    next_record_id: AtomicU64,
    codec: C,
}

impl<C: Codec> super::ImportStorage for ImportStorage<C> {
    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn begin_write(&mut self) -> Result<ImportTransaction<'_, C>> {
        let txn = self
            .conn
            .transaction()
            .wrap_err("failed to begin transaction")?;

        Ok(ImportTransaction {
            txn,
            next_record_id: &self.next_record_id,
            codec: self.codec.clone(),
        })
    }
}

pub struct ImportTransaction<'s, C> {
    txn: Mutex<Transaction<'s>>,
    next_record_id: &'s AtomicU64,
    codec: C,
}

impl<'s, C: Codec> super::ImportTransaction for ImportTransaction<'s, C> {
    fn open_tables(&mut self) -> Result<ImportTables<'s, '_, C::Encoder>> {
        Ok(ImportTables {
            txn: Mutex::new(self.txn),
            next_record_id: &self.next_record_id,
            encoder: self.codec.encoder(),
        })
    }

    fn commit(self) -> Result<()> {
        self.txn.commit()?;
        Ok(())
    }
}

pub struct ImportTables<'s, 't, E> {
    txn: &'t Mutex<Transaction<'s>>,
    next_record_id: &'s AtomicU64,
    encoder: E,
}

impl<E: Encoder> super::ImportTables for ImportTables<'_, '_, E> {
    fn insert_record(&mut self, record: impl Into<Record>) -> Result<RecordId> {
        self.insert_record_(&record.into())
    }

    fn insert_term(&mut self, term: &Term, record_id: RecordId) -> Result<()> {
        let mut txn = self.txn.lock().expect("mutex poisoned");
        if let Some(headword) = term.headword() {
            txn.execute(
                "INSERT INTO headwords (key, id)
                VALUES (?, ?)",
                (headword.as_str(), record_id.0),
            )
            .wrap_err("failed to insert headword")?;
        }
        if let Some(reading) = term.reading() {
            self.txn
                .execute(
                    "INSERT INTO readings (key, id)
                    VALUES (?, ?)",
                    (reading.as_str(), record_id.0),
                )
                .wrap_err("failed to insert reading")?;
        }
        Ok(())
    }
}

impl<E: Encoder> ImportTables<'_, '_, E> {
    fn insert_record_(&mut self, record: &Record) -> Result<RecordId> {
        let id = RecordId(self.next_record_id.fetch_add(1, atomic::Ordering::SeqCst));
        let blob = self
            .encoder
            .encode(record)
            .wrap_err("failed to encode record")?;
        self.txn
            .execute(
                "INSERT INTO records (id, data)
                VALUES (?, ?)",
                (id.0, blob.as_ref()),
            )
            .wrap_err("failed to insert record")?;
        Ok(id)
    }
}
