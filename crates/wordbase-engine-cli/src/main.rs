#![doc = include_str!("../README.md")]

use {
    anyhow::{Context as _, Result},
    ascii_table::AsciiTable,
    bytes::Bytes,
    directories::ProjectDirs,
    std::{path::PathBuf, time::Instant},
    tokio::{fs, sync::oneshot},
    tracing::{info, level_filters::LevelFilter},
    tracing_subscriber::EnvFilter,
    wordbase::{DictionaryId, NormString, ProfileId, ProfileMeta, RecordKind},
    wordbase_engine::{Engine, Event, import::ImportStarted, profile::ProfileConfig},
};

#[derive(Debug, clap::Parser)]
struct Args {
    #[arg(long)]
    db_path: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, clap::Parser)]
enum Command {
    /// View and manage profiles
    Profile {
        #[command(subcommand)]
        command: ProfileCommand,
    },
    /// View and manage dictionaries
    #[command(alias = "dict")]
    Dictionary {
        #[command(subcommand)]
        command: DictionaryCommand,
    },
    /// Deinflect some text and return its lemmas
    Deinflect {
        /// Text to deinflect
        text: String,
    },
    /// Deinflect some text and fetch records for its lemmas
    Lookup {
        /// Text to look up
        text: String,
    },
    /// Fetch records for a lemma directly
    LookupLemma {
        /// Lemma to look up
        lemma: String,
    },
    /// Manage texthooker functions
    #[command(alias = "hook")]
    Texthooker {
        #[command(subcommand)]
        command: TexthookerCommand,
    },
}

#[derive(Debug, clap::Parser)]
enum ProfileCommand {
    /// List all profiles
    Ls,
    /// Create a new profile copied from the current profile
    New {
        /// New profile name
        name: String,
    },
    /// Set a property of a profile
    Set {
        /// Profile ID to modify
        profile_id: i64,
        #[command(subcommand)]
        command: ProfileSetCommand,
    },
    /// Delete a profile with the given ID
    Rm {
        /// Profile ID
        id: i64,
    },
}

#[derive(Debug, clap::Parser)]
enum ProfileSetCommand {
    /// Mark this profile as the current profile
    Current,
    /// Set the name of a profile
    Name {
        /// New profile name, or none to unset
        name: Option<String>,
    },
    /// Set which Anki deck a profile will add notes to
    AnkiDeck {
        /// Anki deck name
        deck: Option<String>,
    },
    /// Set which Anki model is used for creating notes
    AnkiModel {
        /// Anki model name
        model: Option<String>,
    },
}

#[derive(Debug, clap::Parser)]
enum DictionaryCommand {
    /// List all dictionaries
    Ls,
    /// Get info on a specific dictionary
    Info {
        /// Dictionary ID
        id: i64,
    },
    /// Import a dictionary file from the filesystem
    Import {
        /// Path to the dictionary file
        path: PathBuf,
    },
    /// Enable a dictionary for the current profile
    Enable {
        /// Dictionary ID
        id: i64,
    },
    /// Disable a dictionary for the current profile
    Disable {
        /// Dictionary ID
        id: i64,
    },
    /// Modify the state of a dictionary
    Set {
        #[command(subcommand)]
        command: DictionarySetCommand,
    },
    /// Delete a dictionary with the given ID
    Rm {
        /// Dictionary ID
        id: i64,
    },
}

#[derive(Debug, clap::Parser)]
enum DictionarySetCommand {
    /// Set the sorting position of a dictionary
    Position {
        /// Dictionary ID
        id: i64,
        /// New dictionary position
        position: i64,
    },
    /// Set which dictionary is used for sorting by term frequency
    Sorting {
        /// Dictionary ID, or none to unset
        id: Option<i64>,
    },
}

#[derive(Debug, clap::Parser)]
enum TexthookerCommand {
    /// Get the texthooker pull server URL
    GetUrl,
    /// Set the texthooker pull server URL
    SetUrl {
        /// Server URL, should start with `ws://`
        url: String,
    },
    /// Print incoming texthooker sentences from the pull server
    Watch,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .without_time()
        .init();
    let args = <Args as clap::Parser>::parse();

    let db_path = if let Some(db_path) = args.db_path {
        db_path
    } else {
        ProjectDirs::from("io.github", "aecsocket", "Wordbase")
            .context("failed to get default app directories")?
            .data_dir()
            .join("wordbase.db")
    };
    info!("Using {db_path:?} as database path");

    let engine = Engine::new(db_path)
        .await
        .context("failed to create engine")?;

    match args.command {
        Command::Profile {
            command: ProfileCommand::Ls,
        } => {
            profile_ls(&engine);
        }
        Command::Profile {
            command: ProfileCommand::New { name },
        } => profile_new(&engine, name).await?,
        Command::Profile {
            command:
                ProfileCommand::Set {
                    profile_id,
                    command: ProfileSetCommand::Current,
                },
        } => profile_set_current(&engine, profile_id).await?,
        Command::Profile {
            command:
                ProfileCommand::Set {
                    profile_id,
                    command: ProfileSetCommand::Name { name },
                },
        } => profile_set_name(&engine, profile_id, name).await?,
        Command::Profile {
            command:
                ProfileCommand::Set {
                    profile_id,
                    command: ProfileSetCommand::AnkiDeck { deck },
                },
        } => profile_set_anki_deck(&engine, profile_id, deck).await?,
        Command::Profile {
            command:
                ProfileCommand::Set {
                    profile_id,
                    command: ProfileSetCommand::AnkiModel { model },
                },
        } => profile_set_anki_model(&engine, profile_id, model).await?,
        Command::Profile {
            command: ProfileCommand::Rm { id },
        } => profile_rm(&engine, id).await?,
        Command::Dictionary {
            command: DictionaryCommand::Ls,
        } => dictionary_ls(&engine),
        Command::Dictionary {
            command: DictionaryCommand::Info { id },
        } => dictionary_info(&engine, id)?,
        Command::Dictionary {
            command: DictionaryCommand::Import { path },
        } => dictionary_import(&engine, path).await?,
        Command::Dictionary {
            command: DictionaryCommand::Enable { id },
        } => dictionary_enable(&engine, id).await?,
        Command::Dictionary {
            command: DictionaryCommand::Disable { id },
        } => dictionary_disable(&engine, id).await?,
        Command::Dictionary {
            command:
                DictionaryCommand::Set {
                    command: DictionarySetCommand::Position { id, position },
                },
        } => dictionary_set_position(&engine, id, position).await?,
        Command::Dictionary {
            command:
                DictionaryCommand::Set {
                    command: DictionarySetCommand::Sorting { id },
                },
        } => dictionary_set_sorting(&engine, id).await?,
        Command::Dictionary {
            command: DictionaryCommand::Rm { id },
        } => dictionary_rm(&engine, id).await?,
        Command::Deinflect { text } => deinflect(&engine, text),
        Command::Lookup { text } => lookup(&engine, text).await?,
        Command::LookupLemma { lemma } => lookup_lemma(&engine, lemma).await?,
        Command::Texthooker {
            command: TexthookerCommand::GetUrl,
        } => texthooker_get_url(&engine),
        Command::Texthooker {
            command: TexthookerCommand::SetUrl { url },
        } => texthooker_set_url(&engine, url).await?,
        Command::Texthooker {
            command: TexthookerCommand::Watch,
        } => texthooker_watch(&engine).await?,
    }

    Ok(())
}

fn profile_ls(engine: &Engine) {
    let mut table = AsciiTable::default();
    table.column(1).set_header("ID");
    table.column(2).set_header("Name");
    table.column(3).set_header("Sorting Dict");
    table.column(4).set_header("Anki Deck");
    table.column(5).set_header("Dictionaries");

    let dicts = engine.dictionaries();
    let name_of_dict = |dict_id: DictionaryId| {
        dicts
            .by_id
            .get(&dict_id)
            .map_or_else(|| "?".into(), |dict| dict.meta.name.clone())
    };

    let profiles = engine.profiles();

    let data = profiles
        .by_id
        .values()
        .map(|profile| {
            let num_dictionaries = profile.enabled_dictionaries.len();
            let enabled_dictionaries = profile
                .enabled_dictionaries
                .iter()
                .map(|dict| name_of_dict(*dict))
                .collect::<Vec<_>>()
                .join(", ");

            let selected = if profile.id == profiles.current_id {
                "✔"
            } else {
                ""
            };
            let sorting_dictionary = profile
                .sorting_dictionary
                .map(name_of_dict)
                .unwrap_or_default();

            vec![
                selected.to_string(),
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

async fn profile_new(engine: &Engine, name: String) -> Result<()> {
    let name = NormString::new(name).context("invalid profile name")?;
    let new_id = engine
        .insert_profile(&ProfileMeta {
            name: Some(name),
            accent_color: None,
        })
        .await?;
    println!("Created profile with ID {}", new_id.0);
    Ok(())
}

async fn profile_set_current(engine: &Engine, profile_id: i64) -> Result<()> {
    let profile_id = ProfileId(profile_id);
    engine.set_current_profile(profile_id).await?;
    Ok(())
}

async fn profile_set_name(engine: &Engine, profile_id: i64, name: Option<String>) -> Result<()> {
    let name = name
        .map(|s| NormString::new(s).context("invalid profile name"))
        .transpose()?;
    let profile_id = ProfileId(profile_id);
    let profiles = engine.profiles();
    let profile = profiles
        .by_id
        .get(&profile_id)
        .context("no profile with this ID")?;
    engine
        .set_profile_meta(
            profile_id,
            &ProfileMeta {
                name,
                ..profile.meta.clone()
            },
        )
        .await?;
    Ok(())
}

async fn profile_set_anki_deck(
    engine: &Engine,
    profile_id: i64,
    deck: Option<String>,
) -> Result<()> {
    let anki_deck = deck
        .map(|s| NormString::new(s).context("invalid deck name"))
        .transpose()?;
    let profile_id = ProfileId(profile_id);
    let profiles = engine.profiles();
    let profile = profiles
        .by_id
        .get(&profile_id)
        .context("no profile with this ID")?;
    engine
        .set_profile_config(
            profile_id,
            &ProfileConfig {
                anki_deck,
                ..profile.config.clone()
            },
        )
        .await?;
    Ok(())
}

async fn profile_set_anki_model(
    engine: &Engine,
    profile_id: i64,
    model: Option<String>,
) -> Result<()> {
    let anki_model = model
        .map(|s| NormString::new(s).context("invalid model name"))
        .transpose()?;
    let profile_id = ProfileId(profile_id);
    let profiles = engine.profiles();
    let profile = profiles
        .by_id
        .get(&profile_id)
        .context("no profile with this ID")?;
    engine
        .set_profile_config(
            profile_id,
            &ProfileConfig {
                anki_model,
                ..profile.config.clone()
            },
        )
        .await?;
    Ok(())
}

async fn profile_rm(engine: &Engine, id: i64) -> Result<()> {
    let id = ProfileId(id);
    engine.remove_profile(id).await?;
    Ok(())
}

fn dictionary_ls(engine: &Engine) {
    let mut table = AsciiTable::default();
    table.column(0).set_header("Sort");
    table.column(1).set_header("On");
    table.column(2).set_header("Pos");
    table.column(3).set_header("ID");
    table.column(4).set_header("Name");
    table.column(5).set_header("Version");

    let dictionaries = engine.dictionaries();
    let data = dictionaries
        .by_id
        .values()
        .map(|dict| {
            vec![
                (if dictionaries.sorting_id == Some(dict.id) {
                    "✔"
                } else {
                    ""
                })
                .to_string(),
                (if dict.enabled { "✔" } else { "" }).to_string(),
                format!("{}", dict.position),
                format!("{}", dict.id.0),
                dict.meta.name.clone(),
                dict.meta.version.clone().unwrap_or_default(),
            ]
        })
        .collect::<Vec<_>>();
    table.print(&data);
}

fn dictionary_info(engine: &Engine, id: i64) -> Result<()> {
    let id = DictionaryId(id);
    let dictionaries = engine.dictionaries();
    let dictionary = dictionaries
        .by_id
        .get(&id)
        .context("no dictionary with this ID")?;
    println!(
        "{:?} version {:?}",
        dictionary.meta.name, dictionary.meta.version
    );
    println!(
        "  {} | ID {} | Position {}",
        if dictionary.enabled {
            "Enabled"
        } else {
            "Disabled "
        },
        dictionary.id.0,
        dictionary.position
    );

    if let Some(url) = &dictionary.meta.url {
        println!("  URL: {url}");
    }

    if let Some(description) = &dictionary.meta.description {
        if !description.trim().is_empty() {
            println!();
            println!("--- Description ---");
            println!();
            println!("{description}");
        }
    }
    Ok(())
}

async fn dictionary_import(engine: &Engine, path: PathBuf) -> Result<()> {
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

        info!(
            "Importing {:?} version {:?}",
            tracker.meta.name, tracker.meta.version
        );

        while let Some(progress) = tracker.recv_progress.recv().await {
            info!("{:.02}% imported", progress * 100.0);
        }
    };

    let (result, ()) = tokio::join!(import_task, tracker_task);
    result
        .context("import task canceled")?
        .context("failed to import dictionary")?;

    let elapsed = Instant::now().duration_since(start);
    info!("Import complete in {elapsed:?}");
    Ok(())
}

async fn dictionary_enable(engine: &Engine, id: i64) -> Result<()> {
    let id = DictionaryId(id);
    engine.enable_dictionary(id).await?;
    Ok(())
}

async fn dictionary_disable(engine: &Engine, id: i64) -> Result<()> {
    let id = DictionaryId(id);
    engine.disable_dictionary(id).await?;
    Ok(())
}

async fn dictionary_set_position(engine: &Engine, id: i64, position: i64) -> Result<()> {
    let id = DictionaryId(id);
    engine.set_dictionary_position(id, position).await?;
    Ok(())
}

async fn dictionary_set_sorting(engine: &Engine, id: Option<i64>) -> Result<()> {
    let id = id.map(DictionaryId);
    engine.set_sorting_dictionary(id).await?;
    Ok(())
}

async fn dictionary_rm(engine: &Engine, id: i64) -> Result<()> {
    let id = DictionaryId(id);
    engine.remove_dictionary(id).await?;
    Ok(())
}

fn deinflect(engine: &Engine, text: String) {
    for deinflection in engine.deinflect(&text) {
        let scan_len = deinflection.scan_len;
        let text_part = text.get(..scan_len).map_or_else(
            || format!("(invalid scan len {scan_len})"),
            ToOwned::to_owned,
        );
        let lemma = deinflection.lemma;
        println!("{text_part:?} -> {:?}", &*lemma);
    }
}

async fn lookup(engine: &Engine, text: String) -> Result<()> {
    for result in engine.lookup(&text, 0, RecordKind::ALL).await? {
        println!("{result:#?}");
    }
    Ok(())
}

async fn lookup_lemma(engine: &Engine, lemma: String) -> Result<()> {
    for result in engine.lookup_lemma(&lemma, RecordKind::ALL).await? {
        println!("{result:#?}");
    }
    Ok(())
}

fn texthooker_get_url(engine: &Engine) {
    let url = engine.texthooker_url();
    println!("{url}");
}

async fn texthooker_set_url(engine: &Engine, url: String) -> Result<()> {
    engine.set_texthooker_url(url).await?;
    Ok(())
}

async fn texthooker_watch(engine: &Engine) -> Result<()> {
    println!("Watching for texthooker sentences");
    let mut recv_event = engine.recv_event();
    loop {
        let event = recv_event.recv().await.context("event channel closed")?;
        match event {
            Event::PullTexthookerConnected => {
                println!("Connected");
            }
            Event::PullTexthookerDisconnected => {
                println!("Disconnected");
            }
            Event::TexthookerSentence(sentence) => {
                println!("{sentence:?}");
            }
        }
    }
}
