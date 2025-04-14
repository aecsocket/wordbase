use anyhow::{Context, Result};
use ascii_table::AsciiTable;
use wordbase::{DictionaryId, NormString, ProfileMeta};
use wordbase_engine::{Engine, profile::ProfileState};

pub fn ls(engine: &Engine) {
    let mut table = AsciiTable::default();
    table.column(1).set_header("ID");
    table.column(2).set_header("Name");
    table.column(3).set_header("Sorting Dict");
    table.column(4).set_header("Anki Deck");
    table.column(5).set_header("Dictionaries");

    let dicts = engine.dictionaries();
    let name_of_dict = |dict_id: DictionaryId| {
        dicts
            .get(&dict_id)
            .map_or_else(|| "?".into(), |dict| dict.meta.name.clone())
    };

    let profiles = engine.profiles();

    let data = profiles
        .values()
        .map(|profile| {
            let num_dictionaries = profile.enabled_dictionaries.len();
            let enabled_dictionaries = profile
                .enabled_dictionaries
                .iter()
                .map(|dict| name_of_dict(*dict))
                .collect::<Vec<_>>()
                .join(", ");

            let sorting_dictionary = profile
                .sorting_dictionary
                .map(name_of_dict)
                .unwrap_or_default();

            vec![
                format!("{}", profile.id.0),
                profile
                    .meta
                    .name
                    .as_ref()
                    .map_or_else(|| "(default)".into(), |s| s.clone().into_inner()),
                sorting_dictionary,
                profile
                    .config
                    .anki_deck
                    .as_ref()
                    .map(|s| s.clone().into_inner())
                    .unwrap_or_default(),
                format!("({num_dictionaries}) {enabled_dictionaries}"),
            ]
        })
        .collect::<Vec<_>>();
    table.print(&data);
}

pub async fn copy(engine: &Engine, profile: &ProfileState, name: String) -> Result<()> {
    let name = NormString::new(name).context("invalid new name")?;
    let new_id = engine
        .copy_profile(
            profile.id,
            &ProfileMeta {
                name: Some(name),
                accent_color: None,
            },
        )
        .await?;
    println!("{}", new_id.0);
    Ok(())
}

pub fn info(_engine: &Engine, profile: &ProfileState) {
    println!("{profile:#?}");
}

pub async fn set_name(engine: &Engine, profile: &ProfileState, name: Option<String>) -> Result<()> {
    let name = name
        .map(|name| NormString::new(name).context("invalid new name"))
        .transpose()?;
    engine
        .set_profile_meta(
            profile.id,
            &ProfileMeta {
                name,
                ..profile.meta.clone()
            },
        )
        .await?;
    Ok(())
}

pub async fn rm(engine: &Engine, profile: &ProfileState) -> Result<()> {
    engine.remove_profile(profile.id).await?;
    Ok(())
}
