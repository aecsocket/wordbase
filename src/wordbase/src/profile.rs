use {
    crate::{IndexMap, NotFound, Wordbase},
    anyhow::{Context, Result, bail},
    arc_swap::ArcSwap,
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

    pub(super) async fn sync(db: &Pool<Sqlite>, profiles: &ArcSwap<Profiles>) -> Result<()> {
        let fetched = Profiles::fetch(db)
            .await
            .context("failed to sync profiles")?;
        profiles.store(Arc::new(fetched));
        Ok(())
    }
}

impl Wordbase {
    #[must_use]
    pub fn profiles(&self) -> Arc<Profiles> {
        self.profiles.load().clone()
    }

    pub async fn add_profile(&self, name: Option<NormString>) -> Result<ProfileId> {
        let name = name.as_ref().map(|s| s.as_str());
        let id = sqlx::query!("INSERT INTO profile (name) VALUES ($1)", name)
            .execute(&self.db)
            .await
            .context("failed to insert profile")?
            .last_insert_rowid();
        let id = ProfileId(id);

        Profiles::sync(&self.db, &self.profiles).await?;
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
            "INSERT INTO profile (name, sorting_dictionary, font_family, anki_deck, \
             anki_note_type)
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

        Profiles::sync(&self.db, &self.profiles).await?;
        Ok(new_id)
    }

    pub async fn remove_profile(&self, id: ProfileId) -> Result<()> {
        let result = sqlx::query!("DELETE FROM profile WHERE id = $1", id.0)
            .execute(&self.db)
            .await?;
        if result.rows_affected() == 0 {
            bail!(NotFound);
        }

        Profiles::sync(&self.db, &self.profiles).await?;
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

        Profiles::sync(&self.db, &self.profiles).await?;
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

        Profiles::sync(&self.db, &self.profiles).await?;
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
const _: () = {
    use {
        crate::{FfiResult, Wordbase, WordbaseError},
        std::collections::HashMap,
    };

    #[uniffi::export(async_runtime = "tokio")]
    impl Wordbase {
        #[uniffi::method(name = "profiles")]
        pub fn ffi_profiles(&self) -> HashMap<ProfileId, Profile> {
            self.profiles()
                .iter()
                .map(|(id, profile)| (*id, (**profile).clone()))
                .collect()
        }

        #[uniffi::method(name = "add_profile")]
        pub async fn ffi_add_profile(&self, name: Option<NormString>) -> FfiResult<ProfileId> {
            Ok(self.add_profile(name).await?)
        }

        #[uniffi::method(name = "copy_profile")]
        pub async fn ffi_copy_profile(
            &self,
            src_id: ProfileId,
            new_name: Option<NormString>,
        ) -> FfiResult<ProfileId> {
            self.copy_profile(src_id, new_name)
                .await
                .map_err(WordbaseError::Ffi)
        }

        #[uniffi::method(name = "remove_profile")]
        pub async fn ffi_remove_profile(&self, id: ProfileId) -> FfiResult<()> {
            Ok(self.remove_profile(id).await?)
        }

        #[uniffi::method(name = "set_profile_name")]
        pub async fn ffi_set_profile_name(
            &self,
            profile_id: ProfileId,
            name: Option<NormString>,
        ) -> FfiResult<()> {
            Ok(self.set_profile_name(profile_id, name).await?)
        }

        #[uniffi::method(name = "set_font_family")]
        pub async fn ffi_set_font_family(
            &self,
            profile_id: ProfileId,
            font_family: Option<String>,
        ) -> FfiResult<()> {
            Ok(self
                .set_font_family(profile_id, font_family.as_deref())
                .await?)
        }
    }
};
