use {
    anyhow::{Context, Result},
    futures::{StreamExt, TryStreamExt},
    sqlx::{Executor, Sqlite},
    wordbase::{DictionaryId, DictionaryMeta, DictionaryState, protocol::NotFound},
};

pub async fn insert<'e, 'c: 'e, E>(executor: E, meta: &DictionaryMeta) -> Result<DictionaryId>
where
    E: 'e + Executor<'c, Database = Sqlite>,
{
    let meta = serde_json::to_string(meta).context("failed to serialize dictionary meta")?;
    let result = sqlx::query!(
        "INSERT INTO dictionary (position, meta)
        VALUES (
            (SELECT COALESCE(MAX(position), 0) + 1 FROM dictionary),
            $1
        )",
        meta
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
        "SELECT EXISTS(SELECT 1 FROM dictionary WHERE json_extract(meta, '$.name') = $1)",
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
        "SELECT id, position, meta
        FROM dictionary
        ORDER BY position"
    )
    .fetch(executor)
    .map(|record| {
        let record = record.context("failed to fetch record")?;
        let meta = serde_json::from_str::<DictionaryMeta>(&record.meta)
            .context("failed to deserialize dictionary meta")?;
        anyhow::Ok(DictionaryState {
            id: DictionaryId(record.id),
            position: record.position,
            meta,
        })
    })
    .try_collect::<Vec<_>>()
    .await
}

pub async fn remove<'e, 'c: 'e, E>(
    executor: E,
    dictionary_id: DictionaryId,
) -> Result<Result<(), NotFound>>
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
        Err(NotFound)
    })
}

pub async fn set_position<'e, 'c: 'e, E>(
    executor: E,
    dictionary_id: DictionaryId,
    position: i64,
) -> Result<Result<(), NotFound>>
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
        Err(NotFound)
    })
}
