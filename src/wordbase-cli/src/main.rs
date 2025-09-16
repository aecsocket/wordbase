#![doc = include_str!("../README.md")]

use {
    eyre::{Context, OptionExt, Result},
    std::{io, path::PathBuf, time::Instant},
    tracing::{info, level_filters::LevelFilter},
    tracing_subscriber::EnvFilter,
    wordbase::{
        DictionaryId,
        db::{self, Storage},
        import::FinishImport,
    },
};

#[derive(clap::Parser)]
struct Args {
    #[clap(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    Import { path: PathBuf },
    Lookup { lemma: String },
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

    let data_dir =
        wordbase_desktop::data_dir().ok_or_eyre("failed to get default data directory")?;

    match args.command {
        Command::Import { path } => {
            let dictionaries_dir = data_dir.join("dictionaries");
            std::fs::create_dir_all(&dictionaries_dir)
                .wrap_err("failed to create dictionaries directory")?;
            let storage = db::redb::Storage { dictionaries_dir };
            let storage = storage.begin_import(DictionaryId::random())?;

            let start = Instant::now();
            wordbase::import::yomitan::start(&path)?.finish(storage)?;
            info!("Finished in {:?}", start.elapsed());
        }
        Command::Lookup { lemma } => {
            const ITERS: u32 = 10_000;

            // {
            //     let lookups =
            // Lookups::new(&data_dir.join("dictionary_rkyv.redb"))?;

            //     let start = Instant::now();
            //     for i in 0..ITERS {
            //         for record in
            // lookups.lookup_lemma_rkyv(&lemma)?.unsorted() {
            //             std::hint::black_box(record);
            //         }
            //         if i % 1000 == 0 {
            //             tracing::info!("{i}");
            //         }
            //     }
            //     tracing::info!("rkyv access: {:?}", start.elapsed());

            //     let start = Instant::now();
            //     for i in 0..ITERS {
            //         for record in
            // lookups.lookup_lemma_rkyv(&lemma)?.unsorted() {
            //             std::hint::black_box(record.deserialize());
            //         }
            //         if i % 1000 == 0 {
            //             tracing::info!("{i}");
            //         }
            //     }
            //     tracing::info!("rkyv deserialize: {:?}", start.elapsed());
            // }

            // {
            //     let lookups =
            // Lookups::new(&data_dir.join("dictionary_rmp.redb"))?;

            //     let start = Instant::now();
            //     for i in 0..ITERS {
            //         for record in lookups.lookup_lemma_rmp(&lemma)? {
            //             let record = record?;
            //             std::hint::black_box(record);
            //         }
            //         if i % 1000 == 0 {
            //             tracing::info!("{i}");
            //         }
            //     }
            //     tracing::info!("rmp deserialize: {:?}", start.elapsed());
            // }
        }
    }
    Ok(())
}
