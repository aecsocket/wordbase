use {
    crate::{Engine, IndexMap, profile::Profiles},
    anyhow::{Context, Result, bail},
    derive_more::{Display, Error},
    futures::TryStreamExt,
    sqlx::{Pool, Sqlite},
    std::sync::Arc,
    tokio_stream::StreamExt,
    wordbase::{Dictionary, DictionaryId, DictionaryMeta},
};

#[derive(Debug, Default)]
pub struct Dictionaries {
    pub by_id: IndexMap<DictionaryId, Dictionary>,
    pub sorting_id: Option<DictionaryId>,
}

impl Dictionaries {
    pub(super) async fn fetch(db: &Pool<Sqlite>, profiles: &Profiles) -> Result<Self> {
        let by_id = fetch_owned(db)
            .await
            .context("failed to fetch dictionaries")?
            .into_iter()
            .map(|dict| (dict.id, dict))
            .collect();
        let sorting_id = profiles
            .by_id
            .get(&profiles.current_id)
            .and_then(|profile| profile.sorting_dictionary);
        Ok(Self { by_id, sorting_id })
    }
}

impl Engine {
    #[must_use]
    pub fn dictionaries(&self) -> Arc<Dictionaries> {
        self.dictionaries.load().clone()
    }

    async fn sync_dictionaries(&self) -> Result<()> {
        self.dictionaries.store(Arc::new(
            Dictionaries::fetch(&self.db, &self.profiles.load())
                .await
                .context("failed to sync dictionaries")?,
        ));
        Ok(())
    }

    pub async fn enable_dictionary(&self, id: DictionaryId) -> Result<()> {
        sqlx::query!(
            "INSERT OR IGNORE INTO profile_enabled_dictionary (profile, dictionary)
            VALUES ((SELECT current_profile FROM config), $1)",
            id.0,
        )
        .execute(&self.db)
        .await?;

        self.sync_dictionaries().await?;
        Ok(())
    }

    pub async fn disable_dictionary(&self, id: DictionaryId) -> Result<()> {
        sqlx::query!(
            "DELETE FROM profile_enabled_dictionary
            WHERE
                profile = (SELECT current_profile FROM config)
                AND dictionary = $1",
            id.0
        )
        .execute(&self.db)
        .await?;

        self.sync_dictionaries().await?;
        Ok(())
    }

    pub async fn set_dictionary_position(&self, id: DictionaryId, position: i64) -> Result<()> {
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
            bail!(NotFound);
        }

        self.sync_dictionaries().await?;
        Ok(())
    }

    pub async fn set_sorting_dictionary(&self, id: Option<DictionaryId>) -> Result<()> {
        let id = id.map(|id| id.0);
        sqlx::query!(
            "UPDATE profile SET sorting_dictionary = $1
            WHERE id = (SELECT current_profile FROM config)",
            id,
        )
        .execute(&self.db)
        .await?;

        self.sync_dictionaries().await?;
        self.sync_profiles().await?;
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
        Ok(())
    }
}

async fn fetch_owned(db: &Pool<Sqlite>) -> Result<Vec<Dictionary>> {
    sqlx::query!(
        r#"SELECT
                dictionary.id,
                dictionary.position,
                dictionary.meta,
                ped.dictionary IS NOT NULL AS "enabled!: bool"
            FROM dictionary
            LEFT JOIN profile_enabled_dictionary ped
                ON dictionary.id = ped.dictionary
                AND ped.profile = (SELECT current_profile FROM config)
            ORDER BY position"#
    )
    .fetch(db)
    .map(|record| {
        let record = record.context("failed to fetch record")?;
        let meta = serde_json::from_str::<DictionaryMeta>(&record.meta)
            .context("failed to deserialize dictionary meta")?;
        anyhow::Ok(Dictionary {
            id: DictionaryId(record.id),
            position: record.position,
            enabled: record.enabled,
            meta,
        })
    })
    .try_collect()
    .await
}

#[derive(Debug, Clone, Display, Error)]
#[display("dictionary not found")]
pub struct NotFound;
