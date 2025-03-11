#![doc = include_str!("../README.md")]

use {
    anyhow::{Context, Result},
    ascii_table::{Align, AsciiTable},
    futures::StreamExt,
    wordbase::{
        DictionaryId, RecordKind,
        hook::HookSentence,
        protocol::{LookupRequest, PopupAnchor, ShowPopupRequest},
    },
    wordbase_client_tokio::SocketClient,
};

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
    Lookup {
        text: String,
    },
    Popup {
        #[arg(long)]
        target_pid: Option<u32>,
        #[arg(long)]
        target_title: Option<String>,
        #[arg(long)]
        target_wm_class: Option<String>,
        #[arg(long, default_value_t = 0)]
        origin_x: i32,
        #[arg(long, default_value_t = 0)]
        origin_y: i32,
        text: String,
    },
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
        /// ID of the dictionary, as seen in `dictionary ls`
        id: i64,
    },
    /// Enable a dictionary for lookups
    Enable {
        /// ID of the dictionary, as seen in `dictionary ls`
        id: i64,
    },
    /// Disable a dictionary for lookups
    Disable {
        /// ID of the dictionary, as seen in `dictionary ls`
        id: i64,
    },
    /// Move a dictionary to a new sorting position
    #[clap(alias = "pos")]
    Position {
        /// ID of the dictionary, as seen in `dictionary ls`
        id: i64,
        /// New position of the dictionary
        position: i64,
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
            Command::Dictionary {
                command: DictionaryCommand::Position { id, position },
            } => position_dictionary(&mut client, id, position).await,
            Command::Lookup { text } => lookup(&mut client, text).await,
            Command::Popup {
                target_pid,
                target_title,
                target_wm_class,
                origin_x,
                origin_y,
                text,
            } => {
                show_popup(
                    &mut client,
                    target_pid,
                    target_title,
                    target_wm_class,
                    origin_x,
                    origin_y,
                    text,
                )
                .await
            }
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

    let mut table = AsciiTable::default();
    table.column(0).set_header("#").set_align(Align::Right);
    table.column(1).set_header("ID").set_align(Align::Right);
    table.column(2).set_header("On").set_align(Align::Left);
    table.column(3).set_header("Name").set_align(Align::Left);
    table.column(4).set_header("Version").set_align(Align::Left);

    table.print(dictionaries.values().map(|dictionary| {
        [
            format!("{}", dictionary.position),
            format!("{}", dictionary.id.0),
            format!("{}", if dictionary.enabled { "✔" } else { "" }),
            format!("{}", dictionary.meta.name),
            format!("{}", dictionary.meta.version),
        ]
    }));
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

async fn position_dictionary(client: &mut SocketClient, id: i64, position: i64) -> Result<()> {
    client
        .set_dictionary_position(DictionaryId(id), position)
        .await??;
    Ok(())
}

async fn lookup(client: &mut SocketClient, text: String) -> Result<()> {
    let mut num_records = 0usize;
    let mut records = client
        .lookup(LookupRequest {
            text,
            record_kinds: vec![
                RecordKind::GlossaryPlainText,
                RecordKind::GlossaryHtml,
                RecordKind::GlossaryPlainText,
                RecordKind::YomitanGlossary,
            ],
        })
        .await
        .context("failed to start lookup")?;
    while let Some(record) = records.next().await {
        let record = record.context("failed to receive record")?;
        num_records += 1;
        println!("{record:#?}");
    }
    println!("Total records: {num_records}");
    Ok(())
}

async fn show_popup(
    client: &mut SocketClient,
    target_pid: Option<u32>,
    target_title: Option<String>,
    target_wm_class: Option<String>,
    origin_x: i32,
    origin_y: i32,
    text: String,
) -> Result<()> {
    let response = client
        .show_popup(ShowPopupRequest {
            target_id: None,
            target_pid,
            target_title,
            target_wm_class,
            origin: (origin_x, origin_y),
            anchor: PopupAnchor::TopLeft,
            text,
        })
        .await?;
    println!("{response:?}");
    Ok(())
}

async fn send_hook_sentence(client: &mut SocketClient, sentence: String) -> Result<()> {
    client
        .hook_sentence(HookSentence {
            process_path: "wordbase-cli".into(),
            sentence,
        })
        .await?;
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
