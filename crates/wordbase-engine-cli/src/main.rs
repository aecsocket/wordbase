#![doc = include_str!("../README.md")]

use {
    anyhow::{Context as _, Result},
    ascii_table::AsciiTable,
    directories::ProjectDirs,
    std::{path::PathBuf, time::Instant},
    tokio::{fs, sync::oneshot},
    tracing::{info, level_filters::LevelFilter},
    tracing_subscriber::EnvFilter,
    wordbase::DictionaryId,
    wordbase_engine::{Config, Engine, import::ImportTracker},
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
    Dictionary {
        #[command(subcommand)]
        command: DictionaryCommand,
    },
}

#[derive(Debug, clap::Parser)]
enum ProfileCommand {
    /// List all profiles
    Ls,
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

    let engine = Engine::new(&Config {
        db_path,
        max_db_connections: 8,
        max_concurrent_imports: 4,
    })
    .await
    .context("failed to create engine")?;

    match args.command {
        Command::Profile {
            command: ProfileCommand::Ls,
        } => profile_ls(engine).await?,
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
            command: DictionaryCommand::Position { id, position },
        } => dictionary_position(engine, id, position).await?,
        Command::Dictionary {
            command: DictionaryCommand::Rm { id },
        } => dictionary_rm(engine, id).await?,
    }

    Ok(())
}

async fn profile_ls(engine: Engine) -> Result<()> {
    let mut table = AsciiTable::default();
    table.column(0).set_header("ID");
    table.column(1).set_header("Name");

    let data = engine
        .profiles()
        .await?
        .into_iter()
        .map(|profile| {
            vec![
                format!("{}", profile.id.0),
                profile.meta.name.unwrap_or_default(),
            ]
        })
        .collect::<Vec<_>>();
    table.print(&data);
    Ok(())
}

async fn dictionary_ls(engine: Engine) -> Result<()> {
    let mut table = AsciiTable::default();
    table.column(0).set_header("Pos");
    table.column(1).set_header("ID");
    table.column(2).set_header("Name");
    table.column(3).set_header("Version");

    let data = engine
        .dictionaries()
        .await?
        .into_iter()
        .map(|dictionary| {
            vec![
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
        "  ID {} | Position {}",
        dictionary.id.0, dictionary.position
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

    let (result, _) = tokio::join!(engine.import_dictionary(&data, send_tracker), tracker_task);
    result.context("failed to import dictionary")?;

    let elapsed = Instant::now().duration_since(start);
    info!("Import complete in {elapsed:?}");
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
