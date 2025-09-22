use {
    crate::{App, checkmark},
    ascii_table::AsciiTable,
    eyre::{Context, Result},
    std::{path::Path, sync::Arc, time::Instant},
    tracing::info,
    wordbase_engine::{DictionaryId, dictionaries::OpenDictionaries},
};

pub async fn list(app: &App) -> Result<Arc<OpenDictionaries>> {
    let dicts = app.storage.dictionaries();
    let cells = dicts
        .iter()
        .map(|dict| {
            vec![
                checkmark(app.profile.enabled_dictionaries.contains(&dict.id)).to_string(),
                dict.id.to_string(),
                dict.meta.name.clone(),
                dict.meta.version.clone().unwrap_or_default(),
            ]
        })
        .collect::<Vec<_>>();

    let mut table = AsciiTable::default();
    table.column(0).set_header("On");
    table.column(1).set_header("ID");
    table.column(2).set_header("Name");
    table.column(3).set_header("Version");
    info!("\n{}", table.format(&cells));

    Ok(dicts)
}

pub async fn import(app: &App, path: &Path) -> Result<DictionaryId> {
    let start = Instant::now();
    let dict_id = app.storage.import_dictionary(&path).await?;
    info!("Imported in {:?}", start.elapsed());

    Ok(dict_id)
}

pub async fn remove(app: &App, dict_id: &str) -> Result<()> {
    let id = dict_id
        .parse::<DictionaryId>()
        .wrap_err("invalid dictionary ID")?;
    let start = Instant::now();
    app.storage.remove_dictionary(id).await?;
    info!("Removed in {:?}", start.elapsed());

    Ok(())
}

pub async fn enable(app: &App, dict_id: &str) -> Result<()> {
    let dict_id = dict_id
        .parse::<DictionaryId>()
        .wrap_err("invalid dictionary ID")?;
    app.storage
        .enable_dictionary(app.profile_id, dict_id)
        .await?;

    Ok(())
}

pub async fn disable(app: &App, dict_id: &str) -> Result<()> {
    let dict_id = dict_id
        .parse::<DictionaryId>()
        .wrap_err("invalid dictionary ID")?;
    app.storage
        .disable_dictionary(app.profile_id, dict_id)
        .await?;

    Ok(())
}
