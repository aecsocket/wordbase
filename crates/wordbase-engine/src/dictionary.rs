use anyhow::{Context, Result};
use futures::TryStreamExt;
use tokio_stream::StreamExt;
use wordbase::{DictionaryId, DictionaryMeta, DictionaryState, protocol::NotFound};

use crate::{Engine, Event};

impl Engine {
    pub async fn dictionaries(&self) -> Result<Vec<DictionaryState>> {
        sqlx::query!(
            "SELECT id, position, meta
            FROM dictionary
            ORDER BY position"
        )
        .fetch(&self.db)
        .map(|record| {
            let record = record.context("failed to fetch record")?;
            let meta = serde_json::from_str::<DictionaryMeta>(&record.meta)
                .context("failed to deserialize dictionary meta")?;
            anyhow::Ok(DictionaryState {
                id: DictionaryId(record.id),
                position: record.position,
                meta,
            })
        })
        .try_collect()
        .await
    }

    pub async fn dictionary(&self, id: DictionaryId) -> Result<Result<DictionaryState, NotFound>> {
        let record = sqlx::query!(
            "SELECT id, position, meta FROM dictionary WHERE id = $1 LIMIT 1",
            id.0
        )
        .fetch_one(&self.db)
        .await;
        match record {
            Ok(record) => {
                let meta = serde_json::from_str::<DictionaryMeta>(&record.meta)
                    .context("failed to deserialize dictionary meta")?;
                Ok(Ok(DictionaryState {
                    id: DictionaryId(record.id),
                    position: record.position,
                    meta,
                }))
            }
            Err(sqlx::Error::RowNotFound) => Ok(Err(NotFound)),
            Err(err) => Err(anyhow::Error::new(err)),
        }
    }

    pub async fn set_dictionary_position(
        &self,
        id: DictionaryId,
        position: i64,
    ) -> Result<Result<(), NotFound>> {
        let result = sqlx::query!(
            "UPDATE dictionary
            SET position = $1
            WHERE id = $2",
            position,
            id.0
        )
        .execute(&self.db)
        .await?;
        if result.rows_affected() == 0 {
            return Ok(Err(NotFound));
        }

        self.sync_dictionaries()
            .await
            .context("failed to sync dictionaries")?;
        Ok(Ok(()))
    }

    pub async fn delete_dictionary(&self, id: DictionaryId) -> Result<Result<(), NotFound>> {
        let result = sqlx::query!("DELETE FROM dictionary WHERE id = $1", id.0)
            .execute(&self.db)
            .await?;
        if result.rows_affected() == 0 {
            return Ok(Err(NotFound));
        }

        self.sync_dictionaries()
            .await
            .context("failed to sync dictionaries")?;
        Ok(Ok(()))
    }

    pub async fn sync_dictionaries(&self) -> Result<()> {
        let dictionaries = self
            .dictionaries()
            .await
            .context("failed to fetch all dictionaries")?;
        _ = self.send_event.send(Event::SyncDictionaries(dictionaries));
        Ok(())
    }
}
