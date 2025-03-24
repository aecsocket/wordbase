pub mod dictionary;
pub mod profile;
pub mod term;

use {
    anyhow::{Context, Result},
    sqlx::{
        Pool, Sqlite,
        sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    },
    std::path::Path,
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

    sqlx::query(include_str!("setup.sql"))
        .execute(&db)
        .await
        .context("failed to set up database")?;

    Ok(db)
}
