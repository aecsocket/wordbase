use {
    crate::db::ImportStorage,
    bytes::Bytes,
    eyre::Result,
    std::{
        fs::File,
        io::Cursor,
        path::{Path, PathBuf},
    },
};

pub mod yomitan;

pub trait OpenArchive: Sync {
    fn open_archive(&self) -> Result<impl Archive>;
}

impl<A, F> OpenArchive for F
where
    A: Archive,
    F: Fn() -> Result<A> + Sync,
{
    #[expect(refining_impl_trait, reason = "explicit refinement")]
    fn open_archive(&self) -> Result<A> {
        (self)()
    }
}

impl OpenArchive for Bytes {
    fn open_archive(&self) -> Result<impl Archive> {
        Ok(Cursor::new(self.clone()))
    }
}

impl OpenArchive for &[u8] {
    fn open_archive(&self) -> Result<impl Archive> {
        Ok(Cursor::new(self))
    }
}

impl OpenArchive for &Path {
    fn open_archive(&self) -> Result<impl Archive> {
        Ok(File::open(self)?)
    }
}

impl OpenArchive for PathBuf {
    fn open_archive(&self) -> Result<impl Archive> {
        Ok(File::open(self)?)
    }
}

pub trait Archive: Send + Sync + Unpin + std::io::Read + std::io::Seek {}

impl<T: Send + Sync + Unpin + std::io::Read + std::io::Seek> Archive for T {}

pub trait FinishImport {
    fn finish(self, storage: impl ImportStorage) -> Result<()>;
}
