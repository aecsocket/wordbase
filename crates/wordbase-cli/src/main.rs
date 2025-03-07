#![doc = include_str!("../README.md")]

use anyhow::{Context, Result};
use futures::StreamExt;
use wordbase::{DictionaryId, hook::HookSentence};
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
    #[clap(alias = "l")]
    Lookup { text: String },
    Hook {
        #[command(subcommand)]
        command: HookCommand,
    },
}

#[derive(Debug, clap::Subcommand)]
enum DictionaryCommand {
    /// List all dictionaries
    Ls,
    /// Remove a dictionary with a specific ID
    Rm {
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

#[derive(Debug, clap::Subcommand)]
enum HookCommand {
    /// Sends a texthooker sentence message
    Send { text: String },
    /// Watches for texthooker sentence messages and outputs them
    Watch,
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
                command: DictionaryCommand::Ls,
            } => {
                list_dictionaries(&client);
                Ok(())
            }
            Command::Dictionary {
                command: DictionaryCommand::Rm { id },
            } => remove_dictionary(&mut client, id).await,
            Command::Dictionary {
                command: DictionaryCommand::Enable { id },
            } => enable_dictionary(&mut client, id).await,
            Command::Dictionary {
                command: DictionaryCommand::Disable { id },
            } => disable_dictionary(&mut client, id).await,
            Command::Lookup { text } => lookup(&mut client, text).await,
            Command::Hook {
                command: HookCommand::Send { text },
            } => send_hook_sentence(&mut client, text).await,
            Command::Hook {
                command: HookCommand::Watch,
            } => watch_hook_sentences(&mut client).await,
        }
    })
    .await;

    _ = client.close().await;
    result
}

fn list_dictionaries(client: &SocketClient) {
    let dictionaries = client.dictionaries();
    println!("Dictionaries ({}):", dictionaries.len());
    for (index, dictionary) in dictionaries.values().enumerate() {
        let index = index + 1;
        let enabled = if dictionary.enabled { "[on]" } else { "[  ]" };
        let id = dictionary.id.0;
        let name = &dictionary.name;
        let version = &dictionary.version;
        println!("  {index}. {enabled} [ID {id}] {name} ver {version}");
    }
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
    let mut entries = client
        .lookup(text)
        .await
        .context("failed to start lookup")?;
    while let Some(entry) = entries.next().await {
        let entry = entry.context("failed to receive lookup entry")?;
        println!("{entry:?}");
    }
    Ok(())
}

async fn send_hook_sentence(client: &mut SocketClient, sentence: String) -> Result<()> {
    client
        .hook_sentence(HookSentence {
            process_path: "wordbase-cli".into(),
            sentence,
        })
        .await
        .context("failed to send hook sentence")?;
    Ok(())
}

async fn watch_hook_sentences(client: &mut SocketClient) -> Result<()> {
    loop {
        if let wordbase_client_tokio::Event::HookSentence(sentence) =
            client.poll().await.context("failed to poll client")?
        {
            println!("{sentence:?}");
        }
    }
}
