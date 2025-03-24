//! TODO

use {
    anyhow::{Context, Result},
    futures::TryStreamExt,
    std::{convert::Infallible, io::Cursor},
    tokio::fs,
    wordbase::{ProfileId, ProfileMeta, RecordKind, protocol::LookupRequest},
};

#[tokio::main]
async fn main() -> Result<()> {
    let (engine, engine_task) = wordbase_engine::run(&wordbase_engine::Config {
        db_path: "/home/dev/wordbase.db".into(),
        max_db_connections: 8,
        max_concurrent_imports: 4,
    })
    .await
    .context("failed to create engine")?;
    tokio::spawn(engine_task);

    // engine
    //     .profiles
    //     .create(&ProfileMeta {
    //         name: Some("hello world".into()),
    //     })
    //     .await
    //     .context("failed to create profile")?;

    println!("A");
    // engine
    //     .profiles
    //     .remove(ProfileId(1))
    //     .await
    //     .context("failed to delete profile")?
    //     .context("failed to delete profile")?;
    println!("B");

    println!("profiles: {:#?}", engine.profiles.all().await);

    let data = fs::read("/home/dev/dictionaries/jmnedict.zip")
        .await
        .context("failed to read dictionary to memory")?;

    // let import = engine
    //     .imports
    //     .yomitan(|| Ok::<_, Infallible>(Cursor::new(&data)))
    //     .await
    //     .context("failed to start importing dictionary")?;
    // println!("Importing {:?}", import.meta.name);

    // // while let Some(progress) = import.await {
    // //     let progress = progress.context("failed to import dictionary")?;
    // //     println!("{:.2}% done", progress * 100.0);
    // // }
    // println!("Import complete");

    let records = engine
        .lookups
        .lookup(LookupRequest {
            text: "hello".into(),
            record_kinds: vec![RecordKind::YomitanRecord],
        })
        .await
        .try_collect::<Vec<_>>()
        .await?;
    println!("{records:#?}");

    Ok(())
}
