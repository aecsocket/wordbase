use {
    anyhow::{Context as _, Result},
    std::path::PathBuf,
};

pub fn data_dir() -> Result<PathBuf> {
    let dirs = directories::ProjectDirs::from("dance.aruarian", "aecsocket", "Wordbase")
        .context("failed to get default app directories")?;
    Ok(dirs.data_dir().to_path_buf())
}
