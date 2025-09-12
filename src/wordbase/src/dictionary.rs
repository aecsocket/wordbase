use {
    crate::{IndexMap, NotFound, Profiles, Wordbase},
    anyhow::{Context, Result, bail},
    arc_swap::ArcSwap,
    derive_more::Deref,
    futures::TryStreamExt,
    serde::{Deserialize, Serialize},
    sqlx::{Acquire, Pool, Sqlite},
    std::{sync::Arc, time::Instant},
    tokio_stream::StreamExt,
    tracing::info,
    wordbase_api::{Dictionary, DictionaryId, DictionaryMeta, ProfileId},
};

#[derive(Debug, Default, Deref, Serialize, Deserialize)]
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

    pub(super) async fn sync(
        db: &Pool<Sqlite>,
        dictionaries: &ArcSwap<Dictionaries>,
    ) -> Result<()> {
        let fetched = Dictionaries::fetch(db)
            .await
            .context("failed to sync dictionaries")?;
        dictionaries.store(Arc::new(fetched));
        Ok(())
    }
}

impl Wordbase {
    #[must_use]
    pub fn dictionaries(&self) -> Arc<Dictionaries> {
        self.dictionaries.load().clone()
    }

    pub async fn remove_dictionary(&self, id: DictionaryId) -> Result<()> {
        let mut conn = self.db.acquire().await?;

        // FK constraints are slow to uphold when deleting in bulk like this
        // so we disable them (for this connection only) to do a bulk delete
        // it's now on us to uphold the constraints, but we're good programmers :)
        sqlx::query!("PRAGMA foreign_keys = OFF")
            .execute(&mut *conn)
            .await
            .context("failed to disable foreign keys")?;

        let mut tx = conn.begin().await.context("failed to begin transaction")?;

        let start = Instant::now();

        info!("Deleting term records for {id:?}");
        sqlx::query!("DELETE FROM term_record WHERE source = $1", id.0)
            .execute(&mut *tx)
            .await
            .context("failed to delete term records")?;

        info!("Deleting records");
        sqlx::query!("DELETE FROM record WHERE source = $1", id.0)
            .execute(&mut *tx)
            .await
            .context("failed to delete records")?;

        info!("Deleting frequency records");
        sqlx::query!("DELETE FROM frequency WHERE source = $1", id.0)
            .execute(&mut *tx)
            .await
            .context("failed to delete frequency rows")?;

        info!("Deleting dictionary record");
        let result = sqlx::query!("DELETE FROM dictionary WHERE id = $1", id.0)
            .execute(&mut *tx)
            .await
            .context("failed to delete dictionary row")?;
        if result.rows_affected() == 0 {
            bail!(NotFound);
        }

        info!("Committing");
        tx.commit().await.context("failed to commit transaction")?;

        info!("Vacuuming");
        sqlx::query!("VACUUM")
            .execute(&self.db)
            .await
            .context("failed to vacuum")?;

        let end = Instant::now();
        info!("Finished delete in {:?}", end.duration_since(start));

        Dictionaries::sync(&self.db, &self.dictionaries).await?;
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

        Dictionaries::sync(&self.db, &self.dictionaries).await?;
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

        Profiles::sync(&self.db, &self.profiles).await?;
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

        Profiles::sync(&self.db, &self.profiles).await?;
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

        Profiles::sync(&self.db, &self.profiles).await?;
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

#[cfg(feature = "uniffi")]
const _: () = {
    use {
        crate::{FfiResult, Wordbase},
        std::collections::HashMap,
    };

    #[uniffi::export(async_runtime = "tokio")]
    impl Wordbase {
        #[uniffi::method(name = "dictionaries")]
        pub fn ffi_dictionaries(&self) -> HashMap<DictionaryId, Dictionary> {
            self.dictionaries()
                .iter()
                .map(|(id, dict)| (*id, (**dict).clone()))
                .collect()
        }

        #[uniffi::method(name = "remove_dictionary")]
        pub async fn ffi_remove_dictionary(&self, id: DictionaryId) -> FfiResult<()> {
            Ok(self.remove_dictionary(id).await?)
        }

        #[uniffi::method(name = "swap_dictionary_positions")]
        pub async fn ffi_swap_dictionary_positions(
            &self,
            a_id: DictionaryId,
            b_id: DictionaryId,
        ) -> FfiResult<()> {
            Ok(self.swap_dictionary_positions(a_id, b_id).await?)
        }

        #[uniffi::method(name = "set_sorting_dictionary")]
        pub async fn ffi_set_sorting_dictionary(
            &self,
            profile_id: ProfileId,
            dictionary_id: Option<DictionaryId>,
        ) -> FfiResult<()> {
            Ok(self
                .set_sorting_dictionary(profile_id, dictionary_id)
                .await?)
        }

        #[uniffi::method(name = "enable_dictionary")]
        pub async fn ffi_enable_dictionary(
            &self,
            profile_id: ProfileId,
            dictionary_id: DictionaryId,
        ) -> FfiResult<()> {
            Ok(self.enable_dictionary(profile_id, dictionary_id).await?)
        }

        #[uniffi::method(name = "disable_dictionary")]
        pub async fn ffi_disable_dictionary(
            &self,
            profile_id: ProfileId,
            dictionary_id: DictionaryId,
        ) -> FfiResult<()> {
            Ok(self.disable_dictionary(profile_id, dictionary_id).await?)
        }
    }
};
