#![doc = include_str!("../README.md")]

extern crate libadwaita as adw;
extern crate webkit6 as webkit;

mod ui;

use adw::{gio, glib, prelude::*};
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;

const APP_ID: &str = "com.github.aecsocket.Wordbase";

#[tokio::main]
async fn main() -> glib::ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();
    glib::log_set_default_handler(glib::rust_log_handler);

    let app = adw::Application::builder().application_id(APP_ID).build();
    info!("activated");

    app.connect_activate(|app| {
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .content(&ui::Settings::new())
            .build();
        window.present();
    });

    app.run()
}
