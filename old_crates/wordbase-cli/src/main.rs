#![doc = include_str!("../README.md")]

use {
    anyhow::{Context, Result},
    ascii_table::{Align, AsciiTable},
    futures::StreamExt,
    wordbase::{
        RecordKind,
        hook::HookSentence,
        protocol::{LookupRequest, PopupAnchor, ShowPopupRequest, WindowFilter},
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
    #[clap(alias = "dics")]
    Dictionaries,
    Lookup {
        text: String,
    },
    Popup(PopupCommand),
    Hook {
        #[command(subcommand)]
        command: HookCommand,
    },
}

#[derive(Debug, clap::Subcommand)]
enum HookCommand {
    /// Sends a texthooker sentence message
    Send { text: String },
    /// Watches for texthooker sentence messages and outputs them
    Watch,
}

#[derive(Debug, clap::Parser)]
struct PopupCommand {
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
            Command::Dictionaries => {
                list_dictionaries(&client);
                Ok(())
            }
            Command::Lookup { text } => lookup(&mut client, text).await,
            Command::Popup(popup) => show_popup(&mut client, popup).await,
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
    table.column(0).set_header("ID").set_align(Align::Right);
    table.column(1).set_header("Name").set_align(Align::Left);
    table.column(2).set_header("Version").set_align(Align::Left);

    table.print(dictionaries.values().map(|dictionary| {
        [
            format!("{}", dictionary.id.0),
            format!("{}", dictionary.meta.name),
            format!("{}", dictionary.meta.version),
        ]
    }));
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
                RecordKind::YomitanRecord,
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

async fn show_popup(client: &mut SocketClient, popup: PopupCommand) -> Result<()> {
    let response = client
        .show_popup(ShowPopupRequest {
            target_window: WindowFilter {
                id: None,
                pid: popup.target_pid,
                title: popup.target_title,
                wm_class: popup.target_wm_class,
            },
            origin: (popup.origin_x, popup.origin_y),
            anchor: PopupAnchor::TopLeft,
            text: popup.text,
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
