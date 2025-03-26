mod yomichan_audio;
mod yomitan;

use {
    crate::{Engine, db},
    anyhow::{Context, Result},
    bytes::Bytes,
    derive_more::{Display, Error, From},
    futures::{FutureExt, StreamExt, future::BoxFuture, stream::FuturesUnordered},
    sqlx::{Pool, Sqlite, Transaction},
    std::{
        convert::Infallible,
        io::Cursor,
        sync::{Arc, LazyLock},
    },
    tokio::sync::{Mutex, mpsc, oneshot},
    wordbase::{DictionaryFormat, DictionaryId, DictionaryMeta, RecordType, Term},
};

const IMPORT_FORMATS: LazyLock<Vec<Arc<dyn ImportFormat>>> = LazyLock::new(|| {
    vec![
        Arc::new(yomitan::Yomitan),
        Arc::new(yomichan_audio::YomichanAudio),
    ]
});

pub trait ImportFormat {
    fn dictionary_format(&self) -> DictionaryFormat;

    fn validate(&self, archive: Bytes) -> BoxFuture<Result<()>>;

    fn import(
        &self,
        engine: &Engine,
        archive: Bytes,
        send_tracker: oneshot::Sender<ImportTracker>,
    ) -> BoxFuture<Result<(), ImportError>>;
}

pub async fn format_of(archive: Bytes) -> Option<DictionaryFormat> {
    let formats = IMPORT_FORMATS;
    let mut tasks = formats
        .iter()
        .map(|format| {
            format
                .validate(archive.clone())
                .map(move |result| (format, result))
        })
        .collect::<FuturesUnordered<_>>();
    while let Some((format, result)) = tasks.next().await {
        if result.is_ok() {
            return Some(format.dictionary_format());
        }
    }
    None
}

/// Failed to import a dictionary.
#[derive(Debug, Display, Error, From)]
pub enum ImportError {
    /// Dictionary with this name already exists.
    #[display("already exists")]
    AlreadyExists,
    /// Dictionary was parsed, but it had no records to insert into the
    /// database.
    #[display("no records to insert")]
    NoRecords,
    /// Implementation-specific error.
    Other(#[from] anyhow::Error),
}

/// Tracks the state of a dictionary import operation.
#[derive(Debug)]
pub struct ImportTracker {
    /// Parsed dictionary meta.
    pub meta: DictionaryMeta,
    pub recv_progress: mpsc::Receiver<f64>,
}

#[derive(Debug)]
pub(super) struct Importer {
    insert_lock: Mutex<()>,
}

impl Importer {
    pub fn new() -> Self {
        Self {
            insert_lock: Mutex::new(()),
        }
    }
}

impl Engine {
    pub async fn import_dictionary(
        &self,
        data: &[u8],
        send_tracker: oneshot::Sender<ImportTracker>,
    ) -> Result<(), ImportError> {
        self.import_dictionary_yomitan(|| Ok::<_, Infallible>(Cursor::new(data)), send_tracker)
            .await
    }
}

async fn insert_term<R: RecordType>(
    tx: &mut Transaction<'_, Sqlite>,
    source: DictionaryId,
    term: &Term,
    record: &R,
    scratch: &mut Vec<u8>,
) -> Result<()> {
    scratch.clear();
    db::serialize(record, &mut *scratch).context("failed to serialize record")?;

    let headword = term.headword();
    let reading = term.reading();
    let data = &scratch[..];
    sqlx::query!(
        "INSERT INTO term (source, headword, reading, kind, data)
        VALUES ($1, $2, $3, $4, $5)",
        source.0,
        headword,
        reading,
        R::KIND as u16,
        data
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn dictionary_exists_by_name(db: &Pool<Sqlite>, name: &str) -> Result<bool> {
    let result = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM dictionary WHERE json_extract(meta, '$.name') = $1)",
        name
    )
    .fetch_one(db)
    .await?;
    Ok(result > 0)
}

async fn insert_dictionary(
    tx: &mut Transaction<'_, Sqlite>,
    meta: &DictionaryMeta,
) -> Result<DictionaryId> {
    let meta = serde_json::to_string(meta).context("failed to serialize dictionary meta")?;
    let new_id = sqlx::query!(
        "INSERT INTO dictionary (position, meta)
        VALUES ((SELECT COALESCE(MAX(position), 0) + 1 FROM dictionary), $1)",
        meta
    )
    .execute(&mut **tx)
    .await?
    .last_insert_rowid();
    Ok(DictionaryId(new_id))
}
