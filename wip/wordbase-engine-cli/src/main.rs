#![doc = include_str!("../README.md")]

use {
    anyhow::{Context as _, Result},
    ascii_table::AsciiTable,
    bytes::Bytes,
    directories::ProjectDirs,
    std::{collections::HashMap, path::PathBuf, time::Instant},
    tokio::{fs, sync::oneshot},
    tracing::{info, level_filters::LevelFilter},
    tracing_subscriber::EnvFilter,
    wordbase::{DictionaryId, ProfileId, ProfileMeta, RecordKind},
    wordbase_engine::{Config, Engine, Event, import::ImportTracker},
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
    #[command(alias = "dic")]
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
    /// Mark a profile as the current profile
    Set {
        /// Profile ID to mark as current
        id: i64,
    },
    /// Delete a profile with the given ID
    Rm {
        /// Profile ID
        id: i64,
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
    /// Set the position of a dictionary in the ordering
    Position {
        /// Dictionary ID
        id: i64,
        /// New position
        position: i64,
    },
    /// Delete a dictionary with the given ID
    Rm {
        /// Dictionary ID
        id: i64,
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

#[tokio::main]
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
            .context("failed to get config dir")?
            .config_dir()
            .join("wordbase.db")
    };
    info!("Using {db_path:?} as database path");

    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent)
            .await
            .context("failed to create parent directories")?;
    }

    let (engine, engine_task) = Engine::new(&Config {
        db_path,
        max_db_connections: 8,
        max_concurrent_imports: 4,
    })
    .await
    .context("failed to create engine")?;
    let engine_task = async move { engine_task.await.expect("engine error") };

    match args.command {
        Command::Profile {
            command: ProfileCommand::Ls,
        } => profile_ls(engine).await?,
        Command::Profile {
            command: ProfileCommand::New { name },
        } => profile_new(engine, name).await?,
        Command::Profile {
            command: ProfileCommand::Set { id },
        } => profile_set(engine, id).await?,
        Command::Profile {
            command: ProfileCommand::Rm { id },
        } => profile_rm(engine, id).await?,
        Command::Dictionary {
            command: DictionaryCommand::Ls,
        } => dictionary_ls(engine).await?,
        Command::Dictionary {
            command: DictionaryCommand::Info { id },
        } => dictionary_info(engine, id).await?,
        Command::Dictionary {
            command: DictionaryCommand::Import { path },
        } => dictionary_import(engine, path).await?,
        Command::Dictionary {
            command: DictionaryCommand::Enable { id },
        } => dictionary_enable(engine, id).await?,
        Command::Dictionary {
            command: DictionaryCommand::Disable { id },
        } => dictionary_disable(engine, id).await?,
        Command::Dictionary {
            command: DictionaryCommand::Position { id, position },
        } => dictionary_position(engine, id, position).await?,
        Command::Dictionary {
            command: DictionaryCommand::Rm { id },
        } => dictionary_rm(engine, id).await?,
        Command::Deinflect { text } => deinflect(engine, text).await?,
        Command::Lookup { text } => lookup(engine, text).await?,
        Command::Texthooker {
            command: TexthookerCommand::GetUrl,
        } => {
            texthooker_get_url(engine).await?;
        }
        Command::Texthooker {
            command: TexthookerCommand::SetUrl { url },
        } => {
            texthooker_set_url(engine, url).await?;
        }
        Command::Texthooker {
            command: TexthookerCommand::Watch,
        } => {
            tokio::spawn(engine_task);
            texthooker_watch(engine).await?;
        }
    }

    Ok(())
}

async fn profile_ls(engine: Engine) -> Result<()> {
    let mut table = AsciiTable::default();
    table.column(1).set_header("ID");
    table.column(2).set_header("Name");
    table.column(3).set_header("Dictionaries");

    let current_profile_id = engine.current_profile().await?;
    let dictionaries = engine
        .dictionaries()
        .await?
        .into_iter()
        .map(|dictionary| (dictionary.id, dictionary))
        .collect::<HashMap<_, _>>();
    let data = engine
        .profiles()
        .await?
        .into_iter()
        .map(|profile| {
            let num_dictionaries = profile.enabled_dictionaries.len();
            let enabled_dictionaries = profile
                .enabled_dictionaries
                .into_iter()
                .filter_map(|dictionary_id| {
                    dictionaries
                        .get(&dictionary_id)
                        .map(|dictionary| dictionary.meta.name.as_ref())
                })
                .collect::<Vec<_>>()
                .join(", ");

            vec![
                (if profile.id == current_profile_id {
                    "✔"
                } else {
                    ""
                })
                .to_string(),
                format!("{}", profile.id.0),
                profile.meta.name.unwrap_or_else(|| "(default)".into()),
                format!("({num_dictionaries}) {enabled_dictionaries}"),
            ]
        })
        .collect::<Vec<_>>();
    table.print(&data);
    Ok(())
}

async fn profile_new(engine: Engine, name: String) -> Result<()> {
    let new_id = engine
        .insert_profile(ProfileMeta {
            name: Some(name),
            accent_color: [1.0, 1.0, 1.0],
        })
        .await?;
    println!("Created profile with ID {}", new_id.0);
    Ok(())
}

async fn profile_set(engine: Engine, id: i64) -> Result<()> {
    let id = ProfileId(id);
    engine.set_current_profile(id).await?;
    Ok(())
}

async fn profile_rm(engine: Engine, id: i64) -> Result<()> {
    let id = ProfileId(id);
    engine.delete_profile(id).await??;
    Ok(())
}

async fn dictionary_ls(engine: Engine) -> Result<()> {
    let mut table = AsciiTable::default();
    table.column(1).set_header("Pos");
    table.column(2).set_header("ID");
    table.column(3).set_header("Name");
    table.column(4).set_header("Version");

    let data = engine
        .dictionaries()
        .await?
        .into_iter()
        .map(|dictionary| {
            vec![
                (if dictionary.enabled { "✔" } else { "" }).to_string(),
                format!("{}", dictionary.position),
                format!("{}", dictionary.id.0),
                dictionary.meta.name,
                dictionary.meta.version,
            ]
        })
        .collect::<Vec<_>>();
    table.print(&data);
    Ok(())
}

async fn dictionary_info(engine: Engine, id: i64) -> Result<()> {
    let id = DictionaryId(id);
    let dictionary = engine.dictionary(id).await??;
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

    if let Some(url) = dictionary.meta.url {
        println!("  URL: {url}");
    }

    if let Some(description) = dictionary.meta.description {
        if !description.trim().is_empty() {
            println!();
            println!("--- Description ---");
            println!();
            println!("{description}");
        }
    }
    Ok(())
}

async fn dictionary_import(engine: Engine, path: PathBuf) -> Result<()> {
    let start = Instant::now();

    let data = fs::read(path)
        .await
        .map(Bytes::from)
        .context("failed to read dictionary file into memory")?;

    let (send_tracker, recv_tracker) = oneshot::channel::<ImportTracker>();
    let tracker_task = tokio::spawn(async move {
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
    });

    let (result, _) = tokio::join!(engine.import_dictionary(data, send_tracker), tracker_task);
    result.context("failed to import dictionary")?;

    let elapsed = Instant::now().duration_since(start);
    info!("Import complete in {elapsed:?}");
    Ok(())
}

async fn dictionary_enable(engine: Engine, id: i64) -> Result<()> {
    let id = DictionaryId(id);
    engine.enable_dictionary(id).await?;
    Ok(())
}

async fn dictionary_disable(engine: Engine, id: i64) -> Result<()> {
    let id = DictionaryId(id);
    engine.disable_dictionary(id).await?;
    Ok(())
}

async fn dictionary_position(engine: Engine, id: i64, position: i64) -> Result<()> {
    let id = DictionaryId(id);
    engine.set_dictionary_position(id, position).await??;
    Ok(())
}

async fn dictionary_rm(engine: Engine, id: i64) -> Result<()> {
    let id = DictionaryId(id);
    engine.delete_dictionary(id).await??;
    Ok(())
}

async fn deinflect(engine: Engine, text: String) -> Result<()> {
    let lemmas = engine.deinflect(&text).await?;
    println!("{text:?}:");
    for lemma in lemmas {
        println!("  - {lemma:?}");
    }
    Ok(())
}

async fn lookup(engine: Engine, text: String) -> Result<()> {
    let lemmas = engine
        .deinflect(&text)
        .await
        .context("failed to deinflect text")?;

    for lemma in lemmas {
        println!("{lemma:?}:");
        let records = engine.lookup_lemma(&lemma, RecordKind::ALL).await?;
        println!("{records:#?}");
        println!();
    }

    Ok(())
}

async fn texthooker_get_url(engine: Engine) -> Result<()> {
    let url = engine.texthooker_url().await?;
    println!("{url}");
    Ok(())
}

async fn texthooker_set_url(engine: Engine, url: String) -> Result<()> {
    engine.set_texthooker_url(url).await?;
    Ok(())
}

async fn texthooker_watch(engine: Engine) -> Result<()> {
    let mut recv_event = engine.recv_event();
    println!("Watching for texthooker sentences");
    loop {
        let event = recv_event.recv().await?;
        if let Event::HookSentence(sentence) = event {
            println!("{sentence:?}");
        }
    }
}
