use {
    anyhow::{Context, Result},
    serde::{Deserialize, Serialize},
    sqlx::{
        Pool, Sqlite,
        sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    },
    std::{io, path::Path},
};

pub async fn setup(path: &Path, max_db_connections: u32) -> Result<Pool<Sqlite>> {
    let db = SqlitePoolOptions::new()
        .max_connections(max_db_connections)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(path)
                .create_if_missing(true)
                .journal_mode(SqliteJournalMode::Wal)
                .pragma("foreign_keys", "ON"),
        )
        .await
        .context("failed to connect to database")?;

    sqlx::query(include_str!("schema.sql"))
        .execute(&db)
        .await
        .context("failed to set up database")?;

    Ok(db)
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
