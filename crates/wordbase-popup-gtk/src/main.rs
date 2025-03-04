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

fn main() {
    let application = adw::Application::builder()
        .application_id("com.github.aecsocket.WordbasePopup")
        .build();

    application.connect_activate(|application| {
        let dictionary = dictionary::Dictionary::new();

        let list = gtk::ListBox::new();
        list.append(&gtk::Label::builder().label("Hello world!").build());
        list.append(&dictionary);
        list.append(&gtk::Label::builder().label("Goodbye world").build());

        let window = adw::ApplicationWindow::builder()
            .application(application)
            .title("Dictionary")
            .content(&list)
            .build();
        window.present();
    });

    application.run();
}
