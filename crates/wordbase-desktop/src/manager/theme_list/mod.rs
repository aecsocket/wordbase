mod ui;

use {
    super::theme_row,
    crate::{
        APP_EVENTS, AppEvent, forward_events, gettext,
        theme::{CUSTOM_THEMES, ThemeKey, ThemeName},
        toast_result,
    },
    anyhow::{Context, Result},
    foldhash::{HashMap, HashMapExt},
    gtk4::prelude::{CheckButtonExt, ListBoxRowExt},
    relm4::{
        adw::{glib::clone, gtk::pango, prelude::*},
        prelude::*,
    },
    wordbase_engine::{Engine, profile::ProfileConfig},
};

#[derive(Debug)]
pub struct Model {
    _default_theme: AsyncController<theme_row::Model>,
    custom_themes: HashMap<ThemeName, AsyncController<theme_row::Model>>,
    engine: Engine,
    window: gtk::Window,
    toaster: adw::ToastOverlay,
}

#[derive(Debug)]
#[doc(hidden)]
pub enum Msg {
    AskSetFont,
    ResetFont,
}

impl AsyncComponent for Model {
    type Init = (Engine, gtk::Window, adw::ToastOverlay);
    type Input = Msg;
    type Output = ();
    type CommandOutput = AppEvent;
    type Root = ui::ThemeList;
    type Widgets = ();

    fn init_root() -> Self::Root {
        ui::ThemeList::new()
    }

    async fn init(
        (engine, window, toaster): Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        forward_events(&sender);

        root.font_row().connect_activated(clone!(
            #[strong]
            sender,
            move |_| sender.input(Msg::AskSetFont)
        ));
        root.font_reset().connect_clicked(clone!(
            #[strong]
            sender,
            move |_| sender.input(Msg::ResetFont)
        ));

        let default_theme = theme_row::Model::builder()
            .launch((window.clone(), ThemeKey::Default))
            .detach();
        default_theme
            .widget()
            .enabled()
            .set_group(Some(&root.enabled_dummy()));
        root.list()
            .insert(default_theme.widget(), root.import_button().index());

        let mut model = Self {
            _default_theme: default_theme,
            custom_themes: HashMap::new(),
            engine,
            window,
            toaster,
        };
        for name in CUSTOM_THEMES.read().keys() {
            add_row(&mut model, &root, name.clone());
        }
        show_font(&model, &root);
        AsyncComponentParts { model, widgets: () }
    }

    async fn update_with_view(
        &mut self,
        _widgets: &mut Self::Widgets,
        message: Self::Input,
        _sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        toast_result(
            &self.toaster,
            match message {
                Msg::AskSetFont => set_font(self)
                    .await
                    .with_context(|| gettext("Failed to set font")),
                Msg::ResetFont => reset_font(self)
                    .await
                    .with_context(|| gettext("Failed to reset font")),
            },
        );
    }

    async fn update_cmd_with_view(
        &mut self,
        _widgets: &mut Self::Widgets,
        message: Self::CommandOutput,
        _sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            AppEvent::FontSet => show_font(self, root),
            AppEvent::ThemeAdded(theme) => {
                if !self.custom_themes.contains_key(&theme.name) {
                    add_row(self, root, theme.name);
                }
            }
            AppEvent::ThemeRemoved(name) => {
                if let Some(row) = self.custom_themes.remove(&name) {
                    root.list().remove(row.widget());
                }
            }
            _ => {}
        }
    }
}

fn add_row(model: &mut Model, root: &ui::ThemeList, name: ThemeName) {
    let row = theme_row::Model::builder()
        .launch((model.window.clone(), ThemeKey::Custom(name.clone())))
        .detach();
    row.widget()
        .enabled()
        .set_group(Some(&root.enabled_dummy()));
    root.list()
        .insert(row.widget(), root.import_button().index());
    model.custom_themes.insert(name, row);
}

fn show_font(model: &Model, root: &ui::ThemeList) {
    let profile = model.engine.profiles().current.clone();
    if let Some(family) = &profile.config.font_family {
        let subtitle = format!(r#"<span face="{family}">{family}</span>"#);
        root.font_row().set_subtitle(&subtitle);
        root.font_reset().set_visible(true);
    } else {
        root.font_row().set_subtitle("");
        root.font_reset().set_visible(false);
    }
}

async fn set_font(model: &Model) -> Result<()> {
    let Ok(font) = gtk::FontDialog::new()
        .choose_face_future(Some(&model.window), None::<&pango::FontFace>)
        .await
    else {
        return Ok(());
    };

    let profiles = model.engine.profiles();
    model
        .engine
        .set_profile_config(
            profiles.current_id,
            &ProfileConfig {
                font_family: Some(font.family().name().to_string()),
                ..profiles.current.config.clone()
            },
        )
        .await
        .context("failed to set font")?;
    _ = APP_EVENTS.send(AppEvent::FontSet);
    Ok(())
}

async fn reset_font(model: &Model) -> Result<()> {
    let profiles = model.engine.profiles();
    model
        .engine
        .set_profile_config(
            profiles.current_id,
            &ProfileConfig {
                font_family: None,
                ..profiles.current.config.clone()
            },
        )
        .await
        .context("failed to reset font")?;
    _ = APP_EVENTS.send(AppEvent::FontSet);
    Ok(())
}
