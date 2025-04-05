use {
    crate::Engine,
    anyhow::{Context, Result, bail},
    arc_swap::ArcSwap,
    derive_more::{Display, Error},
    foldhash::HashMap,
    futures::TryStreamExt,
    sqlx::{Pool, Sqlite},
    std::sync::Arc,
    tokio_stream::StreamExt,
    wordbase::{Dictionary, DictionaryId, DictionaryMeta},
};

pub type SharedDictionaries = Arc<ArcSwap<Dictionaries>>;

#[derive(Debug)]
pub struct Dictionaries {
    pub by_id: HashMap<DictionaryId, Dictionary>,
}

impl Dictionaries {
    pub(super) async fn fetch(db: &Pool<Sqlite>) -> Result<Self> {
        let by_id = fetch_owned(db)
            .await
            .context("failed to fetch dictionaries")?
            .into_iter()
            .map(|dict| (dict.id, dict))
            .collect();
        Ok(Self { by_id })
    }
}

impl Engine {
    async fn sync_dictionaries(&self) -> Result<()> {
        self.dictionaries.store(Arc::new(
            Dictionaries::fetch(&self.db)
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
