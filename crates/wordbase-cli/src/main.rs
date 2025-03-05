#![doc = include_str!("../README.md")]

use anyhow::{Context, Result};
use wordbase::schema::DictionaryId;
use wordbase_client_tokio::SocketClient;

/// Wordbase command line client.
#[derive(Debug, clap::Parser)]
#[command(version)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, clap::Parser)]
enum Command {
    #[clap(alias = "dic")]
    Dictionary {
        #[command(subcommand)]
        command: DictionaryCommand,
    },
    #[clap(alias = "lk")]
    Lookup { text: String },
}

#[derive(Debug, clap::Subcommand)]
enum DictionaryCommand {
    /// List all dictionaries
    #[clap(alias = "ls")]
    List,
    /// Remove a dictionary with a specific ID
    #[clap(alias = "rm")]
    Remove {
        /// ID of the dictionary, as seen in `dictionary list`
        id: i64,
    },
    /// Enable a dictionary for lookups
    Enable {
        /// ID of the dictionary, as seen in `dictionary list`
        id: i64,
    },
    /// Disable a dictionary for lookups
    Disable {
        /// ID of the dictionary, as seen in `dictionary list`
        id: i64,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = <Args as clap::Parser>::parse();

    let server_url = "ws://127.0.0.1:9518";
    let mut client = wordbase_client_tokio::connect(server_url)
        .await
        .with_context(|| format!("failed to connect to server at {server_url:?}"))?;

    let result = (async {
        match args.command {
            Command::Dictionary {
                command: DictionaryCommand::List,
            } => list_dictionaries(&mut client).await,
            Command::Dictionary {
                command: DictionaryCommand::Remove { id },
            } => remove_dictionary(&mut client, id).await,
            Command::Dictionary {
                command: DictionaryCommand::Enable { id },
            } => enable_dictionary(&mut client, id).await,
            Command::Dictionary {
                command: DictionaryCommand::Disable { id },
            } => disable_dictionary(&mut client, id).await,
            Command::Lookup { text } => lookup(&mut client, text).await,
        }
    })
    .await;

    _ = client.close().await;
    result
}

async fn list_dictionaries(client: &mut SocketClient) -> Result<()> {
    let dictionaries = client.list_dictionaries().await?;
    println!("Dictionaries ({}):", dictionaries.len());
    for dictionary in dictionaries {
        let enabled = if dictionary.enabled { "[on]" } else { "[  ]" };
        println!(
            "  {}. {enabled} {} rev {}",
            dictionary.id.0, dictionary.title, dictionary.revision
        );
    }
    Ok(())
}

async fn remove_dictionary(client: &mut SocketClient, id: i64) -> Result<()> {
    client.remove_dictionary(DictionaryId(id)).await??;
    Ok(())
}

async fn enable_dictionary(client: &mut SocketClient, id: i64) -> Result<()> {
    client.enable_dictionary(DictionaryId(id)).await??;
    Ok(())
}

async fn disable_dictionary(client: &mut SocketClient, id: i64) -> Result<()> {
    client.disable_dictionary(DictionaryId(id)).await??;
    Ok(())
}

async fn lookup(client: &mut SocketClient, text: String) -> Result<()> {
    let info = client.lookup(text).await?.context("no lookup info")?;
    println!("{info:#?}");
    Ok(())
}
