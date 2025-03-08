use {
    anyhow::{Context, Result},
    futures::{StreamExt, TryStreamExt},
    sqlx::{Executor, Sqlite, Transaction},
    wordbase::{Dictionary, DictionaryId, protocol::DictionaryNotFound},
};

pub async fn insert(
    tx: &mut Transaction<'_, Sqlite>,
    dictionary: &Dictionary,
) -> Result<DictionaryId> {
    let max_position = sqlx::query_scalar!("SELECT MAX(position) FROM dictionary")
        .fetch_one(&mut **tx)
        .await
        .context("failed to fetch max dictionary position")?
        .unwrap_or(1);
    let next_position = max_position + 1;

    let result = sqlx::query!(
        "INSERT INTO dictionary (name, version, position)
        VALUES ($1, $2, $3)",
        dictionary.name,
        dictionary.version,
        next_position
    )
    .execute(&mut **tx)
    .await
    .context("failed to insert dictionary")?;

    Ok(DictionaryId(result.last_insert_rowid()))
}

pub async fn all<'e, 'c: 'e, E>(executor: E) -> Result<Vec<Dictionary>>
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
        anyhow::Ok(Dictionary {
            id: DictionaryId(record.id),
            name: record.name,
            version: record.version,
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
    .await
    .context("failed to delete record")?;
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
    .await
    .context("failed to delete record")?;
    Ok(if result.rows_affected() > 0 {
        Ok(())
    } else {
        Err(DictionaryNotFound)
    })
}
