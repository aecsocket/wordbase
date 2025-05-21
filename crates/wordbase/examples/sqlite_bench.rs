#![expect(missing_docs, reason = "util crate")]

use std::{
    thread::current,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use itertools::Itertools;
use sqlx::{
    Executor, query,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};

#[tokio::main]
async fn main() -> Result<()> {
    const BIND_LIMIT: usize = 32766;

    let db = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(
            SqliteConnectOptions::new()
                .journal_mode(SqliteJournalMode::Wal)
                .pragma("synchronous", "OFF"),
        )
        .await
        .context("failed to create database")?;

    db.execute(
        "CREATE TABLE testing (
            id      INTEGER NOT NULL PRIMARY KEY,
            data    INTEGER NOT NULL
        )",
    )
    .await
    .context("failed to setup")?;

    let total = 1_000_000;
    let batch_size = 25_000;

    let mut tx = db.begin().await.context("failed to begin tx")?;

    let start = Instant::now();

    for batch in 0..(total / batch_size) {
        let mut qb = sqlx::QueryBuilder::new("INSERT INTO testing (data) ");
        qb.push_values(
            (0..batch_size).map(|i| batch * batch_size + i),
            |mut b, i| {
                b.push_bind(i);
            },
        );
        let q = qb.build();
        q.execute(&mut *tx).await.context("failed to insert")?;
        println!("did batch {batch}");
    }

    let end = Instant::now();
    tx.commit().await.context("failed to commit")?;

    let dur = end.duration_since(start);
    println!(
        "
        Took {dur:?}
        - {:?} per row
        - {:.1} rows per sec",
        dur / total,
        total as f64 / dur.as_secs_f64(),
    );

    Ok(())
}
