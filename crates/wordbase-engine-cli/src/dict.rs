use {
    anyhow::{Context, Result},
    ascii_table::AsciiTable,
    bytes::Bytes,
    futures::{StreamExt, TryStreamExt},
    std::{path::Path, time::Instant},
    tokio::{fs, sync::oneshot},
    wordbase::{DictionaryId, Profile},
    wordbase_engine::{
        Engine,
        import::{ImportEvent, ImportStarted},
    },
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
                (if profile.sorting_dictionary == Some(dict.id) {
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

    let mut import_events = Box::pin(engine.import_dictionary(data));
    while let Some(event) = import_events
        .try_next()
        .await
        .context("failed to import dictionary")?
    {
        match event {
            ImportEvent::DeterminedKind(kind) => {
                println!("Kind: {kind:?}");
            }
            ImportEvent::ParsedMeta(meta) => {
                println!("Importing {:?} version {:?}", meta.name, meta.version);
            }
            ImportEvent::Progress(progress) => {
                println!("{:.02}% imported", progress * 100.0);
            }
            ImportEvent::Done(id) => {
                println!("Imported as {id:?}");
            }
        }
    }

    let elapsed = Instant::now().duration_since(start);
    println!("Import complete in {elapsed:?}");
    Ok(())
}

pub async fn swap_positions(engine: &Engine, a_id: DictionaryId, b_id: DictionaryId) -> Result<()> {
    engine.swap_dictionary_positions(a_id, b_id).await?;
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
