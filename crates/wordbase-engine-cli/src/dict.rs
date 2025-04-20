use {
    anyhow::{Context, Result},
    ascii_table::AsciiTable,
    bytes::Bytes,
    std::{path::Path, time::Instant},
    tokio::{fs, sync::oneshot},
    wordbase::{DictionaryId, Profile},
    wordbase_engine::{Engine, import::ImportStarted},
};

pub fn ls(engine: &Engine, profile: &Profile) {
    let mut table = AsciiTable::default();
    table.column(0).set_header("Sort");
    table.column(1).set_header("On");
    table.column(2).set_header("Pos");
    table.column(3).set_header("ID");
    table.column(4).set_header("Name");
    table.column(5).set_header("Version");

    let dictionaries = engine.dictionaries();
    let data = dictionaries
        .values()
        .map(|dict| {
            vec![
                (if profile.config.sorting_dictionary == Some(dict.id) {
                    "✔"
                } else {
                    ""
                })
                .to_string(),
                (if profile.enabled_dictionaries.contains(&dict.id) {
                    "✔"
                } else {
                    ""
                })
                .to_string(),
                format!("{}", dict.position),
                format!("{}", dict.id.0),
                dict.meta.name.clone(),
                dict.meta.version.clone().unwrap_or_default(),
            ]
        })
        .collect::<Vec<_>>();
    table.print(&data);
}

pub fn info(engine: &Engine, dict_id: DictionaryId) -> Result<()> {
    let dict = engine
        .dictionaries()
        .get(&dict_id)
        .cloned()
        .context("no dictionary with this ID")?;

    println!("{:?} version {:?}", dict.meta.name, dict.meta.version);
    println!("  ID {} | Position {}", dict.id.0, dict.position);

    if let Some(url) = &dict.meta.url {
        println!("  URL: {url}");
    }

    if let Some(description) = &dict.meta.description {
        if !description.trim().is_empty() {
            println!();
            println!("--- Description ---");
            println!();
            println!("{description}");
        }
    }

    if let Some(attribution) = &dict.meta.attribution {
        if !attribution.trim().is_empty() {
            println!();
            println!("--- Attribution ---");
            println!();
            println!("{attribution}");
        }
    }

    Ok(())
}

pub async fn import(engine: &Engine, path: &Path) -> Result<()> {
    let start = Instant::now();

    let data = fs::read(path)
        .await
        .map(Bytes::from)
        .context("failed to read dictionary file into memory")?;

    let (send_tracker, recv_tracker) = oneshot::channel::<ImportStarted>();
    let import_task = tokio::spawn({
        let engine = engine.clone();
        async move { engine.import_dictionary(data, send_tracker).await }
    });
    let tracker_task = async move {
        let Ok(mut tracker) = recv_tracker.await else {
            return;
        };

        println!(
            "Importing {:?} version {:?}",
            tracker.meta.name, tracker.meta.version
        );

        while let Some(progress) = tracker.recv_progress.recv().await {
            println!("{:.02}% imported", progress * 100.0);
        }
    };

    let (result, ()) = tokio::join!(import_task, tracker_task);
    result
        .context("import task canceled")?
        .context("failed to import dictionary")?;

    let elapsed = Instant::now().duration_since(start);
    println!("Import complete in {elapsed:?}");
    Ok(())
}

pub async fn set_position(engine: &Engine, dict_id: DictionaryId, position: i64) -> Result<()> {
    engine.set_dictionary_position(dict_id, position).await?;
    Ok(())
}

pub async fn enable(engine: &Engine, profile: &Profile, dict_id: DictionaryId) -> Result<()> {
    engine.enable_dictionary(profile.id, dict_id).await?;
    Ok(())
}

pub async fn disable(engine: &Engine, profile: &Profile, dict_id: DictionaryId) -> Result<()> {
    engine.disable_dictionary(profile.id, dict_id).await?;
    Ok(())
}

pub async fn rm(engine: &Engine, dict_id: DictionaryId) -> Result<()> {
    engine.remove_dictionary(dict_id).await?;
    Ok(())
}
