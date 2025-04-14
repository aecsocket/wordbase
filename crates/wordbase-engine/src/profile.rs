use {
    crate::{Engine, IndexMap},
    anyhow::{Context, Result, bail},
    derive_more::Deref,
    foldhash::HashMap,
    futures::StreamExt,
    serde::{Deserialize, Serialize},
    sqlx::{Pool, Sqlite},
    std::sync::Arc,
    wordbase::{DictionaryId, NormString, ProfileId, ProfileMeta},
};

#[derive(Debug, Default, Deref)]
pub struct Profiles(pub IndexMap<ProfileId, Arc<ProfileState>>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileState {
    pub id: ProfileId,
    pub meta: ProfileMeta,
    pub enabled_dictionaries: Vec<DictionaryId>,
    pub sorting_dictionary: Option<DictionaryId>,
    pub config: ProfileConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProfileConfig {
    pub font_family: Option<String>,
    pub anki_deck: Option<NormString>,
    pub anki_model: Option<NormString>,
    #[serde(default)]
    pub anki_model_fields: HashMap<NormString, NormString>,
}

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

    pub(crate) async fn sync_profiles(&self) -> Result<()> {
        let profiles = Profiles::fetch(&self.db)
            .await
            .context("failed to sync profiles")?;
        self.profiles.store(Arc::new(profiles));
        Ok(())
    }

    pub async fn copy_profile(
        &self,
        src_id: ProfileId,
        new_meta: &ProfileMeta,
    ) -> Result<ProfileId> {
        let profiles = self.profiles.load();
        let src = profiles.get(&src_id).context("profile not found")?;

        let meta_json =
            serde_json::to_string(&new_meta).context("failed to serialize profile meta")?;
        let config_json =
            serde_json::to_string(&src.config).context("failed to serialize config")?;
        let sorting_dictionary = src.sorting_dictionary.map(|id| id.0);

        let mut tx = self
            .db
            .begin()
            .await
            .context("failed to begin transaction")?;
        let new_id = sqlx::query!(
            "INSERT INTO profile (meta, config, sorting_dictionary)
            VALUES ($1, $2, $3)",
            meta_json,
            config_json,
            sorting_dictionary
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
        Ok(new_id)
    }

    pub async fn set_profile_meta(&self, id: ProfileId, meta: &ProfileMeta) -> Result<()> {
        let meta_json = serde_json::to_string(&meta).context("failed to serialize profile meta")?;
        sqlx::query!(
            "UPDATE profile SET meta = $1 WHERE id = $2",
            meta_json,
            id.0
        )
        .execute(&self.db)
        .await?;

        self.sync_profiles().await?;
        Ok(())
    }

    pub async fn set_profile_config(&self, id: ProfileId, config: &ProfileConfig) -> Result<()> {
        let config_json = serde_json::to_string(config).context("failed to serialize config")?;
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

    pub async fn set_sorting_dictionary(
        &self,
        profile_id: ProfileId,
        dictionary_id: Option<DictionaryId>,
    ) -> Result<()> {
        let dictionary_id = dictionary_id.map(|id| id.0);
        sqlx::query!(
            "UPDATE profile SET sorting_dictionary = $1
            WHERE id = $2",
            dictionary_id,
            profile_id.0
        )
        .execute(&self.db)
        .await?;

        self.sync_profiles().await?;
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
        Ok(())
    }

    pub async fn remove_profile(&self, id: ProfileId) -> Result<()> {
        let result = sqlx::query!("DELETE FROM profile WHERE id = $1", id.0)
            .execute(&self.db)
            .await?;
        if result.rows_affected() == 0 {
            bail!("profile not found");
        }

        self.sync_profiles().await?;
        Ok(())
    }
}

async fn fetch_owned(db: &Pool<Sqlite>) -> Result<Vec<ProfileState>> {
    let mut profiles = Vec::<ProfileState>::new();

    let mut records = sqlx::query!(
        "SELECT
            profile.id,
            profile.meta,
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
                let meta = serde_json::from_str::<ProfileMeta>(&record.meta)
                    .context("failed to deserialize profile meta")?;
                let config = serde_json::from_str::<ProfileConfig>(&record.config)
                    .context("failed to deserialize profile config")?;
                profiles.push(ProfileState {
                    id,
                    meta,
                    enabled_dictionaries: Vec::new(),
                    sorting_dictionary: record.sorting_dictionary.map(DictionaryId),
                    config,
                });
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
