use {
    crate::{Error, Result, error::Context, storage::EngineStorage},
    derive_more::{Deref, DerefMut},
    eyre::eyre,
    foldhash::{HashMap, HashMapExt, HashSet, HashSetExt},
    futures::StreamExt,
    serde::Serialize,
    sqlx::{Pool, Sqlite},
    std::{collections::hash_map, sync::Arc},
    uuid::Uuid,
    wordbase_core::Profile,
    wordbase_types::{DictionaryId, NormString, ProfileId},
};

#[derive(Debug, Deref, DerefMut, Serialize)]
pub struct Profiles(pub HashMap<ProfileId, Arc<ProfileState>>);

#[derive(Debug, Serialize)]
pub struct ProfileState {
    pub id: ProfileId,
    pub name: Option<NormString>,
    pub sorting_dictionary: Option<DictionaryId>,
    pub enabled_dictionaries: HashSet<DictionaryId>,
}

impl ProfileState {
    #[must_use]
    pub fn to_profile(&self) -> Profile {
        Profile {
            id: self.id,
            name: self.name.clone(),
            sorting_dictionary: self.sorting_dictionary,
            enabled_dictionaries: self.enabled_dictionaries.iter().copied().collect(),
        }
    }
}

pub(crate) async fn create(db: &Pool<Sqlite>, name: &str) -> Result<ProfileId> {
    let profile_id = ProfileId::random();
    let profile_id_data = profile_id.0.as_bytes().as_slice();
    sqlx::query!(
        "INSERT INTO profile (id, name) VALUES (?, ?)",
        profile_id_data,
        name
    )
    .execute(db)
    .await
    .wrap_internal_err("failed to insert profile row")?;
    Ok(profile_id)
}

pub(crate) async fn fetch(db: &Pool<Sqlite>) -> Result<Profiles> {
    fn map_uuid(data: Vec<u8>) -> Result<Uuid> {
        Ok(Uuid::from_bytes(data.try_into().map_err(
            |data: Vec<u8>| {
                Error::Internal(eyre!(
                    "uuid must be {} bytes but value is of length {}",
                    size_of::<Uuid>(),
                    data.len()
                ))
            },
        )?))
    }

    let mut profiles = HashMap::new();

    let mut rows = sqlx::query!(
        "SELECT
            profile.id as profile_id,
            profile.name,
            profile.sorting_dictionary,
            profile_dictionary.dictionary as enabled_dictionary
        FROM profile
        LEFT JOIN profile_dictionary ON profile_dictionary.profile = profile.id
        ORDER BY profile.created_at"
    )
    .fetch(db);
    while let Some(row) = rows
        .next()
        .await
        .transpose()
        .wrap_internal_err("failed to fetch row")?
    {
        let profile_id = map_uuid(row.profile_id)
            .map(ProfileId)
            .wrap_internal_err("failed to map `profile_id`")?;
        let profile = match profiles.entry(profile_id) {
            hash_map::Entry::Occupied(entry) => entry.into_mut(),
            hash_map::Entry::Vacant(entry) => entry.insert(ProfileState {
                id: profile_id,
                name: NormString::new(row.name),
                sorting_dictionary: row
                    .sorting_dictionary
                    .map(|id| map_uuid(id).map(DictionaryId))
                    .transpose()
                    .wrap_internal_err("failed to map `sorting_dictionary`")?,
                enabled_dictionaries: HashSet::new(),
            }),
        };

        if let Some(dictionary) = row.enabled_dictionary {
            profile.enabled_dictionaries.insert(
                map_uuid(dictionary)
                    .map(DictionaryId)
                    .wrap_internal_err("failed to map `enabled_dictionary`")?,
            );
        }
    }

    let profiles = profiles
        .into_iter()
        .map(|(id, profile)| (id, Arc::new(profile)))
        .collect();

    Ok(Profiles(profiles))
}

async fn remove(db: &Pool<Sqlite>, profile_id: ProfileId) -> Result<()> {
    let profile_id = profile_id.0.as_bytes().as_slice();
    sqlx::query!("DELETE FROM profile WHERE id = $1", profile_id)
        .execute(db)
        .await
        .map_err(|err| {
            if let Some(err) = err.as_database_error()
                && err.message().contains("cannot delete last profile")
            {
                crate::Error::Request(eyre!("cannot delete last profile"))
            } else {
                crate::Error::Internal(eyre!(err).wrap_err("failed to delete profile row"))
            }
        })?;
    Ok(())
}

async fn enable_dictionary(
    db: &Pool<Sqlite>,
    profile_id: ProfileId,
    dict_id: DictionaryId,
) -> Result<()> {
    let profile_id = profile_id.0.as_bytes().as_slice();
    let dict_id = dict_id.0.as_bytes().as_slice();
    sqlx::query!(
        "INSERT INTO profile_dictionary (profile, dictionary)
        VALUES (?, ?)",
        profile_id,
        dict_id
    )
    .execute(db)
    .await
    .wrap_internal_err("failed to insert profile dictionary row")?;
    Ok(())
}

async fn disable_dictionary(
    db: &Pool<Sqlite>,
    profile_id: ProfileId,
    dict_id: DictionaryId,
) -> Result<()> {
    let profile_id = profile_id.0.as_bytes().as_slice();
    let dict_id = dict_id.0.as_bytes().as_slice();
    sqlx::query!(
        "DELETE FROM profile_dictionary
        WHERE profile = ? AND dictionary = ?",
        profile_id,
        dict_id
    )
    .execute(db)
    .await
    .wrap_internal_err("failed to delete profile dictionary row")?;
    Ok(())
}

pub(crate) async fn remove_dictionary(db: &Pool<Sqlite>, dict_id: DictionaryId) -> Result<()> {
    let dict_id = dict_id.0.as_bytes().as_slice();
    sqlx::query!(
        "DELETE FROM profile_dictionary
        WHERE dictionary = ?",
        dict_id
    )
    .execute(db)
    .await
    .wrap_internal_err("failed to delete profile dictionary row")?;
    Ok(())
}

impl EngineStorage {
    pub fn profiles(&self) -> Arc<Profiles> {
        self.profiles.load().clone()
    }

    pub fn get_profile(&self, profile_id: ProfileId) -> Result<Arc<ProfileState>> {
        let profiles = self.profiles.load();
        let profile = profiles
            .get(&profile_id)
            .wrap_request_err_with(|| eyre!("invalid profile {profile_id}"))?;
        Ok(profile.clone())
    }

    async fn sync_profiles(&self) -> Result<()> {
        let profiles = fetch(&self.db)
            .await
            .wrap_internal_err("failed to sync profiles")?;
        self.profiles.store(Arc::new(profiles));
        Ok(())
    }

    pub async fn create_profile(&self, name: &str) -> Result<ProfileId> {
        let profile_id = create(&self.db, name).await?;
        self.sync_profiles().await?;
        Ok(profile_id)
    }

    pub async fn remove_profile(&self, profile_id: ProfileId) -> Result<()> {
        remove(&self.db, profile_id).await?;
        self.sync_profiles().await?;
        Ok(())
    }

    pub async fn enable_dictionary(
        &self,
        profile_id: ProfileId,
        dict_id: DictionaryId,
    ) -> Result<()> {
        enable_dictionary(&self.db, profile_id, dict_id).await?;
        self.sync_profiles().await?;
        Ok(())
    }

    pub async fn disable_dictionary(
        &self,
        profile_id: ProfileId,
        dict_id: DictionaryId,
    ) -> Result<()> {
        disable_dictionary(&self.db, profile_id, dict_id).await?;
        self.sync_profiles().await?;
        Ok(())
    }
}
