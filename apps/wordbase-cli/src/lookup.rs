use {crate::App, eyre::Result, tracing::info};

pub async fn sentence(app: &App, first: &str, second: Option<&str>) -> Result<()> {
    let (sentence, cursor) = second.map_or_else(
        || (first.to_string(), 0),
        |second| {
            let mut sentence = first.to_string();
            sentence.push_str(second);
            (sentence, first.len())
        },
    );

    let entries = app
        .deinflectors()
        .lookup(&app.storage, app.profile_id, &sentence, cursor)?;

    info!("{} entries", entries.len());

    Ok(())
}

pub async fn lemma(app: &App, lemma: &str) -> Result<()> {
    let entries = app.storage.lookup_lemma(app.profile_id, lemma)?;
    info!("{} entries", entries.len());

    Ok(())
}
