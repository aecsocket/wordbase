//! foo

extern crate gtk4 as gtk;
extern crate libadwaita as adw;
extern crate webkit6 as webkit;

use gtk::{gio::prelude::*, prelude::*};
use webkit::prelude::WebViewExt;

fn main() {
    let app = adw::Application::builder()
        .application_id("com.git.Ok")
        .build();

    // app.connect_activate(|app| {
    //     let content = webkit::WebView::builder()
    //         .hexpand(true)
    //         .vexpand(true)
    //         .build();
    //     content.load_html(

    //         None,
    //     );

    //     adw::ApplicationWindow::builder()
    //         .application(app)
    //         .content(&content)
    //         .build()
    //         .present();
    // });

    app.run();
}
