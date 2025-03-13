#![doc = include_str!("../README.md")]

extern crate libadwaita as adw;
extern crate webkit6 as webkit;

// mod manager;
mod overlay;
mod platform;
mod popup;

use std::sync::Arc;

use adw::{gio, glib, gtk, prelude::*};
use futures::TryFutureExt;
use platform::Platform;
use tokio::sync::broadcast;
use tracing::{error, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;
use wordbase_server::CHANNEL_BUF_CAP;

const APP_ID: &str = "com.github.aecsocket.Wordbase";

fn gettext(s: &str) -> &str {
    s
}

#[derive(Debug)]
struct Config {
    overlay_text_size: overlay::TextSize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            overlay_text_size: overlay::TextSize::Title2,
        }
    }
}

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

    let config = Arc::<Config>::default();
    let platform = Arc::<dyn Platform>::from(platform::default());
    let app = adw::Application::builder().application_id(APP_ID).build();
    let (send_event, _) = broadcast::channel::<wordbase_server::Event>(CHANNEL_BUF_CAP);
    let (overlays, overlay_task) = overlay::Client::new(overlay::State {
        config: config.clone(),
        platform: platform.clone(),
        app: app.clone(),
        recv_event: send_event.subscribe(),
    });

    glib::spawn_future_local(overlay_task.inspect_err(|err| error!("Overlay task error: {err:?}")));

    app.connect_activate(|app| {
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title(gettext("Wordbase"))
            .default_width(360)
            .default_height(600)
            .build();
        // window.set_content(Some(&content(window.upcast_ref())));
        window.present();
    });

    app.run()
}

/*
#[derive(Debug, Clone)]
struct ThemeMeta {
    name: String,
    version: String,
    authors: Vec<String>,
    description: Option<String>,
    url: Option<String>,
}

enum DictionaryImportState {
    ReadingMeta {
        file_name: String,
    },
    Parsing {
        meta: DictionaryMeta,
        total: u64,
        done: u64,
    },
    Inserting {
        meta: DictionaryMeta,
        total: u64,
        done: u64,
    },
}

fn jitendex_meta() -> DictionaryMeta {
    DictionaryMeta {
        name: "Jitendex.org [2025-02-11]".into(),
        version: "2025.02.11.0".into(),
        description: Some("Jitendex is updated with new content every week. Click the 'Check for Updates' button in the Yomitan 'Dictionaries' menu to upgrade to the latest version.\n\nIf Jitendex is useful for you, please consider giving the project a star on GitHub. You can also leave a tip on Ko-fi.\nVisit https://ko-fi.com/jitendex\n\nMany thanks to everyone who has helped to fund Jitendex.\n\n• epistularum\n• 昭玄大统\n• Maciej Jur\n• Ian Strandberg\n• Kip\n• Lanwara\n• Sky\n• Adam\n• Emanuel".into()),
        url: Some("https://jitendex.org".into()),
    }
}

fn jmnedict_meta() -> DictionaryMeta {
    DictionaryMeta {
        name: "JMnedict [2025-02-18]".into(),
        version: "JMnedict.2025-02-18".into(),
        description: None,
        url: Some("https://github.com/themoeway/yomitan-import".into()),
    }
}

fn dics() -> Vec<DictionaryState> {
    vec![
        DictionaryState {
            meta: jitendex_meta(),
            id: DictionaryId::default(),
            enabled: true,
            position: 0,
        },
        DictionaryState {
            meta: jmnedict_meta(),
            id: DictionaryId::default(),
            enabled: false,
            position: 1,
        },
        DictionaryState {
            meta: DictionaryMeta {
                name: "Empty".into(),
                version: "none".into(),
                description: None,
                url: None,
            },
            id: DictionaryId::default(),
            enabled: false,
            position: 1,
        },
    ]
}

fn imports() -> Vec<DictionaryImportState> {
    vec![
        DictionaryImportState::ReadingMeta {
            file_name: "jitendex-yomitan.zip".into(),
        },
        DictionaryImportState::Parsing {
            meta: jitendex_meta(),
            total: 151,
            done: 72,
        },
        DictionaryImportState::Inserting {
            meta: jmnedict_meta(),
            total: 310_000,
            done: 225_000,
        },
    ]
}

fn default_theme() -> ThemeMeta {
    ThemeMeta {
        name: "Adwaita".into(),
        version: "1.0.0".into(),
        authors: vec!["Wordbase".into()],
        description: Some("Default GNOME Adwaita theme".into()),
        url: None,
    }
}

fn user_themes() -> Vec<ThemeMeta> {
    vec![
        ThemeMeta {
            name: "ClearVision".into(),
            version: "0.1.0".into(),
            authors: vec!["ClearVision Team".into()],
            description: Some("The cool theme".into()),
            url: None,
        },
        ThemeMeta {
            name: "Empty".into(),
            version: "none".into(),
            authors: vec![],
            description: None,
            url: None,
        },
    ]
}

fn content(window: &gtk::Window) -> gtk::Widget {
    let ui = ui::Overview::new();

    for dic in &dics() {
        ui.dictionaries().add(&ui::dictionary_row(dic));
    }

    for import in &imports() {
        ui.dictionaries().add(&ui::dictionary_import_row(import));
    }

    let import = adw::ButtonRow::builder()
        .title("Import")
        .start_icon_name("list-add-symbolic")
        .build();
    ui.dictionaries().add(&import);

    let window = window.clone();
    import.connect_activated(move |_| {
        let window = window.clone();
        glib::spawn_future_local(async move {
            let result = gtk::FileDialog::builder()
                .accept_label("Import")
                .build()
                .open_multiple_future(Some(&window))
                .await;
            println!("{result:?}");
        });
    });

    let (row, default_theme_selection) = ui::theme_row::<false>(&default_theme());
    ui.themes().add(&row);
    default_theme_selection.set_active(true);

    for theme in &user_themes() {
        let (row, user_theme_selection) = ui::theme_row::<true>(theme);
        ui.themes().add(&row);
        user_theme_selection.set_group(Some(&default_theme_selection));
    }

    let import = adw::ButtonRow::builder()
        .title("Import")
        .start_icon_name("list-add-symbolic")
        .build();
    ui.themes().add(&import);

    let search_view = webkit::WebView::new();
    ui.search_content().set_child(Some(&search_view));
    let view_context = search_view.context().expect("web view should have context");
    view_context.set_cache_model(webkit::CacheModel::DocumentViewer);
    search_view.load_html("<h1>Hello world</h1>", None);

    ui.upcast()
}
*/
