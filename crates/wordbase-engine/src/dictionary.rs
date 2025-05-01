use {
    crate::{DictionaryEvent, Engine, EngineEvent, IndexMap, NotFound},
    anyhow::{Context, Result, bail},
    derive_more::Deref,
    futures::TryStreamExt,
    sqlx::{Pool, Sqlite},
    std::sync::Arc,
    tokio_stream::StreamExt,
    wordbase::{Dictionary, DictionaryId, DictionaryMeta, ProfileId},
};

#[derive(Debug, Default, Deref)]
pub struct Dictionaries(pub IndexMap<DictionaryId, Arc<Dictionary>>);

impl Dictionaries {
    pub(super) async fn fetch(db: &Pool<Sqlite>) -> Result<Self> {
        let dictionaries = fetch_owned(db)
            .await
            .context("failed to fetch dictionaries")?
            .into_iter()
            .map(|dict| (dict.id, Arc::new(dict)))
            .collect::<IndexMap<_, _>>();
        Ok(Self(dictionaries))
    }
}

impl Engine {
    #[must_use]
    pub fn dictionaries(&self) -> Arc<Dictionaries> {
        self.dictionaries.load().clone()
    }

    pub(super) async fn sync_dictionaries(&self) -> Result<()> {
        let dictionaries = Dictionaries::fetch(&self.db)
            .await
            .context("failed to sync dictionaries")?;
        self.dictionaries.store(Arc::new(dictionaries));
        Ok(())
    }

    pub async fn remove_dictionary(&self, id: DictionaryId) -> Result<()> {
        let result = sqlx::query!("DELETE FROM dictionary WHERE id = $1", id.0)
            .execute(&self.db)
            .await?;
        if result.rows_affected() == 0 {
            bail!(NotFound);
        }

        self.sync_dictionaries().await?;
        _ = self
            .send_event
            .send(EngineEvent::Dictionary(DictionaryEvent::Removed { id }));
        Ok(())
    }

    pub async fn swap_dictionary_positions(
        &self,
        a_id: DictionaryId,
        b_id: DictionaryId,
    ) -> Result<()> {
        let result = sqlx::query!(
            "WITH positions AS (
                SELECT
                    (SELECT position FROM dictionary WHERE id = $1) AS pos1,
                    (SELECT position FROM dictionary WHERE id = $2) AS pos2
            )
            UPDATE dictionary
            SET position = CASE
                WHEN id = $1 THEN (SELECT pos2 FROM positions)
                WHEN id = $2 THEN (SELECT pos1 FROM positions)
                ELSE position
            END
            WHERE id IN ($1, $2)",
            a_id.0,
            b_id.0
        )
        .execute(&self.db)
        .await?;
        if result.rows_affected() == 0 {
            bail!(NotFound);
        }

        self.sync_dictionaries().await?;
        _ = self
            .send_event
            .send(EngineEvent::Dictionary(DictionaryEvent::PositionsSwapped {
                a_id,
                b_id,
            }));
        Ok(())
    }

    pub async fn set_sorting_dictionary(
        &self,
        profile_id: ProfileId,
        dictionary_id: Option<DictionaryId>,
    ) -> Result<()> {
        let dictionary_id_raw = dictionary_id.map(|id| id.0);
        sqlx::query!(
            "UPDATE profile SET sorting_dictionary = $1
            WHERE id = $2",
            dictionary_id_raw,
            profile_id.0
        )
        .execute(&self.db)
        .await?;

        self.sync_profiles().await?;
        _ = self.send_event.send(EngineEvent::SortingDictionarySet {
            profile_id,
            dictionary_id,
        });
        Ok(())
    }

    pub async fn enable_dictionary(
        &self,
        profile_id: ProfileId,
        dictionary_id: DictionaryId,
    ) -> Result<()> {
        sqlx::query!(
            "INSERT OR IGNORE INTO profile_enabled_dictionary (profile, dictionary)
            VALUES ($1, $2)",
            profile_id.0,
            dictionary_id.0,
        )
        .execute(&self.db)
        .await?;

        self.sync_profiles().await?;
        _ = self
            .send_event
            .send(EngineEvent::Dictionary(DictionaryEvent::Enabled {
                profile_id,
                dictionary_id,
            }));
        Ok(())
    }

    pub async fn disable_dictionary(
        &self,
        profile_id: ProfileId,
        dictionary_id: DictionaryId,
    ) -> Result<()> {
        sqlx::query!(
            "DELETE FROM profile_enabled_dictionary
            WHERE profile = $1 AND dictionary = $2",
            profile_id.0,
            dictionary_id.0
        )
        .execute(&self.db)
        .await?;

        self.sync_profiles().await?;
        _ = self
            .send_event
            .send(EngineEvent::Dictionary(DictionaryEvent::Disabled {
                profile_id,
                dictionary_id,
            }));
        Ok(())
    }
}

async fn fetch_owned(db: &Pool<Sqlite>) -> Result<Vec<Dictionary>> {
    sqlx::query!(
        "SELECT id, position, meta
        FROM dictionary
        ORDER BY position"
    )
    .fetch(db)
    .map(|record| {
        let record = record.context("failed to fetch record")?;
        let meta = serde_json::from_str::<DictionaryMeta>(&record.meta)
            .context("failed to deserialize dictionary meta")?;
        anyhow::Ok(Dictionary {
            id: DictionaryId(record.id),
            position: record.position,
            meta,
        })
    })
    .try_collect()
    .await
}
