#![doc = include_str!("../README.md")]

use {
    eyre::{Context, OptionExt, Result},
    std::{io, path::PathBuf, str::FromStr, time::Instant},
    tokio::fs,
    tracing::{info, level_filters::LevelFilter},
    tracing_subscriber::EnvFilter,
    wordbase_engine::{Dictionaries, DictionaryId},
};

#[derive(Debug, Clone, clap::Parser)]
struct Args {
    #[arg(long)]
    data_dir: Option<PathBuf>,
    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Clone, clap::Subcommand)]
enum Command {
    Dict {
        #[command(subcommand)]
        command: DictCommand,
    },
    Lookup {
        lemma: String,
    },
}

#[derive(Debug, Clone, clap::Subcommand)]
enum DictCommand {
    Ls,
    Import { path: PathBuf },
    Rm { id: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(io::stderr)
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .without_time()
        .init();
    let args = <Args as clap::Parser>::parse();

    let data_dir = if let Some(data_dir) = args.data_dir {
        data_dir
    } else {
        wordbase_desktop::data_dir().ok_or_eyre("failed to get default data directory")?
    };
    fs::create_dir_all(&data_dir)
        .await
        .wrap_err("failed to create data directory")?;

    let dicts_dir = data_dir.join("dictionaries");
    fs::create_dir_all(&dicts_dir)
        .await
        .wrap_err("failed to create dictionaries directory")?;

    let mut dictionaries = Dictionaries::new(&dicts_dir).await?;

    match args.command {
        Command::Dict {
            command: DictCommand::Ls,
        } => {
            info!("Dictionaries ({})", dictionaries.list().len());
            for dict in dictionaries.list() {
                info!(
                    "- {}: {} v{:?}",
                    dict.state.id.0.hyphenated(),
                    dict.state.meta.name,
                    dict.state.meta.version,
                );
            }
        }
        Command::Dict {
            command: DictCommand::Import { path },
        } => {
            let start = Instant::now();
            dictionaries.import(&path).await?;
            info!("Imported in {:?}", start.elapsed());
        }
        Command::Dict {
            command: DictCommand::Rm { id },
        } => {
            let id = id.parse::<DictionaryId>()?;
            let start = Instant::now();
            dictionaries.remove(id).await?;
            info!("Removed in {:?}", start.elapsed());
        }
        Command::Lookup { lemma } => {
            for row in dictionaries.lookup(&lemma).await? {
                info!("{row:?}");
            }
        }
    }
    Ok(())
}
