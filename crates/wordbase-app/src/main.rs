#![doc = include_str!("../README.md")]

extern crate libadwaita as adw;
extern crate webkit6 as webkit;

mod ui;

use adw::{gio, glib, gtk, prelude::*};
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;
use wordbase::{DictionaryId, DictionaryMeta, DictionaryState};

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

    app.connect_activate(|app| {
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title("Wordbase")
            .content(&content())
            .build();
        window.present();
    });

    app.run()
}

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
        authors: vec!["Stephen Kraus".into()],
        description: Some("Jitendex is updated with new content every week. Click the 'Check for Updates' button in the Yomitan 'Dictionaries' menu to upgrade to the latest version.\n\nIf Jitendex is useful for you, please consider giving the project a star on GitHub. You can also leave a tip on Ko-fi.\nVisit https://ko-fi.com/jitendex\n\nMany thanks to everyone who has helped to fund Jitendex.\n\n• epistularum\n• 昭玄大统\n• Maciej Jur\n• Ian Strandberg\n• Kip\n• Lanwara\n• Sky\n• Adam\n• Emanuel".into()),
        url: Some("https://jitendex.org".into()),
    }
}

fn jmnedict_meta() -> DictionaryMeta {
    DictionaryMeta {
        name: "JMnedict [2025-02-18]".into(),
        version: "JMnedict.2025-02-18".into(),
        authors: vec!["yomitan-import".into()],
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
                authors: vec![],
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

fn content() -> gtk::Widget {
    let content = ui::Overview::new();

    for dic in &dics() {
        content.dictionaries().add(&ui::dictionary_row(dic));
    }

    for import in &imports() {
        content
            .dictionaries()
            .add(&ui::dictionary_import_row(import));
    }

    let import = adw::ButtonRow::builder()
        .title("Import")
        .start_icon_name("list-add-symbolic")
        .build();
    content.dictionaries().add(&import);

    let (row, default_theme_selection) = ui::theme_row::<false>(&default_theme());
    content.themes().add(&row);
    default_theme_selection.set_active(true);

    for theme in &user_themes() {
        let (row, user_theme_selection) = ui::theme_row::<true>(theme);
        content.themes().add(&row);
        user_theme_selection.set_group(Some(&default_theme_selection));
    }

    let import = adw::ButtonRow::builder()
        .title("Import")
        .start_icon_name("list-add-symbolic")
        .build();
    content.themes().add(&import);

    content.upcast()
}
