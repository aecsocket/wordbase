use {
    anyhow::{Context, Result},
    notify::{
        Watcher,
        event::{DataChange, ModifyKind},
    },
    std::{
        env,
        path::Path,
        sync::{Arc, LazyLock},
    },
    tokio::fs,
    tracing::{info, warn},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    pub style: String,
}

#[derive(Debug)]
pub struct DefaultTheme {
    pub theme: Arc<Theme>,
    pub watcher_factory: WatcherFactory,
}

#[derive(Debug)]
pub struct WatcherFactory {
    workspace_theme_path: Option<Arc<Path>>,
}

impl WatcherFactory {
    pub fn create(
        self,
        on_new_theme: impl Fn(Theme) + Send + Sync + 'static,
    ) -> Result<Option<notify::RecommendedWatcher>> {
        let Some(workspace_theme_path) = self.workspace_theme_path else {
            return Ok(None);
        };

        let mut watcher = notify::recommended_watcher({
            let tokio = tokio::runtime::Handle::current();
            let workspace_theme_path = workspace_theme_path.clone();
            let on_new_theme = Arc::new(on_new_theme);
            move |event: notify::Result<notify::Event>| {
                let event = match event {
                    Ok(event) => event,
                    Err(err) => {
                        warn!("File watcher error: {err:?}");
                        return;
                    }
                };

                let notify::EventKind::Modify(ModifyKind::Data(DataChange::Any)) = event.kind
                else {
                    return;
                };

                let workspace_theme_path = workspace_theme_path.clone();
                let on_new_theme = on_new_theme.clone();
                tokio.spawn(async move {
                    let style = match fs::read_to_string(&workspace_theme_path).await {
                        Ok(css) => css,
                        Err(err) => {
                            warn!("Failed to read default theme file: {err:?}");
                            return;
                        }
                    };

                    on_new_theme(Theme { style });
                });
            }
        })
        .context("failed to create watcher")?;
        watcher
            .watch(&workspace_theme_path, notify::RecursiveMode::NonRecursive)
            .context("failed to start watching file")?;
        Ok(Some(watcher))
    }
}

pub async fn default_theme() -> Result<DefaultTheme> {
    const THEME_PATH: &str = "default_theme.css";
    const THEME_DATA: &str = include_str!("default_theme.css");

    static THEME_ARC: LazyLock<Arc<Theme>> = LazyLock::new(|| {
        Arc::new(Theme {
            style: THEME_DATA.into(),
        })
    });

    static WORKSPACE_THEME_PATH: LazyLock<Option<Arc<Path>>> = LazyLock::new(|| {
        let manifest_path = env::var("CARGO_MANIFEST_DIR").ok()?;
        let workspace_theme_path =
            Arc::<Path>::from(Path::new(&manifest_path).join("src").join(THEME_PATH));
        info!("Watching {workspace_theme_path:?} for changes");
        Some(workspace_theme_path)
    });

    let Some(workspace_theme_path) = &*WORKSPACE_THEME_PATH else {
        // hardcoded default theme
        return Ok(DefaultTheme {
            theme: THEME_ARC.clone(),
            watcher_factory: WatcherFactory {
                workspace_theme_path: None,
            },
        });
    };

    // dynamic default theme
    let style = fs::read_to_string(workspace_theme_path)
        .await
        .context("failed to read initial default theme CSS")?;
    Ok(DefaultTheme {
        theme: Arc::new(Theme { style }),
        watcher_factory: WatcherFactory {
            workspace_theme_path: Some(workspace_theme_path.clone()),
        },
    })
}
