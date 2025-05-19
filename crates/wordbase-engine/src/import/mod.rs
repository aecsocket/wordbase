mod yomichan_audio;
mod yomitan;

use {
    crate::{CHANNEL_BUF_CAP, DictionaryEvent, Engine, EngineEvent, db},
    anyhow::{Context, Result},
    bytes::Bytes,
    derive_more::{Display, Error, From},
    futures::{Stream, StreamExt, future::BoxFuture, stream::FuturesUnordered},
    sqlx::{Pool, Sqlite, Transaction},
    std::{
        collections::HashMap,
        sync::{Arc, LazyLock},
    },
    tokio::sync::{Mutex, mpsc},
    tracing::debug,
    wordbase::{
        DictionaryId, DictionaryKind, DictionaryMeta, FrequencyValue, RecordId, RecordType, Term,
    },
};

static FORMATS: LazyLock<HashMap<DictionaryKind, Arc<dyn ImportKind>>> = LazyLock::new(|| {
    [
        (
            DictionaryKind::Yomitan,
            Arc::new(yomitan::Yomitan) as Arc<dyn ImportKind>,
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
        send_progress: mpsc::Sender<f64>,
    ) -> BoxFuture<'a, Result<(DictionaryMeta, ImportContinue<'a>)>>;
}

pub type ImportContinue<'a> = BoxFuture<'a, Result<DictionaryId>>;

#[derive(Debug)]
pub enum ImportEvent {
    DeterminedKind(DictionaryKind),
    ParsedMeta(DictionaryMeta),
    Progress(f64),
    Done(DictionaryId),
}

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
    pub fn import_dictionary(
        &self,
        archive: Bytes,
    ) -> impl Stream<Item = Result<ImportEvent, ImportError>> {
        async_stream::try_stream! {
            debug!("Attempting to determine dictionary kind");
            let kind = kind_of(&archive).await.map_err(ImportError::GetKind)?;
            debug!("Importing as {kind:?} dictionary");
            yield ImportEvent::DeterminedKind(kind);

            let importer = FORMATS.get(&kind).ok_or(ImportError::NoImporter { kind })?;
            let (send_progress, mut recv_progress) = mpsc::channel(CHANNEL_BUF_CAP);

            let (meta, continue_task) = importer
                .start_import(self, archive, send_progress)
                .await
                .map_err(|source| ImportError::ParseMeta { kind, source })?;
            debug!(
                "Importing {:?} dictionary {:?} version {:?}",
                meta.kind, meta.name, meta.version
            );

            let already_exists = dictionary_exists_by_name(&self.db, &meta.name)
                .await
                .context("failed to fetch if this dictionary already exists")?;
            if already_exists {
                Err(ImportError::AlreadyExists)?;
                return;
            }
            yield ImportEvent::ParsedMeta(meta);

            while let Some(progress) = recv_progress.recv().await {
                yield ImportEvent::Progress(progress);
            }

            let id = continue_task
                .await
                .map_err(|source| ImportError::Import { kind, source })?;

            self.sync_dictionaries().await?;
            _ = self
                .send_event
                .send(EngineEvent::Dictionary(DictionaryEvent::Added { id }));
        }
    }
}

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
    source: DictionaryId,
    record_id: RecordId,
    term: &Term,
) -> Result<()> {
    let headword = term.headword().map(|s| &**s);
    let reading = term.reading().map(|s| &**s);
    sqlx::query!(
        "INSERT OR IGNORE INTO term_record (source, record, headword, reading)
        VALUES ($1, $2, $3, $4)",
        source.0,
        record_id.0,
        headword,
        reading,
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn insert_frequency(
    tx: &mut Transaction<'_, Sqlite>,
    source: DictionaryId,
    term: &Term,
    frequency: FrequencyValue,
) -> Result<()> {
    let headword = term.headword().map(|s| &**s);
    let reading = term.reading().map(|s| &**s);
    let (mode, value) = match frequency {
        FrequencyValue::Rank(n) => (0, n),
        FrequencyValue::Occurrence(n) => (1, n),
    };
    sqlx::query!(
        "INSERT OR IGNORE INTO frequency (source, headword, reading, mode, value)
        VALUES ($1, $2, $3, $4, $5)",
        source.0,
        headword,
        reading,
        mode,
        value,
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
        "INSERT INTO dictionary (meta, position)
        VALUES ($1, (SELECT COALESCE(MAX(position), 0) + 1 FROM dictionary))",
        meta
    )
    .execute(&mut **tx)
    .await?
    .last_insert_rowid();
    Ok(DictionaryId(new_id))
}
