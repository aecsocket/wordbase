use std::str::FromStr;

use anyhow::Result;
use sqlx::{
    Pool, Sqlite,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};

pub mod config;
pub mod dictionary;
pub mod profile;
pub mod term;

const MAX_CONNECTIONS: u32 = 4;

pub async fn connect(path: impl AsRef<str>) -> Result<Pool<Sqlite>> {
    let db = SqlitePoolOptions::new()
        .max_connections(MAX_CONNECTIONS)
        .connect_with(
            SqliteConnectOptions::from_str(&format!("sqlite://{}", path.as_ref()))?
                .create_if_missing(true)
                .journal_mode(SqliteJournalMode::Wal),
        )
        .await?;
    Ok(db)
}
