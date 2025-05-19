use {
    anyhow::{Context, Result},
    serde::{Deserialize, Serialize},
    sqlx::{
        Pool, Sqlite,
        sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    },
    std::{io, path::Path},
};

pub async fn setup(path: &Path) -> Result<Pool<Sqlite>> {
    let db = SqlitePoolOptions::new()
        .max_connections(MAX_DB_CONNECTIONS)
        .connect_with(connect_options(path))
        .await
        .context("failed to connect to database")?;
    sqlx::migrate!()
        .run(&db)
        .await
        .context("failed to setup database")?;
    Ok(db)
}

const MAX_DB_CONNECTIONS: u32 = 8;

fn connect_options(path: &Path) -> SqliteConnectOptions {
    SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
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
