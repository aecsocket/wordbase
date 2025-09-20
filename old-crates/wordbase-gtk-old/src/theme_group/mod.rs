mod ui;

use {
    crate::{
        AppEvent, CURRENT_PROFILE_ID, forward_events, gettext,
        theme::{CUSTOM_THEMES, ThemeKey, ThemeName},
        toast_result,
        util::{AppComponent, impl_component},
    },
    anyhow::{Context, Result},
    foldhash::{HashMap, HashMapExt},
    gtk4::prelude::{CheckButtonExt, ListBoxRowExt},
    relm4::{
        adw::{glib::clone, gtk::pango, prelude::*},
        prelude::*,
    },
    wordbase::NormString,
};

#[derive(Debug)]
pub struct ThemeGroup {
    _default_theme: AsyncController<theme_row::Model>,
    custom_themes: HashMap<ThemeName, AsyncController<theme_row::Model>>,
}

#[derive(Debug)]
#[doc(hidden)]
pub enum Msg {
    AskSetFont,
    ResetFont,
}

impl_component!(ThemeGroup);

impl AppComponent for ThemeGroup {
    type Args = ();
    type Msg = Msg;
    type Root = ui::ThemeGroup;

    async fn init(
        (): Self::Args,
        ui: Self::Ui,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        ui.font_row().connect_activated(clone!(
            #[strong]
            sender,
            move |_| sender.input(Msg::AskSetFont)
        ));
        ui.font_reset().connect_clicked(clone!(
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
            .set_group(Some(&ui.enabled_dummy()));
        ui.list()
            .insert(default_theme.widget(), ui.import_button().index());

        let mut model = Self {
            _default_theme: default_theme,
            custom_themes: HashMap::new(),
            engine,
            window,
            toaster,
        };
        for name in CUSTOM_THEMES.read().keys() {
            add_row(&mut model, &ui, name.clone());
        }
        show_font(&model, &ui);
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

fn add_row(model: &mut ThemeGroup, root: &ui::ThemeGroup, name: ThemeName) {
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

fn show_font(model: &ThemeGroup, root: &ui::ThemeGroup) {
    let profile = CURRENT_PROFILE.read().as_ref().cloned().unwrap();
    if let Some(family) = &profile.config.font_family {
        let subtitle = format!(r#"<span face="{family}">{family}</span>"#);
        root.font_row().set_subtitle(&subtitle);
        root.font_reset().set_visible(true);
    } else {
        root.font_row().set_subtitle("");
        root.font_reset().set_visible(false);
    }
}

async fn set_font(model: &ThemeGroup) -> Result<()> {
    let Ok(font) = gtk::FontDialog::new()
        .choose_face_future(Some(&model.window), None::<&pango::FontFace>)
        .await
    else {
        return Ok(());
    };

    let mut config = CURRENT_PROFILE
        .read()
        .as_ref()
        .cloned()
        .unwrap()
        .config
        .clone();
    config.font_family = NormString::new(font.family().name());
    model
        .engine
        .set_profile_config(CURRENT_PROFILE_ID.read().unwrap(), config)
        .await
        .context("failed to set font")?;
    _ = APP_EVENTS.send(AppEvent::FontSet);
    Ok(())
}

async fn reset_font(model: &ThemeGroup) -> Result<()> {
    let mut config = CURRENT_PROFILE
        .read()
        .as_ref()
        .cloned()
        .unwrap()
        .config
        .clone();
    config.font_family = None;
    model
        .engine
        .set_profile_config(CURRENT_PROFILE_ID.read().unwrap(), config)
        .await
        .context("failed to reset font")?;
    _ = APP_EVENTS.send(AppEvent::FontSet);
    Ok(())
}
