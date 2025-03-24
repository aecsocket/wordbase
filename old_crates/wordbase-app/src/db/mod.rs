use std::{str::FromStr, thread};

use anyhow::{Context, Result};
use sqlx::{
    Pool, Sqlite,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};

// pub mod config;
// pub mod dictionary;
// pub mod profile;
// pub mod term;

pub async fn setup(path: impl AsRef<str>) -> Result<Pool<Sqlite>> {
    let path = path.as_ref();
    let max_connections = thread::available_parallelism()
        .context("failed to fetch available parallelism")?
        .get();

    let db = SqlitePoolOptions::new()
        .max_connections(u32::try_from(max_connections).unwrap_or(1))
        .connect_with(
            SqliteConnectOptions::from_str(&format!("sqlite://{path}"))
                .context("invalid database file path")?
                .create_if_missing(true)
                .journal_mode(SqliteJournalMode::Wal),
        )
        .await?;
    Ok(db)
}
