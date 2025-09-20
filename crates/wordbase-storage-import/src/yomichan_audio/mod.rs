use {
    eyre::{Result, bail},
    wordbase_api::DictionaryMeta,
    wordbase_storage_api::{
        archive::OpenArchive,
        import::{FinishImport, Importer},
    },
};

mod schema;

/// Importer for [`wordbase_api::v1::yomichan_audio`].
#[derive(Debug)]
pub struct YomichanAudio;

impl Importer for YomichanAudio {
    fn start<'a>(
        &self,
        open_archive: &'a dyn OpenArchive,
    ) -> Result<(DictionaryMeta, Box<dyn FinishImport + 'a>)> {
        start(open_archive)
    }
}

pub fn start(
    open_archive: &dyn OpenArchive,
) -> Result<(DictionaryMeta, Box<dyn FinishImport + '_>)> {
    _ = open_archive;
    bail!("unimplemented");
}

// fn archive_reader(open_archive: &dyn OpenArchive) {}
