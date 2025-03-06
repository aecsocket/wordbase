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

mod format;
mod structured;
mod ui;

use std::{convert::Infallible, time::Duration};

use adw::prelude::*;
use anyhow::{Context, Result};
use foldhash::HashMap;
use format::Terms;
use gtk::{gdk, glib};
use tokio::{
    sync::{mpsc, oneshot},
    time,
};
use tracing::warn;
use wordbase::schema::{Dictionary, DictionaryId, LookupInfo};
use wordbase_client_tokio::SocketClient;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let (send_lookup_request, recv_lookup_request) = mpsc::channel::<LookupRequest>(4);
    tokio::spawn(backend(recv_lookup_request));

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

    app.connect_activate(move |app| {
        let content = ui::Lookup::new();

        let send_lookup_request = send_lookup_request.clone();

        let window_content = content.clone();
        content.entry().connect_changed(move |entry| {
            let query = entry.text().to_string();
            let send_lookup_request = send_lookup_request.clone();
            let (send_response, recv_response) = oneshot::channel();

            let content = content.clone();
            glib::spawn_future_local(async move {
                send_lookup_request
                    .send(LookupRequest {
                        query,
                        send_response,
                    })
                    .await?;
                let response = recv_response.await?;

                if let Some(response) = response {
                    content.lemma().set_text(&response.info.lemma);

                    let terms = Terms::new(&response.dictionaries, response.info);
                    content
                        .dictionary_container()
                        .set_child(Some(&terms.to_ui()));
                } else {
                    content.lemma().set_text("");
                    content
                        .dictionary_container()
                        .set_child(None::<&ui::Dictionary>);
                }

                anyhow::Ok(())
            });
        });

        adw::ApplicationWindow::builder()
            .application(app)
            .title("Dictionary")
            .content(&window_content)
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
    send_response: oneshot::Sender<Option<LookupResponse>>,
}

#[derive(Debug)]
struct LookupResponse {
    dictionaries: HashMap<DictionaryId, Dictionary>,
    info: LookupInfo,
}

async fn backend(mut recv_lookup_request: mpsc::Receiver<LookupRequest>) -> Result<Infallible> {
    loop {
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

        let Err(err) = handle_client(&mut recv_lookup_request, &mut client).await;
        warn!("Lost connection from server: {err:?}");
        _ = client.close().await;
    }
}

async fn handle_client(
    recv_lookup_request: &mut mpsc::Receiver<LookupRequest>,
    client: &mut SocketClient,
) -> Result<Infallible> {
    loop {
        let request = recv_lookup_request
            .recv()
            .await
            .context("lookup request channel closed")?;

        let dictionaries = client
            .list_dictionaries()
            .await
            .context("failed to list dictionaries")?;
        let dictionaries = dictionaries
            .into_iter()
            .map(|dictionary| (dictionary.id, dictionary))
            .collect();

        let info = client
            .lookup(request.query)
            .await
            .context("failed to perform lookup")?;

        let response = info.map(|info| LookupResponse { dictionaries, info });
        _ = request.send_response.send(response);
    }
}
