use {
    anyhow::{Context, Result},
    itertools::Itertools,
    std::time::Instant,
    tracing::info,
    wordbase::{
        Engine, Profile, RecordId, RecordKind,
        render::{HtmlRender, RenderConfig},
    },
};

pub fn deinflect(engine: &Engine, text: &str) {
    for deinflect in engine.deinflect(text, 0) {
        let text_part = text.get(deinflect.span).unwrap_or("(?)");
        info!("{text_part} -> {:?}", deinflect.lemma);
    }
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
    let HtmlRender { body, audio_blobs } = engine
        .render_html(
            &records,
            &RenderConfig {
                s_add_note: "Add Card".into(),
                fn_add_note: Some("unimplemented".into()),
                fn_audio_blob: "Wordbase.audio_blob".into(),
            },
        )
        .context("failed to render HTML")?;

    let audio_blobs = audio_blobs
        .into_iter()
        .map(|(RecordId(record_id), blob)| format!("{record_id}: '{blob}'"))
        .join(",");

    let js = format!(
        "Wordbase.audio_blob = function(record_id) {{
            const audio_blobs = {{ {audio_blobs} }};
            return audio_blobs[record_id];
        }}"
    );

    let document = format!(
        "
<!doctype html>
<html>
    <body>
        {body}
        <script>{js}</script>
        <style>{EXTRA_CSS}</style>
    </body>
</html>
"
    );
    let end = Instant::now();
    info!("Rendered HTML in {:?}", end.duration_since(start));

    println!("{document}");
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
