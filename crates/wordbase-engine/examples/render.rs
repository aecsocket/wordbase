#![expect(missing_docs, reason = "util crate")]

use std::{path::PathBuf, time::Duration};

use anyhow::{Context, Result};
use tera::Tera;
use tokio::fs;
use wordbase::{ProfileId, RecordKind};
use wordbase_engine::Engine;

#[derive(clap::Parser)]
struct Args {
    query: String,
    #[arg(short, long)]
    profile_id: i64,
    #[arg(short, long)]
    out: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = <Args as clap::Parser>::parse();
    let engine = Engine::new(wordbase_engine::data_dir().context("failed to get data dir")?)
        .await
        .context("failed to create engine")?;

    let query = "見る";
    let records = engine
        .lookup(ProfileId(args.profile_id), query, 0, RecordKind::ALL)
        .await
        .context("failed to perform lookup")?;

    let mut tera = Tera::new("record-templates/**/*").unwrap();

    loop {
        let mut context = tera::Context::new();
        context.insert("records", &records);

        match tera.render("records.html", &context) {
            Ok(html) => {
                fs::write(&args.out, &html)
                    .await
                    .context("failed to write HTML")?;
            }
            Err(err) => {
                eprintln!("render error: {err:?}");
            }
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
        if let Err(err) = tera.full_reload() {
            eprintln!("failed to reload: {err:?}");
        }
    }
}
