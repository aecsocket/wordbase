#![doc = include_str!("../README.md")]
#![allow(
    clippy::unused_async,
    reason = "subcommands can choose if they are async or not"
)]

use {
    eyre::{Context, OptionExt, Result, bail, eyre},
    serde::Serialize,
    std::{io, path::PathBuf, sync::Arc},
    tracing::{info, level_filters::LevelFilter},
    tracing_subscriber::EnvFilter,
    wordbase_engine::{
        ProfileId, deinflect::Deinflectors, profiles::ProfileState, storage::EngineStorage,
    },
};

mod dictionary;
mod lookup;
mod profile;

#[derive(Debug, Clone, clap::Parser)]
struct Args {
    #[arg(long)]
    data_dir: Option<PathBuf>,
    #[arg(short, long)]
    profile_id: Option<String>,
    #[arg(short, long)]
    output: Option<OutputFormat>,
    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum OutputFormat {
    Json,
    JsonPretty,
}

#[derive(Debug, Clone, clap::Subcommand)]
enum Command {
    Profile {
        #[command(subcommand)]
        command: ProfileCommand,
    },
    #[clap(alias = "dict")]
    Dictionary {
        #[command(subcommand)]
        command: DictionaryCommand,
    },
    Lookup {
        first: String,
        second: Option<String>,
    },
    LookupLemma {
        lemma: String,
    },
    Serve {
        #[arg(short, long, default_value = "127.0.0.1:9518")]
        bind_addr: String,
    },
}

#[derive(Debug, Clone, clap::Subcommand)]
enum ProfileCommand {
    #[clap(alias = "ls")]
    List,
    Add {
        name: String,
    },
    #[clap(alias = "rm")]
    Remove {
        id: String,
    },
}

#[derive(Debug, Clone, clap::Subcommand)]
enum DictionaryCommand {
    #[clap(alias = "ls")]
    List,
    Import {
        path: PathBuf,
    },
    #[clap(alias = "rm")]
    Remove {
        id: String,
    },
    Enable {
        dictionary_id: String,
    },
    Disable {
        dictionary_id: String,
    },
}

struct App {
    storage: EngineStorage,
    profile: Arc<ProfileState>,
    profile_id: ProfileId,
}

impl App {
    pub fn deinflectors(&self) -> Deinflectors {
        _ = self;
        Deinflectors::new(vec![])
    }
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

    let storage = EngineStorage::new(&data_dir)
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
    let profile = storage
        .profiles()
        .get(&profile_id)
        .cloned()
        .ok_or_else(|| eyre!("unknown profile ID {profile_id}"))?;

    let app = App {
        storage,
        profile,
        profile_id,
    };

    let of = args.output;
    match args.command {
        Command::Profile {
            command: ProfileCommand::List,
        } => finish(of, profile::list(&app).await?),
        Command::Profile {
            command: ProfileCommand::Add { name },
        } => finish(of, profile::add(&app, &name).await?),
        Command::Profile {
            command: ProfileCommand::Remove { id },
        } => finish(of, profile::remove(&app, &id).await?),
        //
        Command::Dictionary {
            command: DictionaryCommand::List,
        } => finish(of, dictionary::list(&app).await?),
        Command::Dictionary {
            command: DictionaryCommand::Import { path },
        } => finish(of, dictionary::import(&app, &path).await?),
        Command::Dictionary {
            command: DictionaryCommand::Remove { id },
        } => finish(of, dictionary::remove(&app, &id).await?),
        Command::Dictionary {
            command: DictionaryCommand::Enable { dictionary_id },
        } => finish(of, dictionary::enable(&app, &dictionary_id).await?),
        Command::Dictionary {
            command: DictionaryCommand::Disable { dictionary_id },
        } => finish(of, dictionary::disable(&app, &dictionary_id).await?),
        //
        Command::Lookup { first, second } => {
            finish(of, lookup::sentence(&app, &first, second.as_deref()).await?);
        }
        Command::LookupLemma { lemma } => finish(of, lookup::lemma(&app, &lemma).await?),
        //
        Command::Serve { bind_addr } => {
            let deinflectors = app.deinflectors();
            info!("");
            info!("  Serving on  {bind_addr}");
            info!("        Docs  http://{bind_addr}/docs");
            info!("");
            wordbase_server_http::serve(app.storage, deinflectors, bind_addr).await?;
        }
    }
    Ok(())
}

fn checkmark(b: bool) -> &'static str {
    if b { "✔" } else { " " }
}

fn finish<T: Serialize>(output_format: Option<OutputFormat>, value: T) {
    match output_format {
        None => {}
        Some(OutputFormat::Json) => {
            _ = serde_json::to_writer(io::stdout(), &value);
        }
        Some(OutputFormat::JsonPretty) => {
            _ = serde_json::to_writer_pretty(io::stdout(), &value);
        }
    }
}
