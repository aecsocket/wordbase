mod ui;

use {
    super::theme_row,
    crate::{APP_EVENTS, AppEvent, forward_events, gettext, toast_result},
    anyhow::{Context, Result},
    gtk4::prelude::{CheckButtonExt, ListBoxRowExt},
    relm4::{
        adw::{glib::clone, gtk::pango, prelude::*},
        prelude::*,
    },
    wordbase_engine::{Engine, profile::ProfileConfig},
};

#[derive(Debug)]
pub struct Model {
    default_theme: Controller<theme_row::Model>,
    custom_themes: Vec<Controller<theme_row::Model>>,
    engine: Engine,
    window: adw::Window,
    toaster: adw::ToastOverlay,
}

#[derive(Debug)]
#[doc(hidden)]
pub enum Msg {
    AskSetFont,
    ResetFont,
}

impl AsyncComponent for Model {
    type Init = (Engine, adw::Window, adw::ToastOverlay);
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
            engine,
            window,
            toaster,
        };
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
            _ => {}
        }
    }
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
