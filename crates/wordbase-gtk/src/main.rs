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

    app.connect_activate(|app| {
        let content = webkit::WebView::builder()
            .hexpand(true)
            .vexpand(true)
            .build();
        content.load_html(
            r##"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>WebView Test</title>
    <style>
        body {
            font-family: "Cantarell", sans-serif;
            background-color: #f6f5f4;
            color: #333;
            margin: 40px;
            padding: 20px;
            max-width: 800px;
        }
        h1, h2, h3 {
            color: #3a3a3a;
        }
        p {
            line-height: 1.6;
        }
        .card {
            background: white;
            padding: 20px;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
            margin-bottom: 20px;
        }
        button {
            background: #3584e4;
            color: white;
            padding: 10px 15px;
            border: none;
            border-radius: 4px;
            cursor: pointer;
        }
        button:hover {
            background: #2a6eb8;
        }
        a {
            color: #0078d4;
            text-decoration: none;
        }
        a:hover {
            text-decoration: underline;
        }
    </style>
</head>
<body>
    <h1>WebView Styling Test</h1>
    <p>This is a sample HTML file for testing rendering inside a GTK WebView.</p>

    <div class="card">
        <h2>Card Title</h2>
        <p>Some descriptive text inside a card component. Adwaita styling applied.</p>
        <button>Click Me</button>
    </div>

    <div class="card">
        <h3>Another Section</h3>
        <p>More sample content with a <a href="#">test link</a>.</p>
    </div>

    <p>More text content below to check scrolling behavior. Lorem ipsum dolor sit amet, consectetur adipiscing elit. Proin vitae orci eget justo consequat suscipit. Morbi malesuada leo nec urna cursus, nec vestibulum orci feugiat. Integer nec sapien ut libero feugiat ultricies id ut lorem. Suspendisse a massa at justo varius sodales id non augue.</p>
    <p>Additional paragraph for more vertical space testing.</p>
</body>
</html>
"##,
            None,
        );

        adw::ApplicationWindow::builder()
            .application(app)
            .content(&content)
            .build()
            .present();
    });

    app.run();
}
