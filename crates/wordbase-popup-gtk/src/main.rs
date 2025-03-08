#![allow(
    clippy::wildcard_imports,
    reason = "in `mod imp`s, we often use `super::*`"
)]
#![allow(
    clippy::new_without_default,
    reason = "`gtk` types do not follow this pattern, so neither do we"
)]

extern crate gtk4 as gtk;
extern crate libadwaita as adw;
extern crate webkit6 as webkit;

// mod format;
mod ui;

use {
    adw::prelude::*,
    anyhow::{Context, Result},
    futures::StreamExt,
    gtk::gdk,
    log::warn,
    std::{cell::RefCell, convert::Infallible, rc::Rc, time::Duration},
    tokio::{
        sync::{broadcast, mpsc},
        time,
    },
    wordbase::{Dictionary, DictionaryId, lookup::LookupEntry, protocol::HookSentence},
    wordbase_client_tokio::{IndexMap, SocketClient},
};

const CHANNEL_BUF_CAP: usize = 4;

type DictionaryMap = IndexMap<DictionaryId, Dictionary>;

#[tokio::main]
async fn main() {
    pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Info)
        .try_init()
        .expect("failed to initialize logger");
    glib::log_set_default_handler(glib::rust_log_handler);

    let (send_lookup_request, recv_lookup_request) =
        mpsc::channel::<LookupRequest>(CHANNEL_BUF_CAP);
    let (send_backend_event, recv_backend_event) =
        broadcast::channel::<BackendEvent>(CHANNEL_BUF_CAP);
    tokio::spawn(tokio_backend(recv_lookup_request, send_backend_event));

    let app = adw::Application::builder()
        .application_id("com.github.aecsocket.WordbasePopup")
        .build();

    app.connect_startup(|_| {
        let css_provider = gtk::CssProvider::new();
        css_provider.load_from_string(include_str!("ui/style.css"));

        gtk::style_context_add_provider_for_display(
            &gdk::Display::default().expect("failed to get display"),
            &css_provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    });

    let recv_backend_event = recv_backend_event.resubscribe();
    app.connect_activate(move |app| {
        let toast_overlay = adw::ToastOverlay::new();

        let content = ui::Lookup::new();
        toast_overlay.set_child(Some(&content));

        let dictionaries = Rc::new(RefCell::new(DictionaryMap::default()));

        {
            let send_lookup_request = send_lookup_request.clone();
            let dictionaries = dictionaries.clone();
            content.entry().connect_changed(move |entry| {
                on_search_changed(entry, send_lookup_request.clone(), dictionaries.clone());
            });
        }

        glib::spawn_future_local(local_backend(
            recv_backend_event.resubscribe(),
            toast_overlay.clone(),
            dictionaries,
        ));

        adw::ApplicationWindow::builder()
            .application(app)
            .title("Dictionary")
            .content(&toast_overlay)
            .default_width(800)
            .default_height(400)
            .build()
            .present();
    });

    app.run();
}

#[derive(Debug)]
struct LookupRequest {
    query: String,
    send_lookup: mpsc::Sender<LookupEntry>,
}

fn on_search_changed(
    entry: &gtk::Entry,
    send_lookup_request: mpsc::Sender<LookupRequest>,
    dictionaries: Rc<RefCell<DictionaryMap>>,
) {
    let query = entry.text().to_string();
    let (send_lookup, mut recv_lookup) = mpsc::channel(CHANNEL_BUF_CAP);
    glib::spawn_future_local(async move {
        send_lookup_request
            .send(LookupRequest { query, send_lookup })
            .await?;

        while let Some(lookup) = recv_lookup.recv().await {}

        // if let Some(response) = dictionaries {
        //     let terms = Terms::new(&response.dictionaries, response.info);
        //     content
        //         .dictionary_container()
        //         .set_child(Some(&terms.to_ui()));
        // } else {
        //     content
        //         .dictionary_container()
        //         .set_child(None::<&ui::Dictionary>);
        // }

        anyhow::Ok(())
    });
}

#[expect(
    clippy::future_not_send,
    reason = "this future is only ran on the main thread"
)]
async fn local_backend(
    mut recv_backend_event: broadcast::Receiver<BackendEvent>,
    toast_overlay: adw::ToastOverlay,
    current_dictionaries: Rc<RefCell<DictionaryMap>>,
) -> Result<()> {
    loop {
        let event = recv_backend_event
            .recv()
            .await
            .context("event channel dropped")?;

        match event {
            BackendEvent::Connected { dictionaries } => {
                toast_overlay.add_toast(adw::Toast::new("Connected to server"));
                current_dictionaries.replace(dictionaries);
            }
            BackendEvent::Disconnected => {
                toast_overlay.add_toast(adw::Toast::new("Disconnected from server"));
                current_dictionaries.replace(DictionaryMap::default());
            }
            BackendEvent::SyncDictionaries { dictionaries } => {
                toast_overlay.add_toast(adw::Toast::new("Updated dictionaries"));
                current_dictionaries.replace(dictionaries);
            }
            BackendEvent::HookSentence(_) => {}
        }
    }
}

#[derive(Debug, Clone)]
enum BackendEvent {
    Connected { dictionaries: DictionaryMap },
    Disconnected,
    SyncDictionaries { dictionaries: DictionaryMap },
    HookSentence(HookSentence),
}

async fn tokio_backend(
    mut recv_lookup_request: mpsc::Receiver<LookupRequest>,
    send_event: broadcast::Sender<BackendEvent>,
) -> Result<Infallible> {
    loop {
        #[expect(clippy::redundant_pub_crate, reason = "false positive")]
        let mut client = loop {
            tokio::select! {
                result = wordbase_client_tokio::connect("ws://127.0.0.1:9518") => {
                    match result {
                        Ok(client) => break client,
                        Err(err) => {
                            warn!("Failed to connect to server: {err:?}");
                            time::sleep(Duration::from_secs(1)).await;
                            continue;
                        }
                    }
                }
                _ = recv_lookup_request.recv() => continue,
            };
        };

        let Err(err) = handle_client(&mut recv_lookup_request, &send_event, &mut client).await;
        warn!("Lost connection from server: {err:?}");
        send_event.send(BackendEvent::Disconnected)?;
        _ = client.close().await;
    }
}

async fn handle_client(
    recv_lookup_request: &mut mpsc::Receiver<LookupRequest>,
    send_event: &broadcast::Sender<BackendEvent>,
    client: &mut SocketClient,
) -> Result<Infallible> {
    send_event.send(BackendEvent::Connected {
        dictionaries: client.dictionaries().clone(),
    })?;

    #[expect(clippy::redundant_pub_crate, reason = "false positive")]
    loop {
        tokio::select! {
            event = client.poll() => {
                let event = event?;
                forward_event(send_event, client, event)?;
            }
            request = recv_lookup_request.recv() => {
                let request = request.context("request channel dropped")?;
                _ = handle_request(client, request).await;
            }
        }
    }
}

fn forward_event(
    send_event: &broadcast::Sender<BackendEvent>,
    client: &SocketClient,
    event: wordbase_client_tokio::Event,
) -> Result<()> {
    match event {
        wordbase_client_tokio::Event::SyncLookupConfig => Ok(()),
        wordbase_client_tokio::Event::SyncDictionaries => {
            send_event.send(BackendEvent::SyncDictionaries {
                dictionaries: client.dictionaries().clone(),
            })?;
            Ok(())
        }
        wordbase_client_tokio::Event::HookSentence(sentence) => {
            send_event.send(BackendEvent::HookSentence(sentence))?;
            Ok(())
        }
    }
}

async fn handle_request(client: &mut SocketClient, request: LookupRequest) -> Result<()> {
    let mut lookups = client
        .lookup(request.query)
        .await
        .context("failed to start lookup")?;
    while let Some(lookup) = lookups.next().await {
        let lookup = lookup.context("failed to receive lookup")?;
        request
            .send_lookup
            .send(lookup)
            .await
            .context("lookup channel dropped")?;
    }
    Ok(())
}
