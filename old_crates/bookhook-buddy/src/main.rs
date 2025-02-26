#![doc = include_str!("../README.md")]

extern crate gdk4 as gdk;
extern crate gtk4 as gtk;
extern crate libadwaita as adw;

mod exstatic;
mod popup;

use adw::{gio::prelude::*, glib::ExitCode, prelude::*};
use exstatic::NewSentence;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt().init();

    let application = adw::Application::builder()
        .application_id("com.github.aecsocket.BookhookBuddy")
        .build();

    let (send_exstatic_server_url, recv_exstatic_server_url) = mpsc::channel::<String>(1);
    let (send_new_sentence, recv_new_sentence) = mpsc::channel::<NewSentence>(4);

    application.connect_activate(move |app| {
        let prefs = {
            let page = adw::PreferencesPage::new();

            let exstatic = {
                let page = adw::PreferencesGroup::builder()
                    .title("exSTATic Settings")
                    .build();

                let server_url = adw::EntryRow::builder()
                    .title("Server URL")
                    .text("ws://127.0.0.1:9001")
                    .show_apply_button(true)
                    .build();
                server_url.connect_apply({
                    let send_exstatic_server_url = send_exstatic_server_url.clone();
                    move |this| {
                        _ = send_exstatic_server_url.try_send(this.text().to_string());
                    }
                });
                page.add(&server_url);

                page
            };
            page.add(&exstatic);

            page
        };

        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        content.append(&adw::HeaderBar::new());
        content.append(&prefs);

        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("First App")
            .default_width(350)
            // add content to window
            .content(&content)
            .build();
        window.present();
    });

    tokio::spawn(exstatic::run(recv_exstatic_server_url, send_new_sentence));
    tokio::spawn(popup::run(recv_new_sentence));
    application.run()
}
