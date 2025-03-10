extern crate gtk4 as gtk;
extern crate libadwaita as adw;

use anyhow::{Context, Result, bail};
use foldhash::{HashMap, HashMapExt};
use futures::never::Never;
use gtk4::{gdk, gio::prelude::*, prelude::*};
use libadwaita::prelude::BinExt;
use tokio::sync::broadcast;
use tracing::{error, info, warn};
use wordbase::{
    DictionaryId, DictionaryState,
    protocol::{LookupRequest, NoRecords, ShowPopupRequest, ShowPopupResponse},
};

use crate::{ServerEvent, lookup};

use super::Request;

const APP_ID: &str = "com.github.aecsocket.Wordbase";

pub fn run(
    lookups: lookup::Client,
    recv_server_event: broadcast::Receiver<ServerEvent>,
    recv_request: broadcast::Receiver<Request>,
) -> Result<()> {
    info!("Using Wayland popup backend via GTK/Adwaita");
    glib::log_set_default_handler(glib::rust_log_handler);

    let app = adw::Application::builder().application_id(APP_ID).build();
    // make the app not close when all its windows are closed
    let _hold_guard = app.hold();

    app.connect_startup(|_| {
        info!("TODO: STARTUP");

        let provider = gtk::CssProvider::new();
        provider.load_from_string(wordbase_gtk::STYLESHEET);

        gtk::style_context_add_provider_for_display(
            &gdk::Display::default().expect("failed to get display"),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    });
    app.connect_activate(move |app| {
        info!("TODO: ACTIVATE");

        let lookups = lookups.clone();
        let recv_request = recv_request.resubscribe();
        let recv_server_event = recv_server_event.resubscribe();
        let app = app.clone();
        glib::spawn_future_local(async move {
            let Err(err) = backend(lookups, recv_request, recv_server_event, app).await;
            error!("GTK app backend closed: {err:?}");
        });
    });

    app.run();
    bail!("GTK application closed")
}

async fn backend(
    lookups: lookup::Client,
    mut recv_request: broadcast::Receiver<Request>,
    mut recv_server_event: broadcast::Receiver<ServerEvent>,
    app: adw::Application,
) -> Result<Never> {
    let popup = create_popup(&app);
    let mut dictionaries = HashMap::<DictionaryId, DictionaryState>::new();
    loop {
        let request = tokio::select! {
            request = recv_request.recv() => request,
            Ok(ServerEvent::SyncDictionaries(new_dictionaries)) = recv_server_event.recv() => {
                dictionaries = new_dictionaries
                    .into_iter()
                    .map(|state| (state.id, state))
                    .collect();
                continue;
            }
        };
        let request = match request {
            Ok(request) => request,
            Err(broadcast::error::RecvError::Closed) => bail!("popup request channel closed"),
            Err(broadcast::error::RecvError::Lagged(n)) => {
                warn!("Popup thread lagged by {n} requests");
                continue;
            }
        };

        let result = handle_request(&lookups, &popup, &dictionaries, request.request).await;
        _ = request.send_response.send(result).await;
    }
}

async fn handle_request(
    lookups: &lookup::Client,
    popup: &PopupInfo,
    dictionaries: &HashMap<DictionaryId, DictionaryState>,
    request: ShowPopupRequest,
) -> Result<Result<ShowPopupResponse, NoRecords>> {
    let records = lookups
        .lookup(LookupRequest {
            text: request.text,
            record_kinds: wordbase_gtk::SUPPORTED_RECORD_KINDS.to_vec(),
        })
        .await
        .context("failed to perform lookup")?;
    if records.is_empty() {
        return Ok(Err(NoRecords));
    }

    let chars_scanned = records
        .iter()
        .map(|record| record.lemma.chars().count())
        .max()
        .and_then(|c| u64::try_from(c).ok())
        .unwrap_or_default();

    // let dictionary = wordbase_gtk::ui_for(
    //     |source| {
    //         dictionaries
    //             .get(&source)
    //             .map(|state| state.meta.name.as_str())
    //             .unwrap_or("?")
    //     },
    //     records,
    // );
    // popup.dictionary_container.set_child(Some(&dictionary));
    // popup.window.set_visible(true);

    Ok(Ok(ShowPopupResponse { chars_scanned }))
}

struct PopupInfo {
    window: gtk::Window,
    dictionary_container: adw::Bin,
}

fn create_popup(app: &adw::Application) -> PopupInfo {
    const MARGIN: i32 = 16;

    let content = gtk::ScrolledWindow::new();

    let dictionary_container = adw::Bin::builder()
        .margin_top(MARGIN)
        .margin_bottom(MARGIN)
        .margin_start(MARGIN)
        .margin_end(MARGIN)
        .build();
    content.set_child(Some(&dictionary_container));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .default_width(600)
        .default_height(300)
        .content(&content)
        .build();

    let controller = gtk::EventControllerMotion::new();
    window.add_controller(controller.clone());

    controller.connect_leave({
        let window = window.clone();
        move |_| {
            window.set_visible(false);
        }
    });

    PopupInfo {
        window: window.upcast(),
        dictionary_container,
    }
}
