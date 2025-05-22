use {
    crate::{Engine, EngineEvent, IndexMap, NotFound, ProfileEvent},
    anyhow::{Context, Result, bail},
    derive_more::Deref,
    futures::StreamExt,
    sqlx::{Pool, Sqlite},
    std::sync::Arc,
    wordbase_api::{DictionaryId, NormString, Profile, ProfileId},
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

    pub async fn add_profile(&self, name: Option<NormString>) -> Result<ProfileId> {
        let name = name.as_ref().map(|s| s.as_str());
        let id = sqlx::query!("INSERT INTO profile (name) VALUES ($1)", name)
            .execute(&self.db)
            .await
            .context("failed to insert profile")?
            .last_insert_rowid();
        let id = ProfileId(id);

        self.sync_profiles().await?;
        _ = self
            .event_tx
            .send(EngineEvent::Profile(ProfileEvent::Added { id }));
        Ok(id)
    }

    pub async fn copy_profile(
        &self,
        src_id: ProfileId,
        new_name: Option<NormString>,
    ) -> Result<ProfileId> {
        let new_name = new_name.as_ref().map(|s| s.as_str());
        let mut tx = self
            .db
            .begin()
            .await
            .context("failed to begin transaction")?;
        let new_id = sqlx::query!(
            "INSERT INTO profile (name, sorting_dictionary, font_family, anki_deck, anki_note_type)
            SELECT $1, sorting_dictionary, font_family, anki_deck, anki_note_type
            FROM profile
            WHERE id = $2",
            new_name,
            src_id.0,
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
            .event_tx
            .send(EngineEvent::Profile(ProfileEvent::Copied {
                src_id,
                new_id,
            }));
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
        _ = self
            .event_tx
            .send(EngineEvent::Profile(ProfileEvent::Removed { id }));
        Ok(())
    }

    pub async fn set_profile_name(
        &self,
        profile_id: ProfileId,
        name: Option<NormString>,
    ) -> Result<()> {
        let name = name.as_ref().map(|s| s.as_str());
        sqlx::query!(
            "UPDATE profile SET name = $1 WHERE id = $2",
            name,
            profile_id.0
        )
        .execute(&self.db)
        .await?;

        self.sync_profiles().await?;
        _ = self
            .event_tx
            .send(EngineEvent::Profile(ProfileEvent::NameSet {
                id: profile_id,
            }));
        Ok(())
    }

    pub async fn set_font_family(
        &self,
        profile_id: ProfileId,
        font_family: Option<&str>,
    ) -> Result<()> {
        sqlx::query!(
            "UPDATE profile SET font_family = $1 WHERE id = $2",
            font_family,
            profile_id.0
        )
        .execute(&self.db)
        .await?;

        self.sync_profiles().await?;
        _ = self
            .event_tx
            .send(EngineEvent::FontFamilySet { profile_id });
        Ok(())
    }
}

async fn fetch_owned(db: &Pool<Sqlite>) -> Result<Vec<Profile>> {
    let mut profiles = Vec::<Profile>::new();

    let mut records = sqlx::query!(
        "SELECT
            profile.id,
            profile.name,
            profile.sorting_dictionary,
            profile.font_family,
            profile.anki_deck,
            profile.anki_note_type,
            ped.dictionary
        FROM profile
        LEFT JOIN profile_enabled_dictionary ped ON profile.id = ped.profile
        ORDER BY profile.id"
    )
    .fetch(db);
    while let Some(record) = records.next().await {
        let record = record.context("failed to fetch record")?;
        let id = ProfileId(record.id);

        let profile_index = profiles.iter_mut().position(|profile| profile.id == id);
        let profile_index = if let Some(index) = profile_index {
            index
        } else {
            let index = profiles.len();
            let mut profile = Profile::new(ProfileId(record.id));
            profile.name = record.name.and_then(NormString::new);
            profile.sorting_dictionary = record.sorting_dictionary.map(DictionaryId);
            profile.font_family = record.font_family;
            profile.anki_deck = record.anki_deck;
            profile.anki_note_type = record.anki_note_type;
            profiles.push(profile);
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

#[cfg(feature = "uniffi")]
#[uniffi::export]
impl Engine {
    #[uniffi::method(name = "profiles")]
    pub fn ffi_profiles(&self) -> Vec<Profile> {
        self.profiles()
            .iter()
            .map(|(_, profile)| (**profile).clone())
            .collect()
    }
}
