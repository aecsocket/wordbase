use {
    anyhow::{Context, Result},
    std::{fmt::Write as _, time::Instant},
    tracing::info,
    wordbase::{Engine, Profile, RecordEntry, RecordKind, render::RenderConfig},
};

pub fn deinflect(engine: &Engine, text: &str) {
    for deinflect in engine.deinflect(text, 0) {
        let text_part = text.get(deinflect.span).unwrap_or("(?)");
        info!("{text_part} -> {:?}", deinflect.lemma);
    }
}

pub async fn lookup(engine: &Engine, profile: &Profile, text: &str) -> Result<Vec<RecordEntry>> {
    let start = Instant::now();
    let records = engine.lookup(profile.id, text, 0, RecordKind::ALL).await?;
    let end = Instant::now();
    // TODO: a nice, sort-of-human-readable output
    info!("Fetched records in {:?}", end.duration_since(start));
    Ok(records)
}

pub async fn lookup_lemma(engine: &Engine, profile: &Profile, lemma: &str) -> Result<()> {
    for result in engine
        .lookup_lemma(profile.id, &lemma, RecordKind::ALL)
        .await?
    {
        println!("{result:#?}");
    }
    Ok(())
}

pub async fn render(engine: &Engine, profile: &Profile, text: &str) -> Result<()> {
    let start = Instant::now();
    let records = engine.lookup(profile.id, text, 0, RecordKind::ALL).await?;
    let end = Instant::now();
    info!("Fetched records in {:?}", end.duration_since(start));

    let start = Instant::now();
    let mut html = engine
        .render_to_html(
            &records,
            &RenderConfig {
                add_note_text: Some("Add Card".into()),
                add_note_js_fn: Some("unimplemented".into()),
            },
        )
        .context("failed to render HTML")?;
    _ = write!(&mut html, "<style>{EXTRA_CSS}</style>");
    let end = Instant::now();
    info!("Rendered HTML in {:?}", end.duration_since(start));

    println!("{html}");
    Ok(())
}

// TODO: this should probably be put into the renderer somehow
const EXTRA_CSS: &str = "
:root {
    --accent-color: #3584e4;
    --on-accent-color: #ffffff;
}

:root {
    --bg-color: #fafafb;
    --fg-color: rgb(0 0 6 / 80%);
}

@media (prefers-color-scheme: dark) {
    :root {
        --bg-color: #222226;
        --fg-color: #ffffff;
    }
}
";
