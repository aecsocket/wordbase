use {
    anyhow::{Context, Result},
    serde::{Deserialize, Serialize},
    sqlx::{
        Pool, Sqlite,
        sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    },
    std::{io, path::Path},
};

pub async fn setup(path: &Path) -> Result<Pool<Sqlite>> {
    let db = SqlitePoolOptions::new()
        .connect_with(connect_options(path))
        .await
        .context("failed to connect to database")?;
    sqlx::query(include_str!("schema.sql"))
        .execute(&db)
        .await
        .context("failed to set up database")?;
    let config = sqlx::query!("SELECT max_db_connections FROM config")
        .fetch_one(&db)
        .await
        .context("failed to fetch initial config")?;
    drop(db);

    let db = SqlitePoolOptions::new()
        .max_connections(u32::try_from(config.max_db_connections).unwrap_or(1))
        .connect_with(connect_options(path))
        .await
        .context("failed to connect to database")?;
    Ok(db)
}

fn connect_options(path: &Path) -> SqliteConnectOptions {
    SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        // .journal_mode(SqliteJournalMode::Wal)
        .pragma("foreign_keys", "ON")
}

pub fn serialize(
    value: &impl Serialize,
    writer: impl io::Write,
) -> Result<(), rmp_serde::encode::Error> {
    value.serialize(&mut rmp_serde::Serializer::new(writer))
}

pub fn deserialize<'a, T: Deserialize<'a>>(buf: &'a [u8]) -> Result<T, rmp_serde::decode::Error> {
    rmp_serde::from_slice(buf)
}
