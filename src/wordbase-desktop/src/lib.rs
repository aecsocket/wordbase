#![doc = include_str!("../README.md")]

#[cfg(feature = "server")]
mod server;
mod texthooker;

#[cfg(feature = "server")]
pub use server::*;
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
    let dirs = directories::ProjectDirs::from("dance.aruarian", "aecsocket", "Wordbase")
        .context("failed to get default app directories")?;
    Ok(dirs.data_dir().to_path_buf())
}
