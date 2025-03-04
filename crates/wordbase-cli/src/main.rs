#![doc = include_str!("../README.md")]

use std::{
    fs::{self, File},
    io::Cursor,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use anyhow::{Context, Result};
use wordbase::{DEFAULT_PORT, protocol::Lookup, yomitan};

#[derive(Debug, clap::Parser)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, clap::Parser)]
enum Command {
    Lookup {
        url: String,
        text: String,
    },
    Dictionary {
        #[command(subcommand)]
        command: DictionaryCommand,
    },
}

#[derive(Debug, clap::Parser)]
enum DictionaryCommand {
    Parse { input: PathBuf },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = <Args as clap::Parser>::parse();

    match args.command {
        Command::Lookup { url, text } => {
            let mut client = wordbase_client_tokio::connect(url)
                .await
                .context("failed to connect to server")?;
            let response = client
                .lookup(Lookup {
                    text: text.clone(),
                    wants_html: false,
                })
                .await
                .context("failed to perform request")?;
            if let Some(response) = response {
                println!("{response:#?}");
                let conjugated = &text[..usize::from(response.conjugated_len)];
                println!("Conjugated: {conjugated}");
            } else {
                println!("(nothing)");
            }
            _ = client.stream.close(None).await;
        }
        Command::Dictionary {
            command: DictionaryCommand::Parse { input },
        } => {
            struct Stats {
                total: usize,
                done: AtomicUsize,
            }

            impl Stats {
                const fn new(total: usize) -> Self {
                    Self {
                        total,
                        done: AtomicUsize::new(0),
                    }
                }
            }

            let file_contents = fs::read(&input)?;
            let new_reader = || Ok(Cursor::new(file_contents.as_slice()));

            // let new_reader = || Ok(fs::File::open(&input)?);

            let (import, index) = yomitan::Parse::new(new_reader)?;

            let tags = Stats::new(import.tag_banks().len());
            let terms = Stats::new(import.term_banks().len());
            let term_metas = Stats::new(import.term_meta_banks().len());
            let kanjis = Stats::new(import.kanji_banks().len());
            let kanji_metas = Stats::new(import.kanji_meta_banks().len());
            import.run(
                |name, bank| {
                    let done = tags.done.fetch_add(1, Ordering::SeqCst) + 1;
                    let total = tags.total;
                    eprintln!("TAG [{done} / {total}] {name} - tags: {}", bank.len());
                },
                |name, bank| {
                    let done = terms.done.fetch_add(1, Ordering::SeqCst) + 1;
                    let total = terms.total;
                    eprintln!("TERM [{done} / {total}] {name} - terms: {}", bank.len());
                },
                |name, bank| {
                    let done = term_metas.done.fetch_add(1, Ordering::SeqCst) + 1;
                    let total = term_metas.total;
                    eprintln!(
                        "META [{done} / {total}] {name} - term metas: {}",
                        bank.len()
                    );
                },
                |name, bank| {
                    let done = kanjis.done.fetch_add(1, Ordering::SeqCst) + 1;
                    let total = kanjis.total;
                    eprintln!("KANJI [{done} / {total}] {name} - kanji: {}", bank.len());
                },
                |name, bank| {
                    let done = kanji_metas.done.fetch_add(1, Ordering::SeqCst) + 1;
                    let total = kanji_metas.total;
                    eprintln!(
                        "KANJI META [{done} / {total}] {name} - kanji metas: {}",
                        bank.len()
                    );
                },
            )?;
        }
    }

    Ok(())
}
