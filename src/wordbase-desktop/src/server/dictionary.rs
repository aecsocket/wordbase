use {
    anyhow::Context,
    bytes::Bytes,
    futures::{StreamExt, stream::BoxStream},
    poem::{Result, error::NotFoundError},
    poem_openapi::{Multipart, Object, Union, payload::EventStream, types::multipart::Upload},
    std::{io::Cursor, sync::Arc},
    wordbase::{
        Dictionary, DictionaryId, DictionaryKind, DictionaryMeta, Engine, ProfileId,
        import::{self, Archive, ImportProgress},
    },
};

pub async fn index(engine: &Engine) -> Vec<Arc<Dictionary>> {
    engine.dictionaries().values().cloned().collect()
}

pub async fn find(engine: &Engine, dictionary_id: DictionaryId) -> Result<Arc<Dictionary>> {
    Ok(engine
        .dictionaries()
        .get(&dictionary_id)
        .ok_or(NotFoundError)?
        .clone())
}

pub async fn delete(engine: &Engine, dictionary_id: DictionaryId) -> Result<()> {
    engine.remove_dictionary(dictionary_id).await?;
    Ok(())
}

pub async fn import(engine: &Engine, req: Import) -> EventStream<BoxStream<'static, ImportEvent>> {
    let import = req
        .archive
        .into_vec()
        .await
        .map(|archive| {
            let archive = Bytes::from(archive);
            engine.import_dictionary(move || {
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

pub async fn position_swap(engine: &Engine, req: PositionSwap) -> Result<()> {
    engine.swap_dictionary_positions(req.a_id, req.b_id).await?;
    Ok(())
}

#[derive(Debug, Clone, Object)]
pub struct PositionSwap {
    pub a_id: DictionaryId,
    pub b_id: DictionaryId,
}

pub async fn enable(engine: &Engine, req: ToggleEnable) -> Result<()> {
    engine
        .enable_dictionary(req.profile_id, req.dictionary_id)
        .await?;
    Ok(())
}

pub async fn disable(engine: &Engine, req: ToggleEnable) -> Result<()> {
    engine
        .disable_dictionary(req.profile_id, req.dictionary_id)
        .await?;
    Ok(())
}

#[derive(Debug, Clone, Object)]
pub struct ToggleEnable {
    pub dictionary_id: DictionaryId,
    pub profile_id: ProfileId,
}
