//! See [`Archive`].

use {
    bytes::Bytes,
    eyre::Result,
    std::{
        fs::File,
        io::{Cursor, Read, Seek},
        path::{Path, PathBuf},
    },
};

/// Stream of bytes which can be read from to import a dictionary.
///
/// This trait is automatically implemented for compatible types.
pub trait Archive: Send + Sync + Read + Seek {}

impl<T: ?Sized + Send + Sync + Read + Seek> Archive for T {}

/// Allows creating a readable [`Archive`].
///
/// This is implemented on:
/// - [`Bytes`]
/// - `&'static [u8]`
/// - [`&Path`][Path] - opening a [`File`]
/// - [`PathBuf`] - opening a [`File`]
pub trait OpenArchive: Send + Sync {
    /// Opens an [`Archive`] and passes ownership to the caller.
    ///
    /// # Errors
    ///
    /// Errors if the archive could not be opened.
    fn open_archive(&self) -> Result<Box<dyn Archive>>;
}

impl<F> OpenArchive for F
where
    F: Fn() -> Result<Box<dyn Archive>> + Send + Sync,
{
    fn open_archive(&self) -> Result<Box<dyn Archive>> {
        (self)()
    }
}

impl OpenArchive for Bytes {
    fn open_archive(&self) -> Result<Box<dyn Archive>> {
        Ok(Box::new(Cursor::new(self.clone())))
    }
}

impl OpenArchive for &'static [u8] {
    fn open_archive(&self) -> Result<Box<dyn Archive>> {
        Ok(Box::new(Cursor::new(*self)))
    }
}

impl OpenArchive for &Path {
    fn open_archive(&self) -> Result<Box<dyn Archive>> {
        Ok(Box::new(File::open(self)?))
    }
}

impl OpenArchive for PathBuf {
    fn open_archive(&self) -> Result<Box<dyn Archive>> {
        Ok(Box::new(File::open(self)?))
    }
}
