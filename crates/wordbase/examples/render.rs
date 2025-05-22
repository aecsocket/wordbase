#![expect(missing_docs, reason = "util crate")]

use std::time::Duration;

use anyhow::{Context, Result};
use tokio::fs;
use wordbase::{Engine, ProfileId, RecordKind};

#[derive(clap::Parser)]
struct Args {
    query: String,
}

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
        match engine.render_to_html(&records, (0x35, 0x84, 0xe4)) {
            Ok(html) => {
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
