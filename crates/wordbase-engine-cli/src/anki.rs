use anyhow::{Context, Result};
use wordbase::Term;
use wordbase_engine::{Engine, profile::ProfileState};

pub async fn create_note(
    engine: &Engine,
    profile: &ProfileState,
    sentence: &str,
    headword: &str,
    reading: &str,
) -> Result<()> {
    let term = Term::new(headword, reading).context("invalid term")?;
    // TODO
    engine
        .connect_anki("http://host.docker.internal:8765", "")
        .await?;
    engine
        .create_anki_note(profile.id, sentence, 0, &term)
        .await?;
    Ok(())
}

pub async fn set_url(engine: &Engine, url: &str) -> Result<()> {
    engine.connect_anki(url, "").await?;
    Ok(())
}
