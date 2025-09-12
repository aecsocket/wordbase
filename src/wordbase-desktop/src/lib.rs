#![doc = include_str!("../README.md")]

#[cfg(feature = "http")]
pub mod http;
mod texthooker;

pub use texthooker::*;
use {
    anyhow::{Context, Result},
    std::path::PathBuf,
};

/// Gets the directory where the desktop app's user data is stored.
///
/// Pass this directory into [`wordbase::Engine::new`] to use the default data
/// directory.
///
/// # Errors
///
/// Errors if [`directories::ProjectDirs::from`] returns [`None`].
pub fn data_dir() -> Result<PathBuf> {
    let dirs = directories::ProjectDirs::from("app.wordbase", "aecsocket", "Wordbase")
        .context("failed to get default app directories")?;
    Ok(dirs.data_dir().to_path_buf())
}
