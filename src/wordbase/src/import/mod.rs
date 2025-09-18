use {
    crate::storage::ImportStorage,
    bytes::Bytes,
    eyre::Result,
    std::{
        fs::File,
        io::Cursor,
        path::{Path, PathBuf},
    },
};

pub mod yomitan;

pub trait OpenArchive: Send + Sync {
    fn open_archive(&self) -> Result<impl Archive + 'static>;
}

impl<A, F> OpenArchive for F
where
    A: Archive + 'static,
    F: Fn() -> Result<A> + Send + Sync,
{
    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn open_archive(&self) -> Result<A> {
        (self)()
    }
}

impl OpenArchive for Bytes {
    fn open_archive(&self) -> Result<impl Archive + 'static> {
        Ok(Cursor::new(self.clone()))
    }
}

impl OpenArchive for &'static [u8] {
    fn open_archive(&self) -> Result<impl Archive + 'static> {
        Ok(Cursor::new(*self))
    }
}

impl OpenArchive for &Path {
    fn open_archive(&self) -> Result<impl Archive + 'static> {
        Ok(File::open(self)?)
    }
}

impl OpenArchive for PathBuf {
    fn open_archive(&self) -> Result<impl Archive + 'static> {
        Ok(File::open(self)?)
    }
}

pub trait Archive: Send + Sync + Unpin + std::io::Read + std::io::Seek {}

impl<T: Send + Sync + Unpin + std::io::Read + std::io::Seek> Archive for T {}

pub trait FinishImport: Send {
    fn finish(
        self,
        storage: impl ImportStorage,
        tx_progress: async_channel::Sender<ImportProgress>,
    ) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct ImportProgress {
    pub progress: f64,
}
