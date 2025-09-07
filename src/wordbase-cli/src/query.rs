use {
    anyhow::{Context, Result},
    std::time::Instant,
    tracing::info,
    wordbase::{Engine, Profile, render::RenderConfig},
};

pub fn make_query(pre_cursor: &str, post_cursor: Option<&str>) -> (String, usize) {
    post_cursor.map_or_else(
        || (pre_cursor.to_string(), 0),
        |post_cursor| (format!("{pre_cursor}{post_cursor}"), pre_cursor.len()),
    )
}

pub fn deinflect(engine: &Engine, pre_cursor: &str, post_cursor: Option<&str>) {
    let (text, cursor) = make_query(pre_cursor, post_cursor);
    for deinflect in engine.deinflect(&text, cursor) {
        let text_part = text.get(deinflect.span).unwrap_or("(?)");
        info!("{text_part} -> {:?}", deinflect.lemma);
    }
}

pub async fn lookup_lemma(engine: &Engine, profile: &Profile, lemma: &str) -> Result<()> {
    for result in engine.lookup_lemma(profile.id, &lemma).await? {
        println!("{result:#?}");
    }
    Ok(())
}

pub async fn render(
    engine: &Engine,
    profile: &Profile,
    pre_cursor: &str,
    post_cursor: Option<&str>,
) -> Result<()> {
    let (text, cursor) = make_query(pre_cursor, post_cursor);
    let start = Instant::now();
    let records = engine.lookup(profile.id, &text, cursor).await?;
    let end = Instant::now();
    info!("Fetched records in {:?}", end.duration_since(start));

    let start = Instant::now();
    let body = engine
        .render_html_body(
            &records,
            &RenderConfig {
                s_add_note: "Add Note".into(),
                s_view_note: "View note in Anki".into(),
                s_add_duplicate_note: "Add duplicate note".into(),
                fn_num_existing_notes: "
                /* <js_callback>(window.wordbase.note_exists({
                    headword: <js_headword>,
                    reading: <js_reading>,
                })) */ <js_callback>(4)
                "
                .into(),
                fn_add_new_note: "<js_callback>()".into(),
                fn_add_duplicate_note: "<js_callback>()".into(),
                fn_view_note: "
                window.wordbase.view_note({
                    headword: <js_headword>,
                    reading: <js_reading>,
                })"
                .into(),
            },
        )
        .context("failed to render HTML")?;

    let document = format!(
        "
<!doctype html>
<html>
    <body>
        {body}
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
