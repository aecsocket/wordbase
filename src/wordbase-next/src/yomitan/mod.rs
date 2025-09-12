use {
    crate::storage::{DictionaryWrite, DictionaryWriteStorage, DictionaryWriteTx},
    eyre::{Context as _, ContextCompat, Result, eyre},
    rayon::prelude::*,
    serde::de::DeserializeOwned,
    std::path::PathBuf,
    tokio::sync::Mutex,
    wordbase_api::{DictionaryKind, DictionaryMeta, Record, Term},
    zip::ZipArchive,
};

mod schema;

pub trait OpenArchive: Send + Sync + Clone {
    fn open_archive(&self) -> Result<Box<dyn Archive>>;
}

impl<F: Fn() -> Result<Box<dyn Archive>> + Send + Sync + Clone> OpenArchive for F {
    fn open_archive(&self) -> Result<Box<dyn Archive>> {
        (self)()
    }
}

pub trait Archive: Send + Sync + std::io::Read + std::io::Seek + Unpin {}

impl<T: Send + Sync + Unpin + std::io::Read + std::io::Seek> Archive for T {}

fn archive_reader(open_archive: &impl OpenArchive) -> Result<ZipArchive<Box<dyn Archive>>> {
    let archive = open_archive
        .open_archive()
        .wrap_err("failed to open archive")?;
    ZipArchive::new(archive).wrap_err("failed to read zip archive")
}

pub fn start_import(open_archive: impl OpenArchive) -> Result<()> {
    let mut archive = archive_reader(&open_archive)?;

    let index = {
        let file = archive
            .by_name(schema::INDEX_PATH)
            .wrap_err_with(|| eyre!("missing `{}`", schema::INDEX_PATH))?;
        serde_json::from_reader::<_, schema::Index>(file).wrap_err("failed to parse index")?
    };

    let mut meta = DictionaryMeta::new(DictionaryKind::Yomitan, &index.title);
    meta.version = Some(index.revision.clone());
    meta.description = index.description.clone();
    meta.url = index.url.clone();
    meta.attribution = index.attribution.clone();

    Ok(())
}

pub struct Storage {
    data_dir: PathBuf,
}

pub fn finish_import(open_archive: impl OpenArchive, storage: &Storage) -> Result<()> {
    let archive = archive_reader(&open_archive)?;

    let mut tag_bank_paths = Vec::new();
    let mut term_bank_paths = Vec::new();
    for file_path in archive.file_names() {
        println!("Processing {file_path:?}");

        if schema::TAG_BANK_PATTERN.is_match(file_path) {
            tag_bank_paths.push(file_path.to_owned());
        } else if schema::TERM_BANK_PATTERN.is_match(file_path) {
            term_bank_paths.push(file_path.to_owned());
        }
    }

    println!("Tags: {tag_bank_paths:?}");
    println!("Terms: {term_bank_paths:?}");

    println!("Beginning write");
    let txn = storage.begin().wrap_err("failed to start transaction")?;
    println!("Make writer");
    let writer = Mutex::new(txn.writer().wrap_err("failed to create writer")?);

    println!("Starting write");

    let (r1, r2) = rayon::join(
        || {
            tag_bank_paths.into_par_iter().try_for_each(|path| {
                import_bank(&open_archive, &writer, &path, import_tag)
                    .wrap_err_with(|| eyre!("failed to import tag bank `{path}`"))
            })
        },
        || {
            term_bank_paths.into_par_iter().try_for_each(|path| {
                import_bank(&open_archive, &writer, &path, import_term)
                    .wrap_err_with(|| eyre!("failed to import term bank `{path}`"))
            })
        },
    );
    r1?;
    r2?;

    drop(writer);
    txn.commit().wrap_err("failed to commit transaction")?;
    Ok(())
}

fn import_bank<D, W: DictionaryWrite>(
    open_archive: &impl OpenArchive,
    writer: &Mutex<W>,
    path: &str,
    mut import_item: impl FnMut(&mut W, D) -> Result<()>,
) -> Result<()>
where
    Vec<D>: DeserializeOwned,
{
    let mut archive = archive_reader(open_archive)?;
    let file = archive.by_name(path).wrap_err("file does not exist")?;
    let bank = serde_json::from_reader::<_, Vec<D>>(file).wrap_err("failed to parse file")?;

    println!("Parsed bank: {path:?}");

    let mut writer = writer.blocking_lock();
    for data in bank {
        import_item(&mut *writer, data)?;
    }
    Ok(())
}

fn import_tag(writer: &mut impl DictionaryWrite, data: schema::Tag) -> Result<()> {
    Ok(())
}

fn import_term(writer: &mut impl DictionaryWrite, data: schema::Term) -> Result<()> {
    let term =
        Term::from_full(data.expression.as_str(), data.reading.as_str()).wrap_err_with(|| {
            eyre!(
                "term ({:?}, {:?}) is not a valid term",
                data.expression,
                data.reading
            )
        })?;

    (|| {
        let record_id = writer
            .insert_record(&Record::YomitanFrequency(Default::default()))
            .wrap_err("failed to insert record")?;
        writer
            .insert_term(&term, record_id)
            .wrap_err("failed to insert term")?;

        println!("Imported term {term:?}");
        eyre::Ok(())
    })()
    .wrap_err_with(|| eyre!("failed to insert {term}"))
}
