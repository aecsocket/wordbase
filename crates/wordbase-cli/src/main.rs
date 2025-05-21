#![doc = include_str!("../README.md")]

mod anki;
mod dict;
mod lookup;
mod profile;

use {
    anyhow::{Context, Result},
    std::path::PathBuf,
    tracing::level_filters::LevelFilter,
    tracing_subscriber::EnvFilter,
    wordbase::{DictionaryId, Engine, ProfileId},
};

#[derive(Debug, clap::Parser)]
struct Args {
    #[arg(long)]
    data_dir: Option<PathBuf>,
    #[arg(long, short)]
    profile: Option<i64>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, clap::Parser)]
enum Command {
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
    /// Deinflect some text and return its lemmas
    Deinflect {
        /// Text to deinflect
        text: String,
    },
    /// Manage profiles
    Profile {
        #[command(subcommand)]
        command: ProfileCommand,
    },
    /// Manage dictionaries
    Dict {
        #[command(subcommand)]
        command: DictCommand,
    },
    /// Manage AnkiConnect functions
    Anki {
        #[command(subcommand)]
        command: AnkiCommand,
    },
    // /// Manage texthooker functions
    // #[command(alias = "hook")]
    // Texthooker {
    //     #[command(subcommand)]
    //     command: TexthookerCommand,
    // },
}

#[derive(Debug, clap::Parser)]
enum ProfileCommand {
    /// List all profiles
    Ls,
    /// Create a new profile copied from the selected profile
    Copy {
        /// New profile name
        name: String,
    },
    /// Get info for the selected profile
    Info,
    /// Set a property of the selected profile
    Set {
        #[command(subcommand)]
        command: ProfileSetCommand,
    },
    /// Delete the selected profile
    Rm,
}

#[derive(Debug, clap::Parser)]
enum ProfileSetCommand {
    /// Set the human-readable profile display name
    Name {
        /// New profile name, or none to unset (default name)
        name: Option<String>,
    },
}

#[derive(Debug, clap::Parser)]
enum DictCommand {
    /// List all dictionaries
    Ls,
    /// Get info on a specific dictionary
    Info {
        /// Dictionary ID
        dict_id: i64,
    },
    /// Import a dictionary file from the filesystem
    Import {
        /// Path to the dictionary file
        path: PathBuf,
    },
    /// Modify the state of a dictionary
    Set {
        /// Dictionary ID
        dict_id: i64,
        #[command(subcommand)]
        command: DictSetCommand,
    },
    /// Delete a dictionary with the given ID
    Rm {
        /// Dictionary ID
        dict_id: i64,
    },
}

#[derive(Debug, clap::Parser)]
enum DictSetCommand {
    /// Enable the dictionary for the selected profile
    Enabled,
    /// Disable the dictionary for the selected profile
    Disabled,
}

#[derive(Debug, clap::Parser)]
enum AnkiCommand {
    Set {
        #[command(subcommand)]
        command: AnkiSetCommand,
    },
    /// Create an Anki note for the given term
    CreateNote {
        sentence: String,
        headword: String,
        reading: String,
    },
}

#[derive(Debug, clap::Parser)]
enum AnkiSetCommand {
    /// Set the AnkiConnect server URL
    Url {
        /// Server URL, should start with `http://`
        url: String,
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

    let data_dir = if let Some(data_dir) = args.data_dir {
        data_dir
    } else {
        wordbase::data_dir()?
    };

    let engine = Engine::new(data_dir)
        .await
        .context("failed to create engine")?;
    let require_profile = || {
        let selected_id = args.profile.context("profile ID not specified")?;
        engine
            .profiles()
            .get(&ProfileId(selected_id))
            .cloned()
            .with_context(|| format!("no profile with ID {selected_id}"))
    };

    match args.command {
        // lookup
        Command::Lookup { text } => lookup::lookup(&engine, &*require_profile()?, &text).await?,
        Command::LookupLemma { lemma } => {
            lookup::lookup_lemma(&engine, &*require_profile()?, &lemma).await?;
        }
        Command::Deinflect { text } => {
            lookup::deinflect(&engine, &text);
        }
        // profile
        Command::Profile {
            command: ProfileCommand::Ls,
        } => profile::ls(&engine),
        Command::Profile {
            command: ProfileCommand::Copy { name },
        } => profile::copy(&engine, &*require_profile()?, name).await?,
        Command::Profile {
            command: ProfileCommand::Info,
        } => profile::info(&engine, &*require_profile()?),
        Command::Profile {
            command:
                ProfileCommand::Set {
                    command: ProfileSetCommand::Name { name },
                },
        } => profile::set_name(&engine, &*require_profile()?, name).await?,
        Command::Profile {
            command: ProfileCommand::Rm,
        } => profile::rm(&engine, &*require_profile()?).await?,
        // dictionary
        Command::Dict {
            command: DictCommand::Ls,
        } => dict::ls(&engine, &*require_profile()?),
        Command::Dict {
            command: DictCommand::Info { dict_id },
        } => dict::info(&engine, DictionaryId(dict_id))?,
        Command::Dict {
            command: DictCommand::Import { path },
        } => dict::import(&engine, &path).await?,
        Command::Dict {
            command:
                DictCommand::Set {
                    dict_id,
                    command: DictSetCommand::Enabled,
                },
        } => dict::enable(&engine, &*require_profile()?, DictionaryId(dict_id)).await?,
        Command::Dict {
            command:
                DictCommand::Set {
                    dict_id,
                    command: DictSetCommand::Disabled,
                },
        } => dict::disable(&engine, &*require_profile()?, DictionaryId(dict_id)).await?,
        Command::Dict {
            command: DictCommand::Rm { dict_id },
        } => dict::rm(&engine, DictionaryId(dict_id)).await?,
        // anki
        Command::Anki {
            command:
                AnkiCommand::CreateNote {
                    sentence,
                    headword,
                    reading,
                },
        } => {
            anki::create_note(
                &engine,
                &*require_profile()?,
                &sentence,
                &headword,
                &reading,
            )
            .await?;
        }
        Command::Anki {
            command:
                AnkiCommand::Set {
                    command: AnkiSetCommand::Url { url },
                },
        } => {
            anki::set_url(&engine, &url).await?;
        }
    }

    Ok(())
}
