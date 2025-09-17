#![expect(missing_docs, reason = "example file")]

use {
    ascii_table::AsciiTable,
    eyre::{Context, Result, eyre},
    humansize::{DECIMAL, format_size},
    std::{
        io,
        path::{Path, PathBuf},
        time::Instant,
    },
    tempfile::TempDir,
    tracing::{debug, info, level_filters::LevelFilter},
    tracing_subscriber::EnvFilter,
    wordbase::{
        DictionaryId, codec,
        import::{self, FinishImport, OpenArchive},
        storage::{self, ImportStorage, LookupStorage, Lookups},
        uuid::Uuid,
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
        open_archive: &args.archive,
        results: &mut results,
    };

    bench_db(&mut cx, "redb+rmp", |path| {
        Ok(storage::Redb::<codec::Rmp>::new(path))
    })?;
    bench_db(&mut cx, "redb+rkyv", |path| {
        Ok(storage::Redb::<codec::Rkyv>::new(path))
    })?;

    bench_db(&mut cx, "heed+rmp", |path| {
        Ok(storage::Heed::<codec::Rmp>::new(path))
    })?;
    bench_db(&mut cx, "heed+rkyv", |path| {
        Ok(storage::Heed::<codec::Rkyv>::new(path))
    })?;

    bench_db(&mut cx, "rocksdb+rmp", |path| {
        storage::RocksDb::<codec::Rmp>::new(path)
    })?;
    bench_db(&mut cx, "rocksdb+rkyv", |path| {
        storage::RocksDb::<codec::Rkyv>::new(path)
    })?;

    let mut table = AsciiTable::default();
    table.column(0).set_header("DB type");
    table.column(1).set_header("Size");
    table.column(2).set_header("Import");
    table.column(3).set_header("Lookup");
    info!("Results:\n{}", table.format(&results));

    Ok(())
}

struct BenchContext<'a, O> {
    temp_dir: &'a TempDir,
    lemma: &'a str,
    open_archive: &'a O,
    results: &'a mut Vec<Vec<String>>,
}

fn bench_db<O: OpenArchive, S: storage::Storage>(
    cx: &mut BenchContext<O>,
    name: &str,
    make_storage: impl FnOnce(&Path) -> Result<S>,
) -> Result<()> {
    info!("Importing into `{name}`");

    (|| {
        const DICTIONARY_ID: DictionaryId = DictionaryId(Uuid::nil());
        const LOOKUP_ITERS: usize = 10_000;

        let start = Instant::now();

        let path = cx.temp_dir.path().join(name);
        let storage = make_storage(&path).wrap_err("failed to make storage")?;
        let mut import_storage = storage
            .begin_import(DICTIONARY_ID)
            .wrap_err("failed to begin import")?;

        import::yomitan::start(cx.open_archive)
            .wrap_err("failed to start import")?
            .finish(&mut import_storage)
            .wrap_err("failed to finish import")?;
        let lookup_storage = import_storage
            .open_lookups()
            .wrap_err("failed to open storage for lookups")?;

        let import_time = start.elapsed();
        let import_time = format!("{import_time:.2?}");

        let storage_size = get_size(&path).wrap_err("failed to get storage size")?;
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
