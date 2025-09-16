#![expect(missing_docs, reason = "example file")]

use {
    ascii_table::AsciiTable,
    eyre::{Context, Result},
    humansize::{DECIMAL, format_size},
    std::{
        io,
        path::{Path, PathBuf},
        time::Instant,
    },
    tempfile::TempDir,
    tracing::{info, level_filters::LevelFilter},
    tracing_subscriber::EnvFilter,
    wordbase::{
        DictionaryId, db,
        import::{self, FinishImport, OpenArchive},
        uuid::Uuid,
    },
};

#[derive(clap::Parser)]
struct Args {
    archive: PathBuf,
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

    let archive = args.archive;
    let mut results = Vec::new();

    info!("Importing into redb");
    bench_db(&temp_dir, &archive, &mut results, "redb", |path| {
        Ok(db::redb::Storage::new(path))
    })
    .wrap_err("redb import failed")?;

    info!("Importing into heed");
    bench_db(&temp_dir, &archive, &mut results, "heed", |path| {
        Ok(db::heed::Storage::new(path))
    })
    .wrap_err("heed import failed")?;

    info!("Importing into rocksdb");
    bench_db(&temp_dir, &archive, &mut results, "rocksdb", |path| {
        Ok(db::rocksdb::Storage::new(path)?)
    })
    .wrap_err("rocksdb import failed")?;

    let mut table = AsciiTable::default();
    table.column(0).set_header("DB type");
    table.column(1).set_header("Elapsed");
    table.column(2).set_header("Storage size");
    info!("Results:\n{}", table.format(&results));

    Ok(())
}

fn bench_db<S: db::Storage>(
    temp_dir: &TempDir,
    open_archive: &impl OpenArchive,
    results: &mut Vec<Vec<String>>,
    name: &str,
    make_storage: impl FnOnce(&Path) -> Result<S>,
) -> Result<()> {
    const DICTIONARY_ID: DictionaryId = DictionaryId(Uuid::nil());

    let start = Instant::now();

    let path = temp_dir.path().join(name);
    let storage = make_storage(&path).wrap_err("failed to make storage")?;
    let storage = storage
        .begin_import(DICTIONARY_ID)
        .wrap_err("failed to begin import")?;

    import::yomitan::start(open_archive)
        .wrap_err("failed to start import")?
        .finish(storage)
        .wrap_err("failed to finish import")?;

    let elapsed = start.elapsed();
    let elapsed = format!("{elapsed:.2?}");

    let storage_size = get_size(&path).wrap_err("failed to get storage size")?;
    let storage_size = format_size(storage_size, DECIMAL);
    info!(
        "Imported stats:
- elapsed: {elapsed}
- storage size: {storage_size}",
    );

    results.push(vec![name.into(), elapsed, storage_size]);
    Ok(())
}

fn get_size(path: impl AsRef<Path>) -> std::io::Result<u64> {
    let metadata = path.as_ref().symlink_metadata()?;
    if metadata.is_dir() {
        Ok(std::fs::read_dir(path)?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                Some(get_size(&entry.path()).ok()?)
            })
            .sum())
    } else {
        Ok(metadata.len())
    }
}
