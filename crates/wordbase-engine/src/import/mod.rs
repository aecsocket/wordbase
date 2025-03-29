mod yomichan_audio;
mod yomitan;

use {
    crate::{Engine, db},
    anyhow::{Context, Result},
    bytes::Bytes,
    derive_more::{Display, Error, From},
    futures::{StreamExt, future::BoxFuture, stream::FuturesUnordered},
    sqlx::{Pool, Sqlite, Transaction},
    std::{
        collections::HashMap,
        sync::{Arc, LazyLock},
    },
    tokio::sync::{Mutex, mpsc, oneshot},
    wordbase::{DictionaryFormat, DictionaryId, DictionaryMeta, RecordType, Term},
};

static FORMATS: LazyLock<HashMap<DictionaryFormat, Arc<dyn Importer>>> = LazyLock::new(|| {
    [
        (
            DictionaryFormat::Yomitan,
            Arc::new(yomitan::Yomitan) as Arc<dyn Importer>,
        ),
        (
            DictionaryFormat::YomichanAudio,
            Arc::new(yomichan_audio::YomichanAudio),
        ),
    ]
    .into()
});

pub trait Importer: Send + Sync {
    fn validate(&self, archive: Bytes) -> BoxFuture<'_, Result<()>>;

    fn import<'a>(
        &'a self,
        engine: &'a Engine,
        archive: Bytes,
        send_tracker: oneshot::Sender<ImportTracker>,
    ) -> BoxFuture<'a, Result<(), ImportError>>;
}

#[derive(Debug, Display, Error)]
pub enum GetFormatError {
    #[display(
        "archive does not represent a valid dictionary format\n{}",
        format_errors(_0)
    )]
    NoFormat(#[error(ignore)] HashMap<DictionaryFormat, anyhow::Error>),
    #[display("archive represents multiple dictionary formats: {_0:?}")]
    MultipleFormats(#[error(ignore)] Vec<DictionaryFormat>),
}

fn format_errors(errors: &HashMap<DictionaryFormat, anyhow::Error>) -> String {
    errors
        .iter()
        .map(|(format, err)| format!("does not represent a `{format:?}` dictionary: {err:?}\n"))
        .collect::<Vec<_>>()
        .join("\n")
}

pub async fn format_of(archive: &Bytes) -> Result<DictionaryFormat, GetFormatError> {
    let mut valid_formats = Vec::<DictionaryFormat>::new();
    let mut format_errors = HashMap::<DictionaryFormat, anyhow::Error>::new();

    let format_results = FORMATS
        .iter()
        .map(|(format, importer)| async move {
            let result = importer.validate(archive.clone()).await;
            (*format, result)
        })
        .collect::<FuturesUnordered<_>>()
        .collect::<HashMap<_, _>>()
        .await;
    for (format, result) in format_results {
        match result {
            Ok(()) => valid_formats.push(format),
            Err(err) => {
                format_errors.insert(format, err);
            }
        }
    }

    match (valid_formats.first(), valid_formats.len()) {
        (None, _) => Err(GetFormatError::NoFormat(format_errors)),
        (Some(format), 1) => Ok(*format),
        (_, _) => Err(GetFormatError::MultipleFormats(valid_formats)),
    }
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
pub(super) struct Imports {
    insert_lock: Mutex<()>,
}

impl Imports {
    pub fn new() -> Self {
        Self {
            insert_lock: Mutex::new(()),
        }
    }
}

#[derive(Debug, Display, Error)]
pub enum ImportAnyError {
    #[display("failed to get archive format")]
    GetFormat(GetFormatError),
    #[display("no importer for format `{format:?}`")]
    NoImporter { format: DictionaryFormat },
    #[display("failed to import as `{format:?}`")]
    Import {
        format: DictionaryFormat,
        source: ImportError,
    },
}

impl Engine {
    pub async fn import_dictionary(
        &self,
        archive: Bytes,
        send_tracker: oneshot::Sender<ImportTracker>,
    ) -> Result<(), ImportAnyError> {
        let format = format_of(&archive)
            .await
            .map_err(ImportAnyError::GetFormat)?;
        let importer = FORMATS
            .get(&format)
            .ok_or(ImportAnyError::NoImporter { format })?;
        importer
            .import(self, archive, send_tracker)
            .await
            .map_err(|source| ImportAnyError::Import { format, source })
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
