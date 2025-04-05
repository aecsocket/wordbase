use {
    crate::Engine,
    anyhow::{Context, Result, bail},
    arc_swap::ArcSwap,
    derive_more::{Display, Error},
    foldhash::HashMap,
    futures::StreamExt,
    sqlx::{Pool, Sqlite},
    std::sync::Arc,
    wordbase::{DictionaryId, Profile, ProfileId, ProfileMeta},
};

pub type SharedProfiles = Arc<ArcSwap<Profiles>>;

#[derive(Debug)]
pub struct Profiles {
    pub by_id: HashMap<ProfileId, Profile>,
    pub current_id: ProfileId,
}

impl Profiles {
    pub(super) async fn fetch(db: &Pool<Sqlite>) -> Result<Self> {
        let by_id = fetch_owned(db)
            .await
            .context("failed to fetch profiles")?
            .into_iter()
            .map(|profile| (profile.id, profile))
            .collect::<HashMap<_, _>>();
        let current_id = ProfileId(
            sqlx::query_scalar!("SELECT current_profile FROM config")
                .fetch_one(db)
                .await
                .context("failed to fetch current profile")?,
        );
        Ok(Self { by_id, current_id })
    }
}

impl Engine {
    async fn sync_profiles(&self) -> Result<()> {
        self.profiles.store(Arc::new(
            Profiles::fetch(&self.db)
                .await
                .context("failed to sync profiles")?,
        ));
        Ok(())
    }

    pub async fn insert_profile(&self, meta: ProfileMeta) -> Result<ProfileId> {
        let current_id = self.profiles.load().current_id;
        let meta_json = serde_json::to_string(&meta).context("failed to serialize profile meta")?;

        let mut tx = self
            .db
            .begin()
            .await
            .context("failed to begin transaction")?;
        let new_id = sqlx::query!("INSERT INTO profile (meta) VALUES ($1)", meta_json)
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

    pub async fn set_profile_sorting_dictionary(
        &self,
        profile_id: ProfileId,
        dictionary_id: Option<DictionaryId>,
    ) -> Result<()> {
        let dictionary_id = dictionary_id.map(|id| id.0);
        sqlx::query!(
            "UPDATE profile SET sorting_dictionary = $1 WHERE id = $2",
            dictionary_id,
            profile_id.0
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

async fn fetch_owned(db: &Pool<Sqlite>) -> Result<Vec<Profile>> {
    let mut profiles = Vec::<Profile>::new();

    let mut records = sqlx::query!(
        "SELECT
            id,
            meta,
            sorting_dictionary,
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
                profiles.push(Profile {
                    id,
                    meta,
                    enabled_dictionaries: Vec::new(),
                    sorting_dictionary: record.sorting_dictionary.map(DictionaryId),
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
