use {
    crate::{APP_EVENTS, AppEvent, record_view::CUSTOM_THEME},
    anyhow::{Context, Result},
    derive_more::Deref,
    foldhash::{HashMap, HashMapExt},
    notify::{
        Watcher,
        event::{CreateKind, ModifyKind, RemoveKind, RenameMode},
    },
    relm4::SharedState,
    std::{
        path::Path,
        sync::{Arc, LazyLock},
    },
    tokio::fs,
    tracing::{info, warn},
};

#[derive(Debug, Clone)]
pub struct Theme {
    pub style: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ThemeKey {
    Default,
    Custom(ThemeName),
}

pub static DEFAULT_THEME: LazyLock<Arc<Theme>> = LazyLock::new(|| {
    Arc::new(Theme {
        style: include_str!("default_theme.css").to_string(),
    })
});

#[derive(Debug, Clone)]
pub struct CustomTheme {
    pub name: ThemeName,
    pub theme: Arc<Theme>,
}

pub static CUSTOM_THEMES: SharedState<HashMap<ThemeName, CustomTheme>> = SharedState::new();

impl CustomTheme {
    pub async fn read_from(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let name = ThemeName::from_path(path).context("invalid theme name")?;
        let style = fs::read_to_string(path)
            .await
            .context("failed to read theme file")?;
        Ok(Self {
            name: ThemeName(Arc::from(name.to_string())),
            theme: Arc::new(Theme { style }),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deref, Hash)]
pub struct ThemeName(pub Arc<str>);

impl ThemeName {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let name = path
            .as_ref()
            .file_stem()
            .context("file has no stem")?
            .to_str()
            .context("file name is not UTF-8")?;

        Ok(Self(Arc::from(name.to_string())))
    }
}

pub async fn watch_themes(data_path: &Path) -> Result<notify::RecommendedWatcher> {
    let themes_path = data_path.join("themes");
    fs::create_dir_all(&themes_path)
        .await
        .context("failed to create themes directory")?;

    let tokio = tokio::runtime::Handle::current();
    let mut watcher = notify::recommended_watcher(move |event| {
        tokio.spawn(async move {
            if let Err(err) = on_file_watcher_event(event).await {
                warn!("Theme file watcher error: {err:?}");
            }
        });
    })
    .context("failed to create file watcher")?;
    watcher
        .watch(&themes_path, notify::RecursiveMode::NonRecursive)
        .context("failed to start watching themes directory")?;

    let mut initial_themes = HashMap::new();
    let mut themes_dir = fs::read_dir(&themes_path)
        .await
        .context("failed to fetch initial themes")?;
    while let Some(theme) = themes_dir
        .next_entry()
        .await
        .context("failed to read entry under themes directory")?
    {
        let is_file = theme.file_type().await.is_ok_and(|ty| ty.is_file());
        if !is_file {
            continue;
        }

        let path = theme.path();
        let theme = CustomTheme::read_from(&path)
            .await
            .with_context(|| format!("failed to read `{path:?}`"))?;
        initial_themes.insert(theme.name.clone(), theme);
    }

    *CUSTOM_THEMES.write() = initial_themes;
    Ok(watcher)
}

async fn on_file_watcher_event(event: notify::Result<notify::Event>) -> Result<()> {
    let event = event.context("file watch error")?;
    match event.kind {
        notify::EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
            let mut paths = event.paths.into_iter();
            let from = ThemeName::from_path(paths.next().context("no rename `from` path")?)
                .context("invalid rename `from` theme name")?;
            let to = ThemeName::from_path(paths.next().context("no rename `to` path")?)
                .context("invalid rename `to` theme name")?;

            let mut themes = CUSTOM_THEMES.write();
            if let Some(theme) = themes.remove(&from) {
                themes.insert(to, theme.clone());
                drop(themes);
                _ = APP_EVENTS.send(AppEvent::ThemeRemoved(from));
                _ = APP_EVENTS.send(AppEvent::ThemeAdded(theme));
            }
        }
        notify::EventKind::Remove(RemoveKind::File)
        | notify::EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
            for path in event.paths {
                let name = ThemeName::from_path(&path)
                    .with_context(|| format!("invalid theme name `{path:?}`"))?;
                info!("{name:?} removed");
                CUSTOM_THEMES.write().remove(&name);
                _ = APP_EVENTS.send(AppEvent::ThemeRemoved(name));
            }
        }
        notify::EventKind::Create(CreateKind::File)
        | notify::EventKind::Modify(ModifyKind::Data(_) | ModifyKind::Name(RenameMode::To)) => {
            for path in event.paths {
                let theme = CustomTheme::read_from(&path)
                    .await
                    .with_context(|| format!("failed to read theme `{path:?}`"))?;
                info!("{:?} updated", theme.name);
                CUSTOM_THEMES
                    .write()
                    .insert(theme.name.clone(), theme.clone());
                _ = APP_EVENTS.send(AppEvent::ThemeAdded(theme));
            }
        }
        _ => {}
    }
    Ok(())
}
