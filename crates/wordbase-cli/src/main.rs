//! TODO

use {
    anyhow::{Context, Result},
    futures::TryStreamExt,
    std::{convert::Infallible, io::Cursor},
    tokio::fs,
    wordbase::{RecordKind, protocol::LookupRequest},
};

#[tokio::main]
async fn main() -> Result<()> {
    let (mut engine, engine_task) = wordbase_engine::run("/home/dev/wordbase.db")
        .await
        .context("failed to create engine")?;
    tokio::spawn(engine_task);

    let data = fs::read("/home/dev/dictionaries/jmnedict.zip")
        .await
        .context("failed to read dictionary to memory")?;

    let import = engine
        .imports
        .yomitan(|| Ok::<_, Infallible>(Cursor::new(&data)))
        .await
        .context("failed to start importing dictionary")?;
    println!("Importing {:?}", import.meta.name);

    while let Some(progress) = import.await {
        let progress = progress.context("failed to import dictionary")?;
        println!("{:.2}% done", progress * 100.0);
    }
    println!("Import complete");

    let records = engine
        .lookups
        .lookup(LookupRequest {
            text: "hello".into(),
            record_kinds: vec![RecordKind::YomitanRecord],
        })
        .await?
        .try_collect::<Vec<_>>()
        .await?;
    println!("{records:#?}");

    Ok(())
}
