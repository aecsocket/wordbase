#![doc = include_str!("../README.md")]

mod anki;
mod dict;
mod lookup;
mod profile;
mod query;

use {
    anyhow::{Context, Result, bail},
    serde::Serialize,
    std::{io, path::PathBuf},
    tracing::level_filters::LevelFilter,
    tracing_subscriber::EnvFilter,
    wordbase::{DictionaryId, Engine, ProfileId},
};

#[derive(Debug, clap::Parser)]
struct Args {
    /// Wordbase engine data directory.
    ///
    /// Defaults to the desktop data directory.
    #[arg(long)]
    data_dir: Option<PathBuf>,
    /// ID of the profile to use for commands.
    ///
    /// If there is only 1 profile present, this may be omitted.
    #[arg(long, short)]
    profile: Option<i64>,
    /// Output format printed to stdout.
    ///
    /// If not specified, nothing will be output to stdout. Log messages will be
    /// output to stderr regardless of this option.
    #[arg(long, short)]
    output: Option<OutputFormat>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum OutputFormat {
    /// JSON format.
    Json,
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
    /// Fetch records for some text and render the results as HTML
    Render {
        /// Text to look up
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
    /// Swap positions of two dictionaries
    Swap {
        /// First dictionary ID
        a_id: i64,
        /// Second dictionary ID
        b_id: i64,
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
    /// Build and output an Anki note for the given term
    Note {
        headword: String,
        #[arg(long, short)]
        sentence: Option<String>,
        #[arg(long, short)]
        reading: Option<String>,
    },
}

#[tokio::main(flavor = "multi_thread")]
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
        wordbase::data_dir()?
    };

    let engine = Engine::new(data_dir)
        .await
        .context("failed to create engine")?;
    let require_profile = || {
        if let Some(profile_id) = args.profile {
            engine
                .profiles()
                .get(&ProfileId(profile_id))
                .cloned()
                .with_context(|| format!("no profile with ID {profile_id}"))
        } else {
            let profiles = engine.profiles();
            match (profiles.len(), profiles.first()) {
                (1, Some((_, profile))) => Ok(profile.clone()),
                (_, _) => bail!(
                    "more than 1 profile exists - you must explicitly specify which profile to \
                     use using `--profile [id]`"
                ),
            }
        }
    };

    match args.command {
        // lookup
        Command::Lookup { text } => output(
            args.output,
            lookup::lookup(&engine, &*require_profile()?, &text).await?,
        ),
        // query
        Command::LookupLemma { lemma } => output(
            args.output,
            query::lookup_lemma(&engine, &*require_profile()?, &lemma).await?,
        ),
        Command::Render { text } => {
            query::render(&engine, &*require_profile()?, &text).await?;
        }
        Command::Deinflect { text } => {
            query::deinflect(&engine, &text);
        }
        // profile
        Command::Profile {
            command: ProfileCommand::Ls,
        } => output(args.output, profile::ls(&engine)),
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
        } => output(args.output, dict::ls(&engine, &*require_profile()?)),
        Command::Dict {
            command: DictCommand::Info { dict_id },
        } => dict::info(&engine, DictionaryId(dict_id))?,
        Command::Dict {
            command: DictCommand::Import { path },
        } => dict::import(&engine, &*require_profile()?, path).await?,
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
            command: DictCommand::Swap { a_id, b_id },
        } => dict::swap_positions(&engine, DictionaryId(a_id), DictionaryId(b_id)).await?,
        Command::Dict {
            command: DictCommand::Rm { dict_id },
        } => dict::rm(&engine, DictionaryId(dict_id)).await?,
        Command::Anki {
            command:
                AnkiCommand::Note {
                    sentence,
                    headword,
                    reading,
                },
        } => output(
            args.output,
            anki::note(
                &engine,
                &*require_profile()?,
                &headword,
                sentence.as_deref(),
                reading.as_deref(),
            )
            .await?,
        ),
    }

    Ok(())
}

fn output<T: Serialize + 'static>(output: Option<OutputFormat>, t: T) {
    match output {
        Some(OutputFormat::Json) => {
            _ = serde_json::to_writer(io::stdout(), &t);
        }
        None => {}
    }
}
