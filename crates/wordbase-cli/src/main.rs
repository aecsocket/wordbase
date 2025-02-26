#![doc = include_str!("../README.md")]

use std::{
    fs::File,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

use anyhow::{Context, Result};
use wordbase::{DEFAULT_PORT, dictionary::yomitan};

#[derive(Debug, clap::Parser)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, clap::Parser)]
enum Command {
    Client {
        #[arg(long, default_value_t = format!("ws://127.0.0.1:{DEFAULT_PORT}"))]
        server_addr: String,
        #[command(subcommand)]
        command: ClientCommand,
    },
    Dictionary {
        #[command(subcommand)]
        command: DictionaryCommand,
    },
}

#[derive(Debug, clap::Parser)]
enum ClientCommand {
    Ping,
    Lookup { text: String },
}

#[derive(Debug, clap::Parser)]
enum DictionaryCommand {
    Parse { input: PathBuf },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = <Args as clap::Parser>::parse();

    match args.command {
        Command::Client {
            server_addr,
            command,
        } => {
            let mut connection = wordbase_tokio_tungstenite::connect(server_addr)
                .await
                .context("failed to connect to server")?;

            match command {
                ClientCommand::Ping => {
                    let x = connection.ping().await?;
                    println!("{x:?}");
                }
                ClientCommand::Lookup { text } => {
                    let x = connection.lookup(text).await?;
                    println!("{x:?}");
                }
            }
            _ = connection.into_inner().close(None).await;
        }
        Command::Dictionary {
            command: DictionaryCommand::Parse { input },
        } => {
            let (import, index) = yomitan::Parse::new(|| {
                let file = File::open(&input)?;
                Ok(file)
            })?;

            let total_banks = import.term_bank_names().len();
            let banks_done = AtomicUsize::new(0);
            import.run(
                |_, _| {},
                |name, bank| {
                    let banks_done = banks_done.fetch_add(1, Ordering::SeqCst) + 1;
                    eprintln!(
                        "[{banks_done} / {total_banks}] {name} - terms: {}",
                        bank.len()
                    );
                },
                |_, _| {},
            )?;
        }
    }

    Ok(())
}
