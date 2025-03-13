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
        "INSERT INTO dictionary (position, name, version, description, url)
        VALUES (
            (SELECT COALESCE(MAX(position), 0) + 1 FROM dictionary),
            $1, $2, $3, $4
        )",
        dictionary.name,
        dictionary.version,
        dictionary.description,
        dictionary.url,
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
        r#"SELECT id as "id!", position, name, version, description, url
        FROM dictionary
        ORDER BY position"#
    )
    .fetch(executor)
    .map(|record| {
        let record = record.context("failed to fetch record")?;
        anyhow::Ok(DictionaryState {
            id: DictionaryId(record.id),
            position: record.position,
            meta: DictionaryMeta {
                name: record.name,
                version: record.version,
                description: record.description,
                url: record.url,
            },
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

pub async fn set_position<'e, 'c: 'e, E>(
    executor: E,
    dictionary_id: DictionaryId,
    position: i64,
) -> Result<Result<(), DictionaryNotFound>>
where
    E: 'e + Executor<'c, Database = Sqlite>,
{
    let result = sqlx::query!(
        "UPDATE dictionary
        SET position = $1
        WHERE id = $2",
        position,
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
