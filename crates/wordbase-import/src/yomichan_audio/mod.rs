use {
    eyre::Result,
    wordbase_api::DictionaryMeta,
    wordbase_storage::{
        archive::OpenArchive,
        import::{FinishImport, StartImport},
    },
};

mod schema;

/// Importer for [`wordbase_api::v1::yomichan_audio`].
pub struct YomichanAudio;

impl StartImport for YomichanAudio {
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
    todo!();
}

fn archive_reader(open_archive: &dyn OpenArchive) {}
