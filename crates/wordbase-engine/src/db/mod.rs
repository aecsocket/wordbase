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

pub async fn setup(path: &Path) -> Result<Pool<Sqlite>> {
    let db = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(path)
                .create_if_missing(true)
                .journal_mode(SqliteJournalMode::Wal),
        )
        .await
        .context("failed to connect to database")?;

    sqlx::query(include_str!("setup.sql"))
        .execute(&db)
        .await
        .context("failed to set up database")?;

    Ok(db)
}
