mod ui;

use {
    crate::{ACTION_PROFILE, APP_ID, gettext, platform::Platform, record_view, theme::Theme},
    anyhow::Result,
    glib::clone,
    relm4::{
        adw::{gdk, gio, prelude::*},
        component::AsyncConnector,
        prelude::*,
    },
    std::sync::Arc,
    tracing::warn,
    wordbase::{PopupAnchor, RecordLookup, WindowFilter},
    wordbase_engine::Engine,
};

pub async fn connector(
    platform: &Arc<dyn Platform>,
    engine: Engine,
    custom_theme: Option<Arc<Theme>>,
) -> Result<AsyncConnector<Model>> {
    let connector = Model::builder().launch((platform.clone(), engine, custom_theme));
    let window = connector.widget();
    platform.init_popup(window.upcast_ref()).await?;
    Ok(connector)
}

#[derive(Debug)]
pub struct Model {
    record_view: Controller<record_view::Model>,
    platform: Arc<dyn Platform>,
    engine: Engine,
    query_override: Option<String>,
}

#[derive(Debug)]
pub enum Msg {
    CustomTheme(Option<Arc<Theme>>),
    Render {
        records: Vec<RecordLookup>,
    },
    Present {
        target_window: WindowFilter,
        origin: (i32, i32),
        anchor: PopupAnchor,
    },
    #[doc(hidden)]
    FromView(record_view::Response),
}

#[derive(Debug)]
pub enum Response {
    Hidden,
    OpenSettings,
}

impl AsyncComponent for Model {
    type Init = (Arc<dyn Platform>, Engine, Option<Arc<Theme>>);
    type Input = Msg;
    type Output = Response;
    type CommandOutput = ();
    type Root = ui::Popup;
    type Widgets = ui::Popup;

    fn init_root() -> Self::Root {
        ui::Popup::new()
    }

    async fn init(
        (platform, engine, custom_theme): Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        relm4::main_application().add_window(&root);

        let settings = gio::Settings::new(APP_ID);
        settings.bind("popup-width", &root, "default-width").build();
        settings
            .bind("popup-height", &root, "default-height")
            .build();

        let model = Self {
            platform,
            record_view: record_view::Model::builder()
                .launch(record_view::Config { custom_theme })
                .forward(sender.input_sender(), Msg::FromView),
            engine,
            query_override: None,
        };
        root.connect_visible_notify({
            let sender = sender.clone();
            move |root| {
                if !root.is_visible() {
                    _ = sender.output(Response::Hidden);
                }
            }
        });
        root.content().set_child(Some(model.record_view.widget()));
        root.manager_profiles().connect_clicked(move |_| {
            _ = sender.output(Response::OpenSettings);
        });
        root.present();
        hide_on_lost_focus(root.upcast_ref());
        root.set_visible(false);
        AsyncComponentParts {
            model,
            widgets: root,
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, _sender: AsyncComponentSender<Self>) {
        widgets.profiles_menu().remove_all();
        for (profile_id, profile) in &self.engine.profiles().by_id {
            let label = profile
                .meta
                .name
                .as_ref()
                .map_or_else(|| gettext("Default Profile"), |s| s.as_str());
            widgets.profiles_menu().append(
                Some(label),
                Some(&format!("app.{ACTION_PROFILE}::{}", profile_id.0)),
            );
        }
    }

    async fn update(
        &mut self,
        message: Self::Input,
        sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            Msg::CustomTheme(theme) => self
                .record_view
                .sender()
                .emit(record_view::Msg::CustomTheme(theme)),
            Msg::Render { records } => {
                self.query_override = None;
                self.record_view.sender().emit(record_view::Msg::Render {
                    dictionaries: self.engine.dictionaries(),
                    records,
                });
            }
            Msg::Present {
                target_window,
                origin,
                anchor,
            } => {
                root.present();
                if let Err(err) = self
                    .platform
                    .move_popup_to_window(root.upcast_ref(), target_window, origin)
                    .await
                {
                    warn!("Failed to present popup: {err:?}");
                }
            }
            Msg::FromView(record_view::Response::Query(query)) => {
                let records = self
                    .engine
                    .lookup(&query, 0, record_view::SUPPORTED_RECORD_KINDS)
                    .await;
                self.query_override = Some(query);
                let Ok(records) = records else { return };
                sender.input(Msg::Render { records });
            }
        }
    }
}

fn hide_on_lost_focus(root: &gtk::Window) {
    // This shit is so ass.
    // Under Wayland, when you start dragging a window (either move or resize),
    // it loses focus (`window.has_focus()`). There's no way to listen for
    // drag start/end events, so we don't know if we've lost focus because the
    // user actually clicked off, or because we're now dragging the window.
    // So to differentiate between the two, we check the *TopLevel's* focused
    // state instead, which stays true if the window is being dragged.

    let toplevel = root
        .surface()
        .expect("window does not have surface")
        .downcast::<gdk::Toplevel>()
        .expect("window surface is not a `gdk::Toplevel`");

    toplevel.connect_state_notify(clone!(
        #[strong]
        root,
        move |toplevel| {
            if !toplevel.state().contains(gdk::ToplevelState::FOCUSED) {
                root.set_visible(false);
            }
        }
    ));
}
