use {
    anyhow::{Context, Result},
    futures::{StreamExt, TryStreamExt},
    sqlx::{Executor, Sqlite},
    wordbase::{DictionaryId, DictionaryMeta, DictionaryState, protocol::DictionaryNotFound},
};

pub async fn insert<'e, 'c: 'e, E>(executor: E, dictionary: &DictionaryMeta) -> Result<DictionaryId>
where
    E: 'e + Executor<'c, Database = Sqlite>,
{
    let result = sqlx::query!(
        "INSERT INTO dictionary (name, version, position)
        VALUES ($1, $2, (SELECT COALESCE(MAX(position), 0) + 1 FROM dictionary))",
        dictionary.name,
        dictionary.version
    )
    .execute(executor)
    .await?;
    Ok(DictionaryId(result.last_insert_rowid()))
}

pub async fn exists_by_name<'e, 'c: 'e, E>(executor: E, name: &str) -> Result<bool>
where
    E: 'e + Executor<'c, Database = Sqlite>,
{
    let result = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM dictionary WHERE name = $1)",
        name
    )
    .fetch_one(executor)
    .await?;
    Ok(result > 0)
}

pub async fn all<'e, 'c: 'e, E>(executor: E) -> Result<Vec<DictionaryState>>
where
    E: 'e + Executor<'c, Database = Sqlite>,
{
    sqlx::query!(
        r#"SELECT id as "id!", name, version, position, enabled
        FROM dictionary
        ORDER BY position"#
    )
    .fetch(executor)
    .map(|record| {
        let record = record.context("failed to fetch record")?;
        anyhow::Ok(DictionaryState {
            meta: DictionaryMeta {
                name: record.name,
                version: record.version,
            },
            id: DictionaryId(record.id),
            position: record.position,
            enabled: record.enabled,
        })
    })
    .try_collect::<Vec<_>>()
    .await
}

pub async fn remove<'e, 'c: 'e, E>(
    executor: E,
    dictionary_id: DictionaryId,
) -> Result<Result<(), DictionaryNotFound>>
where
    E: 'e + Executor<'c, Database = Sqlite>,
{
    let result = sqlx::query!(
        "DELETE FROM dictionary
        WHERE id = $1",
        dictionary_id.0
    )
    .execute(executor)
    .await?;
    Ok(if result.rows_affected() > 0 {
        Ok(())
    } else {
        Err(DictionaryNotFound)
    })
}

pub async fn set_enabled<'e, 'c: 'e, E>(
    executor: E,
    dictionary_id: DictionaryId,
    enabled: bool,
) -> Result<Result<(), DictionaryNotFound>>
where
    E: 'e + Executor<'c, Database = Sqlite>,
{
    let result = sqlx::query!(
        "UPDATE dictionary
        SET enabled = $1
        WHERE id = $2",
        enabled,
        dictionary_id.0
    )
    .execute(executor)
    .await?;
    Ok(if result.rows_affected() > 0 {
        Ok(())
    } else {
        Err(DictionaryNotFound)
    })
}
