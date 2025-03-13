use anyhow::{Context, Result};
use sqlx::{Executor, Pool, Sqlite};
use wordbase::{DictionaryId, ProfileId};

pub async fn copy_current(db: &Pool<Sqlite>, new_name: &str) -> Result<ProfileId> {
    let mut tx = db.begin().await.context("failed to begin transaction")?;

    let result = sqlx::query!(
        "INSERT INTO profile (name, data)
        SELECT $1, profile.data
        FROM profile
        JOIN config ON profile.id = config.current_profile",
        new_name
    )
    .execute(&mut *tx)
    .await
    .context("failed to copy current profile")?;
    let new_profile_id = result.last_insert_rowid();

    sqlx::query!(
        "INSERT INTO profile_enabled_dictionary (profile, dictionary)
        SELECT $1, profile.data
        FROM profile
        JOIN config ON profile.id = config.current_profile",
        new_profile_id
    )
    .execute(&mut *tx)
    .await
    .context("failed to copy enabled dictionaries")?;

    tx.commit().await.context("failed to commit transaction")?;
    Ok(ProfileId(new_profile_id))
}

pub async fn enable_dictionary<'e, 'c: 'e, E>(
    executor: E,
    dictionary_id: DictionaryId,
) -> Result<()>
where
    E: 'e + Executor<'c, Database = Sqlite>,
{
    sqlx::query!(
        "INSERT INTO profile_enabled_dictionary (profile, dictionary)
        VALUES (
            (SELECT current_profile FROM config),
            $1
        )",
        dictionary_id.0
    )
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn disable_dictionary<'e, 'c: 'e, E>(
    executor: E,
    dictionary_id: DictionaryId,
) -> Result<()>
where
    E: 'e + Executor<'c, Database = Sqlite>,
{
    sqlx::query!(
        "DELETE FROM profile_enabled_dictionary
        WHERE profile = (SELECT current_profile FROM config)
        AND dictionary = $1",
        dictionary_id.0
    )
    .execute(executor)
    .await?;
    Ok(())
}
