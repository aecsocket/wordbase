#![doc = include_str!("../README.md")]

use {
    eyre::{Context, OptionExt, Result},
    std::{io, path::PathBuf, time::Instant},
    tracing::{info, level_filters::LevelFilter},
    tracing_subscriber::EnvFilter,
    wordbase::{DictionaryId, import::FinishImport},
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

fn main() -> Result<()> {
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

    match args.command {
        Command::Dict {
            command: DictCommand::Import { path },
        } => {
            let dictionaries_dir = data_dir.join("dictionaries");
            std::fs::create_dir_all(&dictionaries_dir)
                .wrap_err("failed to create dictionaries directory")?;
            let storage = db::redb::Storage { dictionaries_dir };
            let storage = storage.begin_import(DictionaryId::random())?;

            let start = Instant::now();
            wordbase::import::yomitan::start(&path)?.finish(storage)?;
            info!("Finished in {:?}", start.elapsed());
        }
        Command::Lookup { lemma } => {}
    }
    Ok(())
}
