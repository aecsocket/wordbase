#![expect(missing_docs, reason = "example file")]

use {
    ascii_table::AsciiTable,
    eyre::{Context, Result, eyre},
    futures::StreamExt,
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
        codec,
        import::{self, FinishImport, ImportEvent},
        storage::{self, ImportStorage, LookupStorage, Lookups},
    },
};

#[derive(clap::Parser)]
struct Args {
    archive: PathBuf,
    lemma: String,
}

fn main() -> Result<()> {
    let args = <Args as clap::Parser>::parse();

    tracing_subscriber::fmt()
        .with_writer(io::stderr)
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::DEBUG.into())
                .from_env_lossy(),
        )
        .without_time()
        .init();

    let temp_dir = tempfile::tempdir().wrap_err("failed to create temp dir")?;
    info!("Using {:?} as temporary dir", temp_dir.path());

    let mut results = Vec::new();
    let mut cx = BenchContext {
        temp_dir: &temp_dir,
        lemma: &args.lemma,
        archive: &args.archive,
        results: &mut results,
    };

    bench_db(&mut cx, "redb+rmp", || {
        Ok(storage::Redb::<codec::Rmp>::new())
    })?;
    bench_db(&mut cx, "redb+rkyv", || {
        Ok(storage::Redb::<codec::Rkyv>::new())
    })?;

    bench_db(&mut cx, "heed+rmp", || {
        Ok(storage::Heed::<codec::Rmp>::new())
    })?;
    bench_db(&mut cx, "heed+rkyv", || {
        Ok(storage::Heed::<codec::Rkyv>::new())
    })?;

    bench_db(&mut cx, "rocksdb+rmp", || {
        storage::RocksDb::<codec::Rmp>::new()
    })?;
    bench_db(&mut cx, "rocksdb+rkyv", || {
        storage::RocksDb::<codec::Rkyv>::new()
    })?;

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

fn bench_db<S: storage::Storage>(
    cx: &mut BenchContext,
    name: &str,
    make_storage: impl FnOnce() -> Result<S>,
) -> Result<()> {
    const LOOKUP_ITERS: usize = 10_000;

    let dictionary_dir = cx.temp_dir.path().join(name);
    fs::create_dir_all(&dictionary_dir)
        .wrap_err_with(|| eyre!("failed to create directory {dictionary_dir:?}"))?;
    info!("Benchmarking `{name}` at {dictionary_dir:?}");

    (|| {
        let start = Instant::now();
        let storage = make_storage().wrap_err("failed to make storage")?;
        let mut import_storage = storage
            .begin_import(&dictionary_dir)
            .wrap_err("failed to begin import")?;

        let (tx_progress, _) = async_channel::bounded(1);
        let (meta, import) = import::yomitan::start(cx.archive)?;
        info!("{meta:?}");
        import.finish(&mut import_storage, tx_progress)?;

        let lookup_storage = import_storage
            .open_lookups()
            .wrap_err("failed to open storage for lookups")?;

        let import_time = start.elapsed();
        let import_time = format!("{import_time:.2?}");

        let storage_size = get_size(&dictionary_dir).wrap_err("failed to get storage size")?;
        let storage_size = format_size(storage_size, DECIMAL);
        info!(
            "Import stats:
    - elapsed: {import_time}
    - storage size: {storage_size}",
        );

        let lookups = lookup_storage
            .lookups()
            .wrap_err("failed to open lookups")?;

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
