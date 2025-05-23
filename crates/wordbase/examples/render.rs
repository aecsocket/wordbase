#![expect(missing_docs, reason = "util crate")]

use std::{fmt::Write as _, time::Duration};

use anyhow::{Context, Result};
use tokio::fs;
use wordbase::{Engine, ProfileId, RecordKind};

#[derive(clap::Parser)]
struct Args {
    query: String,
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

.content {
    margin: 0 0 0 48px;
}
";

#[tokio::main]
async fn main() -> Result<()> {
    let args = <Args as clap::Parser>::parse();
    let engine = Engine::new(wordbase::data_dir().context("failed to get data dir")?)
        .await
        .context("failed to create engine")?;

    let records = engine
        .lookup(ProfileId(1), &args.query, 0, RecordKind::ALL)
        .await
        .context("failed to perform lookup")?;

    loop {
        match engine.render_to_html(&records) {
            Ok(mut html) => {
                _ = write!(&mut html, "<style>{EXTRA_CSS}</style>");
                fs::write("target/records.html", &html)
                    .await
                    .context("failed to write HTML")?;
            }
            Err(err) => {
                eprintln!("render error: {err:?}");
            }
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
        // if let Err(err) = tera.full_reload() {
        //     eprintln!("failed to reload: {err:?}");
        // }
    }
}
