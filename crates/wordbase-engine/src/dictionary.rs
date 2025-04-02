use {
    crate::Engine,
    anyhow::{Context, Result, bail},
    derive_more::{Display, Error},
    futures::TryStreamExt,
    tokio_stream::StreamExt,
    wordbase::{Dictionary, DictionaryId, DictionaryMeta},
};

impl Engine {
    pub async fn dictionaries(&self) -> Result<Vec<Dictionary>> {
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
        .fetch(&self.db)
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

    pub async fn dictionary(&self, id: DictionaryId) -> Result<Dictionary> {
        let record = sqlx::query!(
            r#"SELECT
                dictionary.id,
                dictionary.position,
                dictionary.meta,
                ped.dictionary IS NOT NULL AS "enabled!: bool"
            FROM dictionary
            LEFT JOIN profile_enabled_dictionary ped
                ON dictionary.id = ped.dictionary
                AND ped.profile = (SELECT current_profile FROM config)
            WHERE id = $1
            LIMIT 1"#,
            id.0
        )
        .fetch_one(&self.db)
        .await?;
        let meta = serde_json::from_str::<DictionaryMeta>(&record.meta)
            .context("failed to deserialize dictionary meta")?;
        Ok(Dictionary {
            id: DictionaryId(record.id),
            position: record.position,
            enabled: record.enabled,
            meta,
        })
    }

    pub async fn enable_dictionary(&self, id: DictionaryId) -> Result<()> {
        sqlx::query!(
            "INSERT OR IGNORE INTO profile_enabled_dictionary (profile, dictionary)
            VALUES ((SELECT current_profile FROM config), $1)",
            id.0,
        )
        .execute(&self.db)
        .await?;
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
        Ok(())
    }

    pub async fn remove_dictionary(&self, id: DictionaryId) -> Result<()> {
        let result = sqlx::query!("DELETE FROM dictionary WHERE id = $1", id.0)
            .execute(&self.db)
            .await?;
        if result.rows_affected() == 0 {
            bail!(NotFound);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Display, Error)]
#[display("dictionary not found")]
pub struct NotFound;
