use std::collections::hash_map::Entry;

use anyhow::{Context, Result};
use foldhash::{HashMap, HashMapExt};
use serde::{Deserialize, Serialize};
use sqlx::{Executor, Pool, Sqlite};
use tokio_stream::StreamExt;
use wordbase::{DictionaryId, Profile, ProfileId, protocol::NotFound};

#[derive(Debug, Serialize, Deserialize)]
struct ProfileMeta<'a> {
    name: &'a str,
}

pub async fn new(db: &Pool<Sqlite>, name: &str) -> Result<ProfileId> {
    let meta =
        serde_json::to_string(&ProfileMeta { name }).context("failed to serialize profile meta")?;

    let mut tx = db.begin().await.context("failed to begin transaction")?;
    let current_profile_id = current_id(&mut *tx)
        .await
        .context("failed to fetch current profile id")?
        .0;
    let new_profile_id = sqlx::query!("INSERT INTO profile (meta) VALUES ($1)", meta)
        .execute(&mut *tx)
        .await
        .context("failed to insert profile")?
        .last_insert_rowid();
    sqlx::query!(
        "INSERT INTO profile_enabled_dictionary (profile, dictionary)
        SELECT $1, dictionary
        FROM profile_enabled_dictionary
        WHERE profile = $2",
        new_profile_id,
        current_profile_id,
    )
    .execute(&mut *tx)
    .await
    .context("failed to copy enabled dictionaries")?;

    tx.commit().await.context("failed to commit transaction")?;
    Ok(ProfileId(new_profile_id))
}

pub async fn current_id<'e, 'c: 'e, E>(executor: E) -> Result<ProfileId>
where
    E: 'e + Executor<'c, Database = Sqlite>,
{
    let id = sqlx::query_scalar!("SELECT current_profile FROM config")
        .fetch_one(executor)
        .await?;
    Ok(ProfileId(id))
}

pub async fn all<'e, 'c: 'e, E>(executor: E) -> Result<HashMap<ProfileId, Profile>>
where
    E: 'e + Executor<'c, Database = Sqlite>,
{
    let mut profiles = HashMap::<ProfileId, Profile>::new();

    let mut records = sqlx::query!(
        "SELECT profile.id, profile.meta, ped.dictionary
        FROM profile
        LEFT JOIN profile_enabled_dictionary ped ON profile.id = ped.profile"
    )
    .fetch(executor);
    while let Some(record) = records.next().await {
        let record = record.context("failed to fetch record")?;
        let id = ProfileId(record.id);
        let entry = match profiles.entry(id) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let meta = serde_json::from_str::<ProfileMeta>(&record.meta)
                    .context("failed to deserialize profile meta")?;
                entry.insert(Profile {
                    id,
                    name: meta.name.into(),
                    enabled_dictionaries: Vec::new(),
                })
            }
        };

        if let Some(dictionary) = record.dictionary {
            entry.enabled_dictionaries.push(DictionaryId(dictionary));
        }
    }

    Ok(profiles)
}

pub async fn remove<'e, 'c: 'e, E>(
    executor: E,
    profile_id: ProfileId,
) -> Result<Result<(), NotFound>>
where
    E: 'e + Executor<'c, Database = Sqlite>,
{
    let result = sqlx::query!(
        "DELETE FROM profile
        WHERE id = $1",
        profile_id.0
    )
    .execute(executor)
    .await?;
    Ok(if result.rows_affected() > 0 {
        Ok(())
    } else {
        Err(NotFound)
    })
}
