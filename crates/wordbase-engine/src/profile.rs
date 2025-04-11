use {
    crate::{Engine, IndexMap},
    anyhow::{Context, Result, bail},
    derive_more::{Display, Error},
    foldhash::HashMap,
    futures::StreamExt,
    serde::{Deserialize, Serialize},
    sqlx::{Pool, Sqlite},
    std::sync::Arc,
    wordbase::{DictionaryId, NormString, ProfileId, ProfileMeta},
};

#[derive(Debug)]
pub struct Profiles {
    pub by_id: IndexMap<ProfileId, Arc<ProfileState>>,
    pub current_id: ProfileId,
    pub current: Arc<ProfileState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileState {
    pub id: ProfileId,
    pub meta: ProfileMeta,
    pub enabled_dictionaries: Vec<DictionaryId>,
    pub sorting_dictionary: Option<DictionaryId>,
    pub config: ProfileConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConfig {
    pub font_family: Option<String>,
    pub font_face: Option<String>,
    pub anki_deck: Option<NormString>,
    pub anki_model: Option<NormString>,
    #[serde(default)]
    pub anki_model_fields: HashMap<NormString, NormString>,
}

impl Profiles {
    pub(super) async fn fetch(db: &Pool<Sqlite>) -> Result<Self> {
        let by_id = fetch_owned(db)
            .await
            .context("failed to fetch profiles")?
            .into_iter()
            .map(|profile| (profile.id, Arc::new(profile)))
            .collect::<IndexMap<_, _>>();
        let current_id = ProfileId(
            sqlx::query_scalar!("SELECT current_profile FROM config")
                .fetch_one(db)
                .await
                .context("failed to fetch current profile")?,
        );
        let current = by_id
            .get(&current_id)
            .with_context(|| format!("{current_id:?} does not exist"))?
            .clone();
        Ok(Self {
            by_id,
            current_id,
            current,
        })
    }
}

impl Engine {
    #[must_use]
    pub fn profiles(&self) -> Arc<Profiles> {
        self.profiles.load().clone()
    }

    pub(crate) async fn sync_profiles(&self) -> Result<()> {
        self.profiles.store(Arc::new(
            Profiles::fetch(&self.db)
                .await
                .context("failed to sync profiles")?,
        ));
        Ok(())
    }

    pub async fn insert_profile(&self, meta: &ProfileMeta) -> Result<ProfileId> {
        let profiles = self.profiles.load();
        let current_id = profiles.current_id;
        let current_profile = profiles
            .by_id
            .get(&current_id)
            .context("no current profile")?;

        let meta_json = serde_json::to_string(meta).context("failed to serialize profile meta")?;
        let config_json =
            serde_json::to_string(&current_profile.config).context("failed to serialize config")?;
        let sorting_dictionary = current_profile.sorting_dictionary.map(|id| id.0);

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
            current_id.0,
        )
        .execute(&mut *tx)
        .await
        .context("failed to copy enabled dictionaries")?;
        tx.commit().await.context("failed to commit transaction")?;

        self.sync_profiles().await?;
        Ok(new_id)
    }

    pub async fn set_current_profile(&self, id: ProfileId) -> Result<()> {
        sqlx::query!("UPDATE config SET current_profile = $1", id.0)
            .execute(&self.db)
            .await?;

        self.sync_profiles().await?;
        Ok(())
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

    pub async fn remove_profile(&self, id: ProfileId) -> Result<()> {
        let result = sqlx::query!("DELETE FROM profile WHERE id = $1", id.0)
            .execute(&self.db)
            .await?;
        if result.rows_affected() == 0 {
            bail!(NotFound);
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

#[derive(Debug, Clone, Display, Error)]
#[display("profile not found")]
pub struct NotFound;
