#![doc = include_str!("../README.md")]

use {
    eyre::{Context, OptionExt, Result, bail},
    std::{io, path::PathBuf, time::Instant},
    tracing::{info, level_filters::LevelFilter},
    tracing_subscriber::EnvFilter,
    wordbase_engine::{DictionaryId, ProfileId, StorageEngine},
};

#[derive(Debug, Clone, clap::Parser)]
struct Args {
    #[arg(long)]
    data_dir: Option<PathBuf>,
    #[arg(short, long)]
    profile_id: Option<String>,
    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Clone, clap::Subcommand)]
enum Command {
    Profile {
        #[command(subcommand)]
        command: ProfileCommand,
    },
    Dict {
        #[command(subcommand)]
        command: DictCommand,
    },
    LookupLemma {
        lemma: String,
    },
}

#[derive(Debug, Clone, clap::Subcommand)]
enum ProfileCommand {
    Ls,
    Add { name: String },
    Rm { id: String },
}

#[derive(Debug, Clone, clap::Subcommand)]
enum DictCommand {
    Ls,
    Import { path: PathBuf },
    Rm { id: String },
    Enable { dictionary_id: String },
    Disable { dictionary_id: String },
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

    let storage = StorageEngine::new(&data_dir)
        .await
        .wrap_err("failed to create storage engine")?;

    let profile_id = if let Some(id) = args.profile_id {
        id.parse::<ProfileId>().wrap_err("invalid profile ID")?
    } else {
        let profiles = storage.profiles();
        match (profiles.keys().next(), profiles.len()) {
            (None, _) => bail!("no profiles"),
            (Some(id), 1) => *id,
            (Some(_), _) => bail!("multiple profiles exist; use `--profile` to select one"),
        }
    };

    match args.command {
        Command::Profile {
            command: ProfileCommand::Ls,
        } => {
            let dictionaries = storage.dictionaries().await;
            let profiles = storage.profiles();
            info!("{} profiles", profiles.len());
            for (id, profile) in profiles.iter() {
                info!(
                    "- {id}: {}",
                    profile.name.as_ref().map_or("(default)", |s| s.as_str()),
                );

                let dict_names = dictionaries
                    .iter()
                    .filter(|dict| profile.enabled_dictionaries.contains(&dict.state.id))
                    .map(|dict| &dict.state.meta.name)
                    .collect::<Vec<_>>();
                info!("  Dictionaries: {dict_names:?}");
            }
        }
        Command::Profile {
            command: ProfileCommand::Add { name },
        } => {
            storage.create_profile(&name).await?;
        }
        Command::Profile {
            command: ProfileCommand::Rm { id },
        } => {
            let id = id.parse::<ProfileId>()?;
            storage.remove_profile(id).await?;
        }
        Command::Dict {
            command: DictCommand::Ls,
        } => {
            let dictionaries = storage.dictionaries().await;
            info!("{} dictionaries", dictionaries.len());
            for dict in dictionaries.iter() {
                info!(
                    "- {}: {} ver. {:?}",
                    dict.state.id, dict.state.meta.name, dict.state.meta.version,
                );
            }
        }
        Command::Dict {
            command: DictCommand::Import { path },
        } => {
            let start = Instant::now();
            storage.import_dictionary(&path).await?;
            info!("Imported in {:?}", start.elapsed());
        }
        Command::Dict {
            command: DictCommand::Rm { id },
        } => {
            let id = id.parse::<DictionaryId>()?;
            let start = Instant::now();
            storage.remove_dictionary(id).await?;
            info!("Removed in {:?}", start.elapsed());
        }
        Command::Dict {
            command: DictCommand::Enable { dictionary_id },
        } => {
            let dictionary_id = dictionary_id.parse::<DictionaryId>()?;
            storage.enable_dictionary(profile_id, dictionary_id).await?;
        }
        Command::Dict {
            command: DictCommand::Disable { dictionary_id },
        } => {
            let dictionary_id = dictionary_id.parse::<DictionaryId>()?;
            storage
                .disable_dictionary(profile_id, dictionary_id)
                .await?;
        }
        Command::LookupLemma { lemma } => {
            for row in storage.lookup_lemma(profile_id, &lemma).await? {
                info!("{row:?}");
            }
        }
    }
    Ok(())
}
