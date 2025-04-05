use {
    crate::CHANNEL_BUF_CAP,
    anyhow::{Context, Result},
    notify::{
        Watcher,
        event::{DataChange, ModifyKind},
    },
    std::{
        env,
        path::{Path, PathBuf},
        sync::{Arc, OnceLock},
    },
    tokio::{fs, sync::broadcast},
    tracing::{info, warn},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    pub style: String,
}

pub type SharedTheme = Arc<Theme>;

pub async fn default() -> SharedTheme {
    default_watcher().await.current.clone()
}

pub async fn recv_default_changed() -> broadcast::Receiver<SharedTheme> {
    default_watcher().await.recv_theme.resubscribe()
}

struct DefaultThemeWatcher {
    current: SharedTheme,
    recv_theme: broadcast::Receiver<SharedTheme>,
    _file_watcher: Option<notify::RecommendedWatcher>,
}

const DEFAULT_THEME_PATH: &str = "default_theme.css";
const DEFAULT_THEME_DATA: &str = include_str!("default_theme.css");
static DEFAULT_THEME_WATCHER: OnceLock<DefaultThemeWatcher> = OnceLock::new();

async fn default_watcher() -> &'static DefaultThemeWatcher {
    if let Some(watcher) = DEFAULT_THEME_WATCHER.get() {
        return watcher;
    }

    let (send_theme, recv_theme) = broadcast::channel(CHANNEL_BUF_CAP);
    if let Some(default_theme_src_path) = default_theme_src_path() {
        info!("Watching {default_theme_src_path:?} for changes");

        // we're running from source, so the developer can hot reload the theme
        let default_theme_src_path = Arc::<Path>::from(default_theme_src_path);
        let file_watcher = create_default_theme_watcher(&default_theme_src_path, send_theme)
            .unwrap_or_else(|err| {
                warn!("Failed to create default theme file watcher: {err:?}");
                None
            });

        let style = match fs::read_to_string(&default_theme_src_path).await {
            Ok(style) => style,
            Err(err) => {
                warn!(
                    "Failed to read default theme from source, falling back to hardcoded: {err:?}"
                );
                DEFAULT_THEME_DATA.to_owned()
            }
        };
        let current = Theme { style };
        DEFAULT_THEME_WATCHER.get_or_init(|| DefaultThemeWatcher {
            current: Arc::new(current),
            recv_theme,
            _file_watcher: file_watcher,
        })
    } else {
        let current = Theme {
            style: DEFAULT_THEME_DATA.to_owned(),
        };
        DEFAULT_THEME_WATCHER.get_or_init(|| DefaultThemeWatcher {
            current: Arc::new(current),
            recv_theme,
            _file_watcher: None,
        })
    }
}

fn default_theme_src_path() -> Option<PathBuf> {
    env::var("CARGO_MANIFEST_DIR").ok().map(|manifest_path| {
        Path::new(&manifest_path)
            .join("src")
            .join(DEFAULT_THEME_PATH)
    })
}

fn create_default_theme_watcher(
    default_theme_src_path: &Arc<Path>,
    send_theme: broadcast::Sender<Arc<Theme>>,
) -> Result<Option<notify::RecommendedWatcher>> {
    let tokio = tokio::runtime::Handle::current();
    let mut watcher = notify::recommended_watcher({
        let default_theme_src_path = default_theme_src_path.clone();
        move |event: notify::Result<notify::Event>| {
            let event = match event {
                Ok(event) => event,
                Err(err) => {
                    warn!("Default theme file watcher error: {err:?}");
                    return;
                }
            };

            let notify::EventKind::Modify(ModifyKind::Data(DataChange::Any)) = event.kind else {
                return;
            };

            let default_theme_src_path = default_theme_src_path.clone();
            let send_theme = send_theme.clone();
            tokio.spawn(async move {
                let style = match fs::read_to_string(&default_theme_src_path).await {
                    Ok(css) => css,
                    Err(err) => {
                        warn!("Failed to read default theme file: {err:?}");
                        return;
                    }
                };
                let theme = Arc::new(Theme { style });
                _ = send_theme.send(theme);
            });
        }
    })
    .context("failed to create watcher")?;
    watcher
        .watch(default_theme_src_path, notify::RecursiveMode::NonRecursive)
        .context("failed to start watching file")?;
    Ok(Some(watcher))
}
