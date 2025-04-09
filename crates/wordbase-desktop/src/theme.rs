use {
    crate::AppMsg,
    anyhow::{Context, Result},
    derive_more::Deref,
    foldhash::{HashMap, HashMapExt},
    glib::clone,
    notify::{
        Watcher,
        event::{CreateKind, ModifyKind, RemoveKind},
    },
    std::{
        path::Path,
        sync::{Arc, LazyLock},
    },
    tokio::fs,
    tracing::warn,
};

#[derive(Debug, Clone)]
pub struct Theme {
    pub style: String,
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

impl CustomTheme {
    pub async fn read_from(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let name = ThemeName::from_path(path).context("invalid theme name")?;
        let style = fs::read_to_string(path)
            .await
            .context("failed to read theme file")?;
        Ok(CustomTheme {
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

        Ok(ThemeName(Arc::from(name.to_string())))
    }
}

pub async fn watch_themes(
    data_dir: &Path,
    sender: relm4::Sender<AppMsg>,
) -> Result<(HashMap<ThemeName, CustomTheme>, notify::RecommendedWatcher)> {
    let themes_dir = data_dir.join("themes");
    fs::create_dir_all(&themes_dir)
        .await
        .context("failed to create themes directory")?;

    let tokio = tokio::runtime::Handle::current();
    let mut watcher = notify::recommended_watcher(move |event| {
        tokio.spawn(clone!(
            #[strong]
            sender,
            async move {
                if let Err(err) = on_file_watcher_event(event, sender).await {
                    warn!("Theme file watcher error: {err:?}");
                }
            }
        ));
    })
    .context("failed to create file watcher")?;
    watcher
        .watch(&themes_dir, notify::RecursiveMode::NonRecursive)
        .context("failed to start watching themes directory")?;

    let mut initial_themes = HashMap::new();
    let mut themes_dir = fs::read_dir(&themes_dir)
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

    Ok((initial_themes, watcher))
}

async fn on_file_watcher_event(
    event: notify::Result<notify::Event>,
    sender: relm4::Sender<AppMsg>,
) -> Result<()> {
    let event = event.context("file watch error")?;

    match event.kind {
        notify::EventKind::Create(CreateKind::File)
        | notify::EventKind::Modify(ModifyKind::Any) => {
            for path in event.paths {
                let theme = CustomTheme::read_from(&path)
                    .await
                    .with_context(|| format!("failed to read theme `{path:?}`"))?;
                sender.emit(AppMsg::ThemeInsert(theme));
            }
        }
        notify::EventKind::Remove(RemoveKind::File) => {
            for path in event.paths {
                let name = ThemeName::from_path(&path)
                    .with_context(|| format!("invalid theme name `{path:?}`"))?;
                sender.emit(AppMsg::ThemeRemove(name));
            }
        }
        _ => {}
    }

    Ok(())
}
