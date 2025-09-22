use {
    crate::App,
    ascii_table::AsciiTable,
    eyre::{Context, Result},
    itertools::Itertools,
    std::sync::Arc,
    tracing::info,
    wordbase_engine::{ProfileId, profiles::Profiles},
};

pub async fn list(app: &App) -> Result<Arc<Profiles>> {
    let dicts = app.storage.dictionaries();
    let profiles = app.storage.profiles();
    let cells = profiles
        .iter()
        .map(|(profile_id, profile)| {
            let dict_names = dicts
                .iter()
                .filter(|dict| profile.enabled_dictionaries.contains(&dict.id))
                .map(|dict| &dict.meta.name)
                .join(", ");

            vec![
                profile_id.to_string(),
                profile
                    .name
                    .as_ref()
                    .map_or("(default)", |s| s.as_str())
                    .to_string(),
                dict_names,
            ]
        })
        .collect::<Vec<_>>();

    let mut table = AsciiTable::default();
    table.column(0).set_header("ID");
    table.column(1).set_header("Name");
    table.column(2).set_header("Dictionaries");
    info!("\n{}", table.format(&cells));

    Ok(profiles)
}

pub async fn add(app: &App, name: &str) -> Result<()> {
    app.storage.create_profile(name).await?;
    Ok(())
}

pub async fn remove(app: &App, id: &str) -> Result<()> {
    let id = id.parse::<ProfileId>().wrap_err("invalid profile ID")?;
    app.storage.remove_profile(id).await?;
    Ok(())
}
