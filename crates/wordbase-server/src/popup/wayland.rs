extern crate gtk4 as gtk;
extern crate libadwaita as adw;

use anyhow::{Context, Result};
use foldhash::{HashMap, HashMapExt};
use futures::never::Never;
use gtk4::{
    gdk,
    gio::{ApplicationHoldGuard, prelude::*},
    prelude::*,
};
use libadwaita::prelude::BinExt;
use sqlx::{Pool, Sqlite};
use tokio::sync::broadcast;
use tracing::{error, info, warn};
use wordbase::{DictionaryId, DictionaryState, protocol::ShowPopupRequest};

use crate::{ServerEvent, term};

const APP_ID: &str = "com.github.aecsocket.Wordbase";

pub fn run(
    db: Pool<Sqlite>,
    rt: tokio::runtime::Handle,
    recv_popup_request: broadcast::Receiver<ShowPopupRequest>,
    recv_server_event: broadcast::Receiver<ServerEvent>,
) -> Result<()> {
    info!("Using Wayland popup backend via GTK/Adwaita");
    glib::log_set_default_handler(glib::rust_log_handler);

    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_startup(|_| {
        let provider = gtk::CssProvider::new();
        provider.load_from_string(wordbase_gtk::STYLESHEET);

        gtk::style_context_add_provider_for_display(
            &gdk::Display::default().expect("failed to get display"),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    });
    app.connect_activate(move |app| {
        let db = db.clone();
        let rt = rt.clone();
        let recv_popup_request = recv_popup_request.resubscribe();
        let recv_server_event = recv_server_event.resubscribe();
        let app = app.clone();
        // make the app not close when all its windows are closed
        let hold_guard = app.hold();
        glib::spawn_future_local(async move {
            let Err(err) = backend(
                db,
                rt,
                recv_popup_request,
                recv_server_event,
                app,
                hold_guard,
            )
            .await;
            error!("GTK app backend closed: {err:?}");
        });
    });

    app.run();
    error!("GTK application closed - the main server is probably about to close");
    Ok(())
}

async fn backend(
    db: Pool<Sqlite>,
    rt: tokio::runtime::Handle,
    mut recv_popup_request: broadcast::Receiver<ShowPopupRequest>,
    mut recv_server_event: broadcast::Receiver<ServerEvent>,
    app: adw::Application,
    _hold_guard: ApplicationHoldGuard,
) -> Result<Never> {
    let mut dictionaries = HashMap::<DictionaryId, DictionaryState>::new();
    loop {
        let request = tokio::select! {
            r = recv_popup_request.recv() => r,
            Ok(ServerEvent::SyncDictionaries(new_dictionaries)) = recv_server_event.recv() => {
                dictionaries = new_dictionaries
                    .into_iter()
                    .map(|state| (state.id, state))
                    .collect();
                continue;
            }
        };
        let request = request.context("popup request channel closed")?;

        if let Err(err) = handle_request(db.clone(), &rt, &app, &dictionaries, request).await {
            warn!("Failed to handle popup request: {err:?}");
        }
    }
}

async fn handle_request(
    db: Pool<Sqlite>,
    rt: &tokio::runtime::Handle,
    app: &adw::Application,
    dictionaries: &HashMap<DictionaryId, DictionaryState>,
    request: ShowPopupRequest,
) -> Result<()> {
    const MARGIN: i32 = 16;

    let records = rt
        .spawn(async move {
            term::lookup(&db, &request.text, wordbase_gtk::SUPPORTED_RECORD_KINDS).await
        })
        .await
        .context("fetch record task dropped")?
        .context("failed to fetch records")?;

    let content = gtk::ScrolledWindow::new();

    let dictionary_container = adw::Bin::builder()
        .margin_top(MARGIN)
        .margin_bottom(MARGIN)
        .margin_start(MARGIN)
        .margin_end(MARGIN)
        .build();
    content.set_child(Some(&dictionary_container));

    let dictionary = wordbase_gtk::ui_for(
        |source| {
            dictionaries
                .get(&source)
                .map(|state| state.meta.name.as_str())
                .unwrap_or("?")
        },
        records,
    );
    dictionary_container.set_child(Some(&dictionary));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .default_width(600)
        .default_height(300)
        .content(&content)
        .build();
    window.present();

    Ok(())
}
