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
    tokio::fs,
    tracing::{debug, info, level_filters::LevelFilter},
    tracing_subscriber::EnvFilter,
    wordbase_storage::{backend, codec},
};

#[derive(clap::Parser)]
struct Args {
    archive: PathBuf,
    lemma: String,
}

#[tokio::main]
async fn main() -> Result<()> {
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
    // temp_dir.disable_cleanup(true);
    info!("Using {:?} as temporary dir", temp_dir.path());

    let mut results = Vec::new();
    let mut cx = BenchContext {
        temp_dir: &temp_dir,
        lemma: &args.lemma,
        archive: &args.archive,
        results: &mut results,
    };

    // bench_db::<backend::Heed, codec::Rmp>(&mut cx, "heed+rmp").await?;
    // bench_db::<backend::Heed, codec::Rkyv>(&mut cx, "heed+rkyv").await?;

    // bench_db::<backend::Libsql, codec::Rmp>(&mut cx, "libsql+rmp").await?;
    // bench_db::<backend::Libsql, codec::Rkyv>(&mut cx, "libsql+rkyv").await?;

    // bench_db::<backend::Redb, codec::Rmp>(&mut cx, "redb+rmp").await?;
    // bench_db::<backend::Redb, codec::Rkyv>(&mut cx, "redb+rkyv").await?;

    // bench_db::<backend::Rocksdb, codec::Rmp>(&mut cx, "rocksdb+rmp").await?;
    bench_db::<backend::Rocksdb, codec::Rkyv>(&mut cx, "rocksdb+rkyv").await?;

    // bench_db::<backend::Turso, codec::Rmp>(&mut cx, "turso+rmp").await?;
    // bench_db::<backend::Turso, codec::Rkyv>(&mut cx, "turso+rkyv").await?;

    let mut table = AsciiTable::default();
    table.column(0).set_header("DB type");
    table.column(1).set_header("Size");
    table.column(2).set_header("Import");
    table.column(3).set_header("Lookup");
    table.column(4).set_header("# records");
    info!("Results:\n{}", table.format(&results));

    Ok(())
}

struct BenchContext<'a> {
    temp_dir: &'a TempDir,
    lemma: &'a str,
    archive: &'a Path,
    results: &'a mut Vec<Vec<String>>,
}

async fn bench_db<B: backend::Backend, C: codec::Codec + Default>(
    cx: &mut BenchContext<'_>,
    name: &str,
) -> Result<()> {
    async move {
        const LOOKUP_ITERS: usize = 10_000;

        let data_dir = cx.temp_dir.path().join(name);
        fs::create_dir_all(&data_dir)
            .await
            .wrap_err_with(|| eyre!("failed to create directory {data_dir:?}"))?;
        info!("Benchmarking `{name}` at {data_dir:?}");

        // import
        let start = Instant::now();
        wordbase_storage::import::<B, C>(&cx.archive, &data_dir).await?;
        let import_time = start.elapsed();

        let import_time = format!("{import_time:.2?}");
        let storage_size = get_size(&data_dir).wrap_err("failed to get storage size")?;
        let storage_size = format_size(storage_size, DECIMAL);
        info!(
            "Import stats:
    - elapsed: {import_time}
    - storage size: {storage_size}",
        );

        // lookups

        let (_, lookups) = wordbase_storage::open::<B, C>(&data_dir, C::default())
            .await
            .wrap_err("failed to open dictionary for lookups")?;

        info!("Benchmarking lookups");
        let start = Instant::now();
        for i in 0..LOOKUP_ITERS {
            lookups
                .lookup(cx.lemma)
                .wrap_err("failed to fetch records")?;

            if i % 1000 == 0 {
                debug!("Performed {i} lookups");
            }
        }
        let lookup_time = start.elapsed();
        let lookup_time = format!("{lookup_time:.2?}");

        let records = lookups
            .lookup(cx.lemma)
            .wrap_err("failed to fetch records")?;
        info!(
            "Looked up {LOOKUP_ITERS} times in {lookup_time}, with {} records",
            records.len()
        );

        cx.results.push(vec![
            name.into(),
            storage_size,
            import_time,
            lookup_time,
            records.len().to_string(),
        ]);
        eyre::Ok(())
    }
    .await
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
