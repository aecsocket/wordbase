mod ui;

use anyhow::{Context, Result};
use gtk4::prelude::{CheckButtonExt, ListBoxRowExt};
use relm4::{
    adw::{glib::clone, gtk::pango, prelude::*},
    prelude::*,
};
use wordbase_engine::{Engine, profile::ProfileConfig};

use crate::{APP_EVENTS, AppEvent, forward_events, gettext};

use super::theme_row;

#[derive(Debug)]
pub struct Model {
    default_theme: Controller<theme_row::Model>,
    custom_themes: Vec<Controller<theme_row::Model>>,
    window: adw::Window,
    engine: Engine,
}

#[derive(Debug)]
#[doc(hidden)]
pub enum Msg {
    SelectFont,
    ResetFont,
}

#[derive(Debug)]
pub enum Response {
    Error(anyhow::Error),
}

impl AsyncComponent for Model {
    type Init = (adw::Window, Engine);
    type Input = Msg;
    type Output = Response;
    type CommandOutput = AppEvent;
    type Root = ui::Themes;
    type Widgets = ();

    fn init_root() -> Self::Root {
        ui::Themes::new()
    }

    async fn init(
        (window, engine): Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        forward_events(&sender);

        root.font_row().connect_activated(clone!(
            #[strong]
            sender,
            move |_| sender.input(Msg::SelectFont)
        ));
        root.font_reset().connect_clicked(clone!(
            #[strong]
            sender,
            move |_| sender.input(Msg::ResetFont)
        ));

        let default_theme = theme_row::Model::builder()
            .launch((window.clone(), None))
            .detach();
        default_theme
            .widget()
            .enabled()
            .set_group(Some(&root.enabled_dummy()));
        root.list()
            .insert(default_theme.widget(), root.import_button().index());

        let model = Self {
            default_theme,
            custom_themes: Vec::new(),
            window,
            engine,
        };
        set_font(&model, &root);
        AsyncComponentParts { model, widgets: () }
    }

    async fn update_with_view(
        &mut self,
        _widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        let result = match message {
            Msg::SelectFont => select_font(self)
                .await
                .with_context(|| gettext("Failed to set font")),
            Msg::ResetFont => reset_font(self)
                .await
                .with_context(|| gettext("Failed to reset font")),
        };
        if let Err(err) = result {
            _ = sender.output(Response::Error(err));
        }
    }

    async fn update_cmd_with_view(
        &mut self,
        _widgets: &mut Self::Widgets,
        message: Self::CommandOutput,
        _sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            AppEvent::FontSet => set_font(self, root),
        }
    }
}

fn set_font(model: &Model, root: &ui::Themes) {
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

async fn select_font(model: &Model) -> Result<()> {
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
