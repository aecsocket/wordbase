use {
    anyhow::{Context, Result},
    futures::{StreamExt, TryStreamExt},
    serde::{Deserialize, Serialize},
    sqlx::{Executor, Sqlite},
    wordbase::{Dictionary, DictionaryId, protocol::NotFound},
};

#[derive(Debug, Serialize, Deserialize)]
struct DictionaryMeta<'a> {
    name: &'a str,
    version: &'a str,
    description: Option<&'a str>,
    url: Option<&'a str>,
}

pub async fn insert<'e, 'c: 'e, E>(executor: E, dictionary: &Dictionary) -> Result<DictionaryId>
where
    E: 'e + Executor<'c, Database = Sqlite>,
{
    let meta = serde_json::to_string(&DictionaryMeta {
        name: &dictionary.name,
        version: &dictionary.version,
        description: dictionary.description.as_deref(),
        url: dictionary.url.as_deref(),
    })
    .context("failed to serialize dictionary meta")?;
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

pub async fn all<'e, 'c: 'e, E>(executor: E) -> Result<Vec<Dictionary>>
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
        anyhow::Ok(Dictionary {
            id: DictionaryId(record.id),
            name: meta.name.to_owned(),
            version: meta.version.to_owned(),
            description: meta.description.map(ToOwned::to_owned),
            url: meta.url.map(ToOwned::to_owned),
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
