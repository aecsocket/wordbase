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

mod dictionary;

use adw::prelude::*;
use gtk::gdk;

fn main() {
    let app = adw::Application::builder()
        .application_id("com.github.aecsocket.WordbasePopup")
        .build();

    app.connect_startup(|_| {
        let css_provider = gtk::CssProvider::new();
        css_provider.load_from_string(include_str!("style.css"));

        gtk::style_context_add_provider_for_display(
            &gdk::Display::default().unwrap(),
            &css_provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    });

    app.connect_activate(|app| {
        let view = gtk::ScrolledWindow::new();

        let contents = adw::Bin::builder()
            .margin_top(16)
            .margin_bottom(16)
            .margin_start(16)
            .margin_end(16)
            .build();
        view.set_child(Some(&contents));

        let dictionary = dictionary::Dictionary::new();
        contents.set_child(Some(&dictionary));

        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Dictionary")
            .content(&view)
            .build();
        window.present();
    });

    app.run();
}
