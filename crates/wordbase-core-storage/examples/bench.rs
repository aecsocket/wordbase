#![expect(missing_docs, reason = "example file")]

use {
    ascii_table::AsciiTable,
    disqualified::ShortName,
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
    wordbase_core::{codec::Codec, dictionary, storage::Storage},
    wordbase_core_storage::{codec, storage},
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

    #[cfg(all(feature = "storage-heed", feature = "codec-rmp"))]
    bench::<storage::Heed, codec::Rmp>(&mut cx).await?;
    #[cfg(all(feature = "storage-heed", feature = "codec-rkyv"))]
    bench::<storage::Heed, codec::Rkyv>(&mut cx).await?;

    #[cfg(all(feature = "storage-libsql", feature = "codec-rmp"))]
    bench::<storage::Libsql, codec::Rmp>(&mut cx).await?;
    #[cfg(all(feature = "storage-libsql", feature = "codec-rkyv"))]
    bench::<storage::Libsql, codec::Rkyv>(&mut cx).await?;

    #[cfg(all(feature = "storage-redb", feature = "codec-rmp"))]
    bench::<storage::Redb, codec::Rmp>(&mut cx).await?;
    #[cfg(all(feature = "storage-redb", feature = "codec-rkyv"))]
    bench::<storage::Redb, codec::Rkyv>(&mut cx).await?;

    #[cfg(all(feature = "storage-rocksdb", feature = "codec-rmp"))]
    bench::<storage::Rocksdb, codec::Rmp>(&mut cx).await?;
    #[cfg(all(feature = "storage-rocksdb", feature = "codec-rkyv"))]
    bench::<storage::Rocksdb, codec::Rkyv>(&mut cx).await?;

    // #[cfg(all(feature = "storage-turso", feature = "codec-rmp"))]
    // bench::<storage::Turso, codec::Rmp>(&mut cx).await?;
    // #[cfg(all(feature = "storage-turso", feature = "codec-rkyv"))]
    // bench::<storage::Turso, codec::Rkyv>(&mut cx).await?;

    let mut table = AsciiTable::default();
    table.column(0).set_header("Storage");
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

async fn bench<S: Storage, C: Codec + Default>(cx: &mut BenchContext<'_>) -> Result<()> {
    let name = ShortName::of::<(S, C)>();

    async move {
        const LOOKUP_ITERS: usize = 10_000;

        let data_dir = cx.temp_dir.path().join(name.0);
        fs::create_dir_all(&data_dir)
            .await
            .wrap_err_with(|| eyre!("failed to create directory {data_dir:?}"))?;
        info!("Benchmarking `{name}` at {data_dir:?}");

        // import
        let start = Instant::now();
        dictionary::import::<S, C>(&cx.archive, &data_dir, wordbase_core_import::IMPORTERS).await?;
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

        let (_, lookups) = dictionary::open::<S, C>(&data_dir, C::default())
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

        let rows = lookups
            .lookup(cx.lemma)
            .wrap_err("failed to fetch records")?;
        info!(
            "Looked up {LOOKUP_ITERS} times in {lookup_time}, with {} records",
            rows.len()
        );
        for row in &rows {
            info!("- {} -> {:?}", row.term, row.record_id);
        }

        cx.results.push(vec![
            name.to_string(),
            storage_size,
            import_time,
            lookup_time,
            rows.len().to_string(),
        ]);
        eyre::Ok(())
    }
    .await
    .wrap_err_with(|| eyre!("failed to benchmark `{name}`"))
}

fn get_size(path: impl AsRef<Path>) -> io::Result<u64> {
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
