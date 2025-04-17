use std::sync::Arc;

use anyhow::Context;
use bytes::Bytes;
use futures::{StreamExt, stream::BoxStream};
use poem::{Result, error::NotFoundError};
use poem_openapi::{
    Multipart, Object, Union,
    payload::EventStream,
    types::{Example, multipart::Upload},
};
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use wordbase::{Dictionary, DictionaryId, DictionaryMeta, ProfileId};
use wordbase_engine::Engine;

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
    let engine = engine.clone();
    let stream = async_stream::stream! {
        let archive = match req
            .archive
            .into_vec()
            .await
        {
            Ok(archive) => Bytes::from(archive),
            Err(err) => {
                yield ImportEvent::ReadIntoMemoryErr(ImportErr {
                    error: err.to_string(),
                });
                return;
            }
        };
        yield ImportEvent::ReadIntoMemoryOk(ReadIntoMemory {});

        let (send_tracker, recv_tracker) = oneshot::channel();
        let task = tokio::spawn({
            let engine = engine.clone();
            async move {
                engine
                    .import_dictionary(archive, send_tracker)
                    .await
                    .map_err(anyhow::Error::new)
            }
        });

        if let Ok(mut tracker) = recv_tracker.await {
            yield ImportEvent::ReadMeta(tracker.meta);

            while let Some(progress) = tracker.recv_progress.recv().await {
                yield ImportEvent::Progress(Progress { progress });
            }
        };

        yield match task.await.context("import task canceled") {
            Ok(Ok(dictionary_id)) => ImportEvent::Ok(ImportOk { dictionary_id }),
            Ok(Err(err)) | Err(err) => {
                ImportEvent::Err(ImportErr {
                    error: format!("{err:?}"),
                })
            },
        };
    };
    EventStream::new(stream.boxed())
}

#[derive(Debug, Multipart)]
pub struct Import {
    pub archive: Upload,
}

#[derive(Debug, Clone, Serialize, Deserialize, Union)]
#[oai(discriminator_name = "event_kind")]
pub enum ImportEvent {
    ReadIntoMemoryOk(ReadIntoMemory),
    ReadIntoMemoryErr(ImportErr),
    ReadMeta(DictionaryMeta),
    Progress(Progress),
    Ok(ImportOk),
    Err(ImportErr),
}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct ReadIntoMemory {}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct Progress {
    pub progress: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct ImportOk {
    pub dictionary_id: DictionaryId,
}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct ImportErr {
    pub error: String,
}

pub async fn set_position(
    engine: &Engine,
    dictionary_id: DictionaryId,
    req: SetPosition,
) -> Result<()> {
    engine
        .set_dictionary_position(dictionary_id, req.position)
        .await?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct SetPosition {
    pub position: i64,
}

pub async fn enable(engine: &Engine, dictionary_id: DictionaryId, req: ToggleEnable) -> Result<()> {
    engine
        .enable_dictionary(req.profile_id, dictionary_id)
        .await?;
    Ok(())
}

pub async fn disable(
    engine: &Engine,
    dictionary_id: DictionaryId,
    req: ToggleEnable,
) -> Result<()> {
    engine
        .enable_dictionary(req.profile_id, dictionary_id)
        .await?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
#[oai(example)]
pub struct ToggleEnable {
    pub profile_id: ProfileId,
}

impl Example for ToggleEnable {
    fn example() -> Self {
        Self {
            profile_id: ProfileId(1),
        }
    }
}
