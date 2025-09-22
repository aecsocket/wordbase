#![doc = include_str!("../README.md")]

mod texthooker;

use std::path::PathBuf;
pub use texthooker::*;

/// Gets the directory where the desktop app's user data is stored.
#[must_use]
pub fn data_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("app.wordbase", "aecsocket", "Wordbase")
        .map(|dirs| dirs.data_dir().to_path_buf())
}
