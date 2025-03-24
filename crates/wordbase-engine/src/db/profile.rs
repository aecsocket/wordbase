use anyhow::{Context, Result};
use sqlx::{Executor, Pool, Sqlite};
use tokio_stream::StreamExt;
use wordbase::{DictionaryId, ProfileId, ProfileMeta, ProfileState, protocol::NotFound};

pub async fn create(db: &Pool<Sqlite>, meta: &ProfileMeta) -> Result<ProfileId> {
    let meta = serde_json::to_string(meta).context("failed to serialize profile meta")?;

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

pub async fn all<'e, 'c: 'e, E>(executor: E) -> Result<Vec<ProfileState>>
where
    E: 'e + Executor<'c, Database = Sqlite>,
{
    let mut profiles = Vec::<ProfileState>::new();

    let mut records = sqlx::query!(
        "SELECT profile.id, profile.meta, ped.dictionary
        FROM profile
        LEFT JOIN profile_enabled_dictionary ped ON profile.id = ped.profile"
    )
    .fetch(executor);
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
                profiles.push(ProfileState {
                    id,
                    meta,
                    enabled_dictionaries: Vec::new(),
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

pub async fn current_id<'e, 'c: 'e, E>(executor: E) -> Result<ProfileId>
where
    E: 'e + Executor<'c, Database = Sqlite>,
{
    let id = sqlx::query_scalar!("SELECT current_profile FROM config")
        .fetch_one(executor)
        .await?;
    Ok(ProfileId(id))
}

pub async fn set_current_id<'e, 'c: 'e, E>(
    executor: E,
    profile_id: ProfileId,
) -> Result<Result<(), NotFound>>
where
    E: 'e + Executor<'c, Database = Sqlite>,
{
    todo!()
    // let id = sqlx::query_scalar!("SELECT current_profile FROM config")
    //     .fetch_one(executor)
    //     .await?;
    // Ok(ProfileId(id))
}
