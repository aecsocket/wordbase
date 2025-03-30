mod yomichan_audio;
mod yomitan_async;
mod yomitan_sync;

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
    tracing::debug,
    wordbase::{DictionaryId, DictionaryKind, DictionaryMeta, RecordType, Term},
};

static FORMATS: LazyLock<HashMap<DictionaryKind, Arc<dyn ImportKind>>> = LazyLock::new(|| {
    [
        (
            DictionaryKind::Yomitan,
            Arc::new(yomitan_async::Yomitan) as Arc<dyn ImportKind>,
        ),
        (
            DictionaryKind::YomichanAudio,
            Arc::new(yomichan_audio::YomichanAudio),
        ),
    ]
    .into()
});

pub trait ImportKind: Send + Sync {
    fn is_of_kind(&self, archive: Bytes) -> BoxFuture<'_, Result<()>>;

    fn start_import<'a>(
        &'a self,
        engine: &'a Engine,
        archive: Bytes,
    ) -> BoxFuture<'a, Result<(ImportStarted, ImportContinue<'a>)>>;
}

pub type ImportContinue<'a> = BoxFuture<'a, Result<DictionaryId>>;

#[derive(Debug, Display, Error)]
pub enum GetKindError {
    #[display(
        "archive does not represent a valid dictionary kind\n{}",
        format_errors(_0)
    )]
    NoFormat(#[error(ignore)] HashMap<DictionaryKind, anyhow::Error>),
    #[display("archive represents multiple dictionary kinds: {_0:?}")]
    MultipleFormats(#[error(ignore)] Vec<DictionaryKind>),
}

fn format_errors(errors: &HashMap<DictionaryKind, anyhow::Error>) -> String {
    errors
        .iter()
        .map(|(format, err)| format!("does not represent a `{format:?}` dictionary: {err:?}\n"))
        .collect::<Vec<_>>()
        .join("\n")
}

pub async fn kind_of(archive: &Bytes) -> Result<DictionaryKind, GetKindError> {
    let mut valid_formats = Vec::<DictionaryKind>::new();
    let mut format_errors = HashMap::<DictionaryKind, anyhow::Error>::new();

    let format_results = FORMATS
        .iter()
        .map(|(format, importer)| async move {
            let result = importer.is_of_kind(archive.clone()).await;
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
        (None, _) => Err(GetKindError::NoFormat(format_errors)),
        (Some(format), 1) => Ok(*format),
        (_, _) => Err(GetKindError::MultipleFormats(valid_formats)),
    }
}

/// Tracks the state of a dictionary import operation.
#[derive(Debug)]
pub struct ImportStarted {
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

#[derive(Debug, Display, Error, From)]
pub enum ImportError {
    #[display("failed to determine dictionary kind")]
    GetKind(GetKindError),
    #[display("no importer for kind `{kind:?}`")]
    NoImporter { kind: DictionaryKind },
    #[display("failed to parse meta as `{kind:?}`")]
    ParseMeta {
        kind: DictionaryKind,
        source: anyhow::Error,
    },
    #[display("dictionary with this name already exists")]
    AlreadyExists,
    #[display("failed to import as `{kind:?}`")]
    Import {
        kind: DictionaryKind,
        source: anyhow::Error,
    },
    #[from]
    Other(anyhow::Error),
}

impl Engine {
    pub async fn import_dictionary(
        &self,
        archive: Bytes,
        send_tracker: oneshot::Sender<ImportStarted>,
    ) -> Result<(), ImportError> {
        debug!("Attempting to determine dictionary kind");
        let kind = kind_of(&archive).await.map_err(ImportError::GetKind)?;
        debug!("Importing as {kind:?}");

        let importer = FORMATS.get(&kind).ok_or(ImportError::NoImporter { kind })?;
        let (tracker, import) = importer
            .start_import(self, archive)
            .await
            .map_err(|source| ImportError::ParseMeta { kind, source })?;
        let meta = &tracker.meta;
        debug!(
            "Importing {:?} dictionary {:?} version {:?}",
            meta.kind, meta.name, meta.version
        );

        let already_exists = dictionary_exists_by_name(&self.db, &meta.name)
            .await
            .context("failed to fetch if this dictionary already exists")?;
        if already_exists {
            return Err(ImportError::AlreadyExists);
        }

        _ = send_tracker.send(tracker);
        let dictionary_id = import
            .await
            .map_err(|source| ImportError::Import { kind, source })?;

        self.enable_dictionary(dictionary_id)
            .await
            .context("failed to enable dictionary")?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RecordId(pub i64);

async fn insert_record<R: RecordType>(
    tx: &mut Transaction<'_, Sqlite>,
    source: DictionaryId,
    record: &R,
    scratch: &mut Vec<u8>,
) -> Result<RecordId> {
    scratch.clear();
    db::serialize(record, &mut *scratch).context("failed to serialize record")?;

    let data = &scratch[..];
    let record_id = sqlx::query!(
        "INSERT INTO record (source, kind, data)
        VALUES ($1, $2, $3)",
        source.0,
        R::KIND as u16,
        data,
    )
    .execute(&mut **tx)
    .await?
    .last_insert_rowid();
    Ok(RecordId(record_id))
}

async fn insert_term_record(
    tx: &mut Transaction<'_, Sqlite>,
    term: &Term,
    record_id: RecordId,
) -> Result<()> {
    let headword = term.headword().map(|s| s.as_str());
    let reading = term.reading().map(|s| s.as_str());
    sqlx::query!(
        "INSERT OR IGNORE INTO term_record (record, headword, reading)
        VALUES ($1, $2, $3)",
        record_id.0,
        headword,
        reading,
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
