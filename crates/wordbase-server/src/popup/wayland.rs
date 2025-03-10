extern crate gtk4 as gtk;
extern crate libadwaita as adw;
extern crate wordbase_gtk_ui as ui;

use anyhow::{Context, Result};
use futures::never::Never;
use gtk4::{
    gio::{ApplicationHoldGuard, prelude::*},
    prelude::*,
};
use sqlx::{Pool, Sqlite};
use tokio::sync::broadcast;
use tracing::{info, warn};
use wordbase::protocol::ShowPopupRequest;

use crate::term;

const APP_ID: &str = "com.github.aecsocket.Wordbase";

pub fn run(
    db: Pool<Sqlite>,
    rt: tokio::runtime::Handle,
    recv_popup_request: broadcast::Receiver<ShowPopupRequest>,
) -> Result<Never> {
    info!("Using Wayland popup backend via GTK/Adwaita");
    glib::log_set_default_handler(glib::rust_log_handler);

    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(move |app| {
        let db = db.clone();
        let rt = rt.clone();
        let recv_popup_request = recv_popup_request.resubscribe();
        let app = app.clone();
        // make the app not close when all its windows are closed
        let hold_guard = app.hold();
        glib::spawn_future_local(backend(db, rt, recv_popup_request, app, hold_guard));
    });

    app.run();
    panic!("GTK application closed");
}

async fn backend(
    db: Pool<Sqlite>,
    rt: tokio::runtime::Handle,
    mut recv_popup_request: broadcast::Receiver<ShowPopupRequest>,
    app: adw::Application,
    _hold_guard: ApplicationHoldGuard,
) -> Result<Never> {
    loop {
        let request = recv_popup_request
            .recv()
            .await
            .context("popup request channel closed")?;

        if let Err(err) = handle_request(db.clone(), &rt, &app, request).await {
            warn!("Failed to handle popup request: {err:?}");
        }
    }
}

async fn handle_request(
    db: Pool<Sqlite>,
    rt: &tokio::runtime::Handle,
    app: &adw::Application,
    request: ShowPopupRequest,
) -> Result<()> {
    let records = rt
        .spawn(async move {
            term::lookup(&db, &request.text, ui::Dictionary::SUPPORTED_RECORD_KINDS).await
        })
        .await
        .context("fetch record task dropped")?
        .context("failed to fetch records")?;

    // let content = ui::Dictionary::from(&records);
    let content = gtk::Label::new(Some("hello world"));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .default_width(600)
        .default_height(300)
        .content(&content)
        .build();
    window.present();

    Ok(())
}
