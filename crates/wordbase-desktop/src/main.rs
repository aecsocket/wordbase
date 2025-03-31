#![doc = include_str!("../README.md")]
#![allow(clippy::future_not_send, reason = "`gtk` types aren't `Send`")]

// mod popup;
mod render;

use {
    anyhow::{Context, Result},
    directories::ProjectDirs,
    foldhash::HashMap,
    notify::{
        Watcher,
        event::{DataChange, ModifyKind},
    },
    relm4::{
        adw::{self, gdk, prelude::*},
        loading_widgets::LoadingWidgets,
        prelude::*,
        view,
    },
    tokio::fs,
    tracing::{info, level_filters::LevelFilter},
    tracing_subscriber::EnvFilter,
    webkit6::prelude::WebViewExt,
    wordbase::{Dictionary, DictionaryId, RecordLookup},
    wordbase_engine::Engine,
};

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    RelmApp::new("io.github.aecsocket.Wordbase").run_async::<App>(());
}

#[derive(Debug)]
struct App {
    engine: Engine,
    _themes_watcher: notify::RecommendedWatcher,
    theme_css: String,
    web_view: webkit6::WebView,
    dictionaries: HashMap<DictionaryId, Dictionary>,
    records: Vec<RecordLookup>,
}

#[derive(Debug)]
enum AppMsg {
    SetThemeCss { theme_css: String },
    Search { query: String },
}

#[relm4::component(async)]
impl AsyncComponent for App {
    type Init = ();
    type Input = AppMsg;
    type Output = ();
    type CommandOutput = ();

    view! {
        adw::Window {
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 16,

                gtk::SearchEntry {
                    set_margin_top: 16,
                    set_margin_bottom: 16,
                    set_margin_start: 16,
                    set_margin_end: 16,
                    set_valign: gtk::Align::Start,
                    connect_search_changed => move |widget| {
                        sender.input(AppMsg::Search { query: widget.text().into() });
                    },
                },

                #[name(web_view)]
                webkit6::WebView {
                    set_hexpand: true,
                    set_vexpand: true,
                    set_background_color: &gdk::RGBA::new(0.0, 0.0, 0.0, 0.0),
                }
            }
        }
    }

    fn init_loading_widgets(root: Self::Root) -> Option<LoadingWidgets> {
        view! {
            #[local]
            root {
                set_title: Some("Wordbase"),

                #[name(spinner)]
                adw::Spinner {}
            }
        }
        Some(LoadingWidgets::new(root, spinner))
    }

    async fn init(
        (): Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let init = init(sender.clone()).await.unwrap();
        let widgets = view_output!();
        let model = Self {
            engine: init.engine,
            _themes_watcher: init.themes_watcher,
            theme_css: init.theme_css,
            dictionaries: init.dictionaries,
            web_view: widgets.web_view.clone(),
            records: Vec::new(),
        };
        AsyncComponentParts { model, widgets }
    }
}

#[derive(Debug)]
struct AppInit {
    engine: Engine,
    dictionaries: HashMap<DictionaryId, Dictionary>,
    themes_watcher: notify::RecommendedWatcher,
    theme_css: String,
}

async fn init(sender: AsyncComponentSender<App>) -> Result<AppInit> {
    let tokio = tokio::runtime::Handle::current();
    let dirs = ProjectDirs::from("io.github", "aecsocket", "Wordbase")
        .context("failed to get default app directories")?;
    let data_path = dirs.data_dir();
    info!("Using {data_path:?} as data path");
    fs::create_dir_all(data_path)
        .await
        .context("failed to create data directory")?;

    let db_path = data_path.join("wordbase.db");
    let (engine, engine_task) = Engine::new(db_path).await?;
    tokio::spawn(async move {
        engine_task.await.expect("engine error");
    });
    let dictionaries = engine
        .dictionaries()
        .await
        .context("failed to fetch initial dictionaries")?
        .into_iter()
        .map(|dict| (dict.id, dict))
        .collect();

    let themes_path = data_path.join("themes");
    fs::create_dir_all(&themes_path)
        .await
        .context("failed to create themes directory")?;

    let mut themes_watcher =
        notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
            let Ok(event) = event else { return };
            tokio.spawn(on_fs_theme_event(sender.clone(), event));
        })
        .context("failed to create themes file watcher")?;
    themes_watcher
        .watch(&themes_path, notify::RecursiveMode::NonRecursive)
        .context("failed to start watching themes directory")?;
    let theme_css = fs::read_to_string(themes_path.join("default.css"))
        .await
        .context("failed to read default theme CSS")?;

    Ok(AppInit {
        engine,
        dictionaries,
        themes_watcher,
        theme_css,
    })
}

async fn on_fs_theme_event(sender: AsyncComponentSender<App>, event: notify::Event) {
    match event.kind {
        notify::EventKind::Modify(ModifyKind::Data(DataChange::Any)) => {
            info!("{event:?}");
        }
        _ => {}
    }
}

const CSS: &str = r#"
/* Base container styling */
.term-box {
  border: 1px solid #e1e4e8;
  border-radius: 8px;
  padding: 12px;
  margin-bottom: 16px;
  background: white;
  box-shadow: 0 1px 3px rgba(0,0,0,0.05);
}

/* Term text styling (top left) */
.term {
  font-size: 1.5rem;
  font-weight: 500;
  margin-bottom: 8px;
}

/* Ruby/furigana styling */
.term rt {
  font-size: 0.7em;
  opacity: 0.8;
}

/* Frequency tags container */
.frequency-box {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin: 8px 0;
}

/* Individual frequency tag */
.frequency {
  background: #f0f3f7;
  border-radius: 16px;
  padding: 4px 10px;
  font-size: 0.85rem;
  display: flex;
  align-items: center;
  gap: 4px;
}

.frequency .source {
  color: #586069;
  font-weight: 500;
}

.frequency .value {
  color: #24292e;
  font-weight: 600;
}

/* Glossaries container */
.source-glossaries-box {
  margin-top: 12px;
}

/* Dictionary source header */
.source-name {
  display: block;
  font-size: 0.8rem;
  color: #586069;
  margin-bottom: 8px;
  padding-bottom: 4px;
  border-bottom: 1px solid #eaecef;
}

/* Glossary entry */
.glossary {
  margin-bottom: 12px;
}

/* Tags styling */
.tag {
  display: inline-block;
  background: #e1f5fe;
  color: #0288d1;
  border-radius: 4px;
  padding: 2px 6px;
  font-size: 0.75rem;
  margin-right: 6px;
  margin-bottom: 4px;
}

/* Definition lists */
.glossary ul {
  padding-left: 1.2em;
  margin: 8px 0;
}

.glossary li {
  margin-bottom: 4px;
  line-height: 1.5;
}

/* Example sentences */
.glossary div[style*="background-color:color-mix"] {
  margin: 8px 0;
  padding: 10px;
  border-left: 3px solid #1a73e8;
  background-color: #f8f9fa !important;
}

/* Kanji variants table */
table {
  border-collapse: collapse;
  margin: 8px 0;
  width: 100%;
}

table th, table td {
  padding: 6px;
  text-align: left;
  border-bottom: 1px solid #eaecef;
}

/* Responsive adjustments */
@media (max-width: 600px) {
  .term {
    font-size: 1.3rem;
  }

  .frequency-box {
    gap: 4px;
  }

  .frequency {
    font-size: 0.75rem;
    padding: 2px 8px;
  }
}

/* Special styling for priority tags */
.tag[title*="priority"] {
  background: #fff8e1;
  color: #ff8f00;
}

/* Auxiliary verb styling */
.tag[title="auxiliary verb"] {
  background: #f3e5f5;
  color: #8e24aa;
}

/* Archaic marker */
.tag[title="archaic"] {
  background: #efebe9;
  color: #6d4c41;
}

/* Verb type indicators */
.tag[title*="verb"] {
  background: #e8f5e9;
  color: #2e7d32;
}
"#;
