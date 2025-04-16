use {
    crate::{Engine, IndexMap, NotFound},
    anyhow::{Context, Result, bail},
    derive_more::Deref,
    futures::TryStreamExt,
    sqlx::{Pool, Sqlite},
    std::sync::Arc,
    tokio_stream::StreamExt,
    wordbase::{Dictionary, DictionaryId, DictionaryMeta},
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

    async fn sync_dictionaries(&self) -> Result<()> {
        let dictionaries = Dictionaries::fetch(&self.db)
            .await
            .context("failed to sync dictionaries")?;
        self.dictionaries.store(Arc::new(dictionaries));
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
