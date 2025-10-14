use {
    anyhow::Context,
    bytes::Bytes,
    futures::{StreamExt, stream::BoxStream},
    std::{io::Cursor, sync::Arc},
};

pub async fn index(app: &App) -> Vec<Arc<Dictionary>> {
    app.engine.dictionaries().values().cloned().collect()
}

pub async fn find(app: &App, dictionary_id: DictionaryId) -> Result<Arc<Dictionary>> {
    Ok(app
        .engine
        .dictionaries()
        .get(&dictionary_id)
        .ok_or(NotFoundError)?
        .clone())
}

pub async fn delete(app: &App, dictionary_id: DictionaryId) -> Result<()> {
    app.engine.remove_dictionary(dictionary_id).await?;
    Ok(())
}

pub async fn import(app: &App, req: Import) -> EventStream<BoxStream<'static, ImportEvent>> {
    let import = req
        .archive
        .into_vec()
        .await
        .map(|archive| {
            let archive = Bytes::from(archive);
            app.engine.import_dictionary(move || {
                let archive = Cursor::new(archive.clone());
                async move { anyhow::Ok(Box::new(archive) as Box<dyn Archive>) }
            })
        })
        .context("failed to read into memory");
    let stream = async_stream::stream! {        let import = match import {
            Ok(import) => import,
            Err(err) => {
                yield ImportEvent::Err(ImportErr {
                    error: format!("{err:?}"),
                });
                return;
            }
        };
        yield ImportEvent::ReadIntoMemory(ReadIntoMemory {});

        for await event in import {
            yield match event {
                Ok(import::ImportEvent::DeterminedKind(kind)) => {
                    ImportEvent::DeterminedKind(DeterminedKind { kind })
                }
                Ok(import::ImportEvent::ParsedMeta(meta)) => {
                    ImportEvent::ParsedMeta(meta)
                }
                Ok(import::ImportEvent::Progress(progress)) => {
                    ImportEvent::Progress(progress)
                }
                Ok(import::ImportEvent::Done(dictionary_id)) => {
                    ImportEvent::Done(ImportDone { dictionary_id })
                }
                Err(err) => {
                    ImportEvent::Err(ImportErr { error: format!("{err:?}") })
                }
            }
        }
    };
    EventStream::new(stream.boxed())
}

#[derive(Debug, Multipart)]
pub struct Import {
    pub archive: Upload,
}

#[derive(Debug, Clone, Union)]
#[oai(discriminator_name = "event_kind")]
pub enum ImportEvent {
    ReadIntoMemory(ReadIntoMemory),
    DeterminedKind(DeterminedKind),
    ParsedMeta(DictionaryMeta),
    Progress(ImportProgress),
    Done(ImportDone),
    Err(ImportErr),
}

#[derive(Debug, Clone, Object)]
pub struct ReadIntoMemory {}

#[derive(Debug, Clone, Object)]
pub struct DeterminedKind {
    pub kind: DictionaryKind,
}

#[derive(Debug, Clone, Object)]
pub struct ImportDone {
    pub dictionary_id: DictionaryId,
}

#[derive(Debug, Clone, Object)]
pub struct ImportErr {
    pub error: String,
}

pub async fn position_swap(app: &App, req: PositionSwap) -> Result<()> {
    app.engine
        .swap_dictionary_positions(req.a_id, req.b_id)
        .await?;
    Ok(())
}

#[derive(Debug, Clone, Object)]
pub struct PositionSwap {
    pub a_id: DictionaryId,
    pub b_id: DictionaryId,
}

pub async fn enable(app: &App, req: ToggleEnable) -> Result<()> {
    app.engine
        .enable_dictionary(req.profile_id, req.dictionary_id)
        .await?;
    Ok(())
}

pub async fn disable(app: &App, req: ToggleEnable) -> Result<()> {
    app.engine
        .disable_dictionary(req.profile_id, req.dictionary_id)
        .await?;
    Ok(())
}

#[derive(Debug, Clone, Object)]
pub struct ToggleEnable {
    pub dictionary_id: DictionaryId,
    pub profile_id: ProfileId,
}
