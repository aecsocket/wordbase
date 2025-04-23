use {
    crate::{Engine, EngineEvent, IndexMap, NotFound},
    anyhow::{Context, Result, bail},
    derive_more::Deref,
    futures::StreamExt,
    sqlx::{Pool, Sqlite},
    std::sync::Arc,
    wordbase::{DictionaryId, NormString, Profile, ProfileConfig, ProfileId},
};

#[derive(Debug, Default, Deref)]
pub struct Profiles(pub IndexMap<ProfileId, Arc<Profile>>);

impl Profiles {
    pub(super) async fn fetch(db: &Pool<Sqlite>) -> Result<Self> {
        let profiles = fetch_owned(db)
            .await
            .context("failed to fetch profiles")?
            .into_iter()
            .map(|profile| (profile.id, Arc::new(profile)))
            .collect::<IndexMap<_, _>>();
        Ok(Self(profiles))
    }
}

impl Engine {
    #[must_use]
    pub fn profiles(&self) -> Arc<Profiles> {
        self.profiles.load().clone()
    }

    pub(super) async fn sync_profiles(&self) -> Result<()> {
        let profiles = Profiles::fetch(&self.db)
            .await
            .context("failed to sync profiles")?;
        self.profiles.store(Arc::new(profiles));
        Ok(())
    }

    pub async fn add_profile(&self, config: ProfileConfig) -> Result<ProfileId> {
        let config_json = serde_json::to_string(&config).context("failed to serialize config")?;
        let sorting_dictionary = config.sorting_dictionary.map(|id| id.0);

        let id = sqlx::query!(
            "INSERT INTO profile (config, sorting_dictionary)
            VALUES ($1, $2)",
            config_json,
            sorting_dictionary,
        )
        .execute(&self.db)
        .await
        .context("failed to insert profile")?
        .last_insert_rowid();
        let id = ProfileId(id);

        self.sync_profiles().await?;
        _ = self.send_event.send(EngineEvent::ProfileAdded { id });
        Ok(id)
    }

    pub async fn copy_profile(
        &self,
        src_id: ProfileId,
        config: ProfileConfig,
    ) -> Result<ProfileId> {
        let src = self.profiles().get(&src_id).cloned().context(NotFound)?;
        let mut new_config = src.config.clone();
        new_config.merge_from(config);
        let config_json =
            serde_json::to_string(&new_config).context("failed to serialize config")?;
        let sorting_dictionary = new_config.sorting_dictionary.map(|id| id.0);

        let mut tx = self
            .db
            .begin()
            .await
            .context("failed to begin transaction")?;
        let new_id = sqlx::query!(
            "INSERT INTO profile (config, sorting_dictionary)
            VALUES ($1, $2)",
            config_json,
            sorting_dictionary,
        )
        .execute(&mut *tx)
        .await
        .context("failed to insert profile")?
        .last_insert_rowid();
        let new_id = ProfileId(new_id);

        sqlx::query!(
            "INSERT INTO profile_enabled_dictionary (profile, dictionary)
            SELECT $1, dictionary
            FROM profile_enabled_dictionary
            WHERE profile = $2",
            new_id.0,
            src_id.0,
        )
        .execute(&mut *tx)
        .await
        .context("failed to copy enabled dictionaries")?;
        tx.commit().await.context("failed to commit transaction")?;

        self.sync_profiles().await?;
        _ = self
            .send_event
            .send(EngineEvent::ProfileCopied { src_id, new_id });
        Ok(new_id)
    }

    pub async fn remove_profile(&self, id: ProfileId) -> Result<()> {
        let result = sqlx::query!("DELETE FROM profile WHERE id = $1", id.0)
            .execute(&self.db)
            .await?;
        if result.rows_affected() == 0 {
            bail!(NotFound);
        }

        self.sync_profiles().await?;
        _ = self.send_event.send(EngineEvent::ProfileRemoved { id });
        Ok(())
    }

    pub async fn set_profile_name(&self, id: ProfileId, name: Option<NormString>) -> Result<()> {
        let mut config = self.profiles().get(&id).context(NotFound)?.config.clone();
        config.name = name;

        let config_json = serde_json::to_string(&config).context("failed to serialize config")?;
        sqlx::query!(
            "UPDATE profile SET config = $1 WHERE id = $2",
            config_json,
            id.0
        )
        .execute(&self.db)
        .await?;

        self.sync_profiles().await?;
        _ = self.send_event.send(EngineEvent::ProfileNameSet { id });
        Ok(())
    }

    #[deprecated]
    pub async fn set_profile_config(&self, id: ProfileId, config: ProfileConfig) -> Result<()> {
        let config_json = serde_json::to_string(&config).context("failed to serialize config")?;
        sqlx::query!(
            "UPDATE profile SET config = $1 WHERE id = $2",
            config_json,
            id.0
        )
        .execute(&self.db)
        .await?;

        self.sync_profiles().await?;
        Ok(())
    }
}

async fn fetch_owned(db: &Pool<Sqlite>) -> Result<Vec<Profile>> {
    let mut profiles = Vec::<Profile>::new();

    let mut records = sqlx::query!(
        "SELECT
            profile.id,
            profile.config,
            profile.sorting_dictionary,
            ped.dictionary
        FROM profile
        LEFT JOIN profile_enabled_dictionary ped ON profile.id = ped.profile
        ORDER BY profile.id"
    )
    .fetch(db);
    while let Some(record) = records.next().await {
        let record = record.context("failed to fetch record")?;
        let id = ProfileId(record.id);

        let profile_index =
            if let Some(index) = profiles.iter_mut().position(|profile| profile.id == id) {
                index
            } else {
                let index = profiles.len();
                let mut config = serde_json::from_str::<ProfileConfig>(&record.config)
                    .context("failed to deserialize profile config")?;
                config.sorting_dictionary = record.sorting_dictionary.map(DictionaryId);
                profiles.push(Profile::new(id, config));
                index
            };

        if let Some(dictionary) = record.dictionary {
            profiles[profile_index]
                .enabled_dictionaries
                .push(DictionaryId(dictionary));
        }
    }
    Ok(profiles)
}
