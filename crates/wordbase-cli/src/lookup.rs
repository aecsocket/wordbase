use {
    anyhow::{Context, Result},
    std::{fmt::Write as _, path::Path, time::Instant},
    tokio::fs,
    wordbase::{Engine, Profile, RecordKind, render::RenderConfig},
};

pub fn deinflect(engine: &Engine, text: &str) {
    for lemma in engine.deinflect(text) {
        let scan_len = lemma.scan_len;
        let text_part = text.get(..scan_len).map_or_else(
            || format!("(invalid scan len {scan_len})"),
            ToOwned::to_owned,
        );
        let lemma = lemma.lemma;
        println!("{text_part:?} -> {:?}", &*lemma);
    }
}

pub async fn lookup(engine: &Engine, profile: &Profile, text: &str) -> Result<()> {
    let start = Instant::now();
    let records = engine.lookup(profile.id, text, 0, RecordKind::ALL).await?;
    let end = Instant::now();
    for result in records {
        println!("{result:#?}");
    }
    println!("Fetched records in {:?}", end.duration_since(start));
    Ok(())
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

pub async fn render(engine: &Engine, profile: &Profile, text: &str, output: &Path) -> Result<()> {
    let start = Instant::now();
    let records = engine.lookup(profile.id, text, 0, RecordKind::ALL).await?;
    let end = Instant::now();
    println!("Fetched records in {:?}", end.duration_since(start));

    let start = Instant::now();
    let mut html = engine
        .render_to_html(
            &records,
            &RenderConfig {
                add_card_text: "Add Card".into(),
                add_card_js_fn: "unimplemented".into(),
            },
        )
        .context("failed to render HTML")?;
    let end = Instant::now();
    println!("Rendered HTML in {:?}", end.duration_since(start));

    _ = write!(&mut html, "<style>{EXTRA_CSS}</style>");
    fs::write(output, &html)
        .await
        .context("failed to write to file")?;

    Ok(())
}

const EXTRA_CSS: &str = "
:root {
    --bg-color: #fafafb;
    --fg-color: rgb(0 0 6 / 80%);
    --accent-color: #3584e4;
}

@media (prefers-color-scheme: dark) {
    :root {
        --bg-color: #222226;
        --fg-color: #ffffff;
    }
}
";
