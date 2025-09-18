#![expect(missing_docs, reason = "example file")]

use {
    ascii_table::AsciiTable,
    eyre::{Context, Result, eyre},
    humansize::{DECIMAL, format_size},
    std::{
        fs, io,
        path::{Path, PathBuf},
        time::Instant,
    },
    tempfile::TempDir,
    tracing::{debug, info, level_filters::LevelFilter},
    tracing_subscriber::EnvFilter,
    wordbase::{
        codec::{Rkyv, Rmp},
        import::{self, FinishImport},
        storage::{Heed, Lookups, Redb, RocksDb, Storage},
    },
};

#[derive(clap::Parser)]
struct Args {
    archive: PathBuf,
    lemma: String,
}

fn main() -> Result<()> {
    color_eyre::install().wrap_err("failed to install `color_eyre`")?;
    tracing_subscriber::fmt()
        .with_writer(io::stderr)
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::DEBUG.into())
                .from_env_lossy(),
        )
        .without_time()
        .init();
    let args = <Args as clap::Parser>::parse();

    let temp_dir = tempfile::tempdir().wrap_err("failed to create temp dir")?;
    info!("Using {:?} as temporary dir", temp_dir.path());

    let mut results = Vec::new();
    let mut cx = BenchContext {
        temp_dir: &temp_dir,
        lemma: &args.lemma,
        archive: &args.archive,
        results: &mut results,
    };

    bench_db::<Redb<Rmp>>(&mut cx, "redb+rmp")?;
    bench_db::<Redb<Rkyv>>(&mut cx, "redb+rkyv")?;

    bench_db::<Heed<Rmp>>(&mut cx, "heed+rmp")?;
    bench_db::<Heed<Rkyv>>(&mut cx, "heed+rkyv")?;

    bench_db::<RocksDb<Rmp>>(&mut cx, "rocksdb+rmp")?;
    bench_db::<RocksDb<Rkyv>>(&mut cx, "rocksdb+rkyv")?;

    let mut table = AsciiTable::default();
    table.column(0).set_header("DB type");
    table.column(1).set_header("Size");
    table.column(2).set_header("Import");
    table.column(3).set_header("Lookup");
    info!("Results:\n{}", table.format(&results));

    Ok(())
}

struct BenchContext<'a> {
    temp_dir: &'a TempDir,
    lemma: &'a str,
    archive: &'a Path,
    results: &'a mut Vec<Vec<String>>,
}

fn bench_db<S>(cx: &mut BenchContext, name: &str) -> Result<()>
where
    S: Storage,
    S::Codec: Default,
{
    const LOOKUP_ITERS: usize = 10_000;

    let dict_dir = cx.temp_dir.path().join(name);
    fs::create_dir_all(&dict_dir)
        .wrap_err_with(|| eyre!("failed to create directory {dict_dir:?}"))?;
    info!("Benchmarking `{name}` at {dict_dir:?}");

    (|| {
        let start = Instant::now();
        let storage = S::new().wrap_err("failed to make storage")?;
        let import_storage = storage
            .begin_import(&dict_dir)
            .wrap_err("failed to begin import")?;

        let (tx_progress, _) = async_channel::bounded(1);
        let (meta, import) = import::yomitan::start(cx.archive)?;
        info!("{meta:?}");
        import.finish(import_storage, tx_progress)?;

        let import_time = start.elapsed();
        let import_time = format!("{import_time:.2?}");

        let storage_size = get_size(&dict_dir).wrap_err("failed to get storage size")?;
        let storage_size = format_size(storage_size, DECIMAL);
        info!(
            "Import stats:
    - elapsed: {import_time}
    - storage size: {storage_size}",
        );

        let lookups = storage
            .open(&dict_dir)
            .wrap_err("failed to open storage for lookups")?;

        let start = Instant::now();
        for i in 0..LOOKUP_ITERS {
            lookups
                .lookup_lemma(cx.lemma)
                .wrap_err("failed to fetch records")?;

            if i % 1000 == 0 {
                debug!("Performed {i} lookups");
            }
        }
        let lookup_time = start.elapsed();
        let lookup_time = format!("{lookup_time:.2?}");

        let records = lookups
            .lookup_lemma(cx.lemma)
            .wrap_err("failed to fetch records")?;
        info!(
            "Looked up {LOOKUP_ITERS} times in {lookup_time}, with {} records",
            records.len()
        );

        cx.results
            .push(vec![name.into(), storage_size, import_time, lookup_time]);
        eyre::Ok(())
    })()
    .wrap_err_with(|| eyre!("failed to benchmark `{name}`"))
}

fn get_size(path: impl AsRef<Path>) -> std::io::Result<u64> {
    let metadata = path.as_ref().symlink_metadata()?;
    if metadata.is_dir() {
        Ok(std::fs::read_dir(path)?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                get_size(entry.path()).ok()
            })
            .sum())
    } else {
        Ok(metadata.len())
    }
}
