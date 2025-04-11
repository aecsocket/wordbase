mod ui;

use {
    crate::{
        ACTION_PROFILE, APP_BROKER, APP_ID, AppEvent, AppMsg, forward_events, gettext,
        platform::Platform, record_view,
    },
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
) -> Result<AsyncConnector<Model>> {
    let connector = Model::builder().launch((platform.clone(), engine));
    let window = connector.widget();
    platform.init_popup(window.upcast_ref()).await?;
    Ok(connector)
}

#[derive(Debug)]
pub struct Model {
    record_view: AsyncController<record_view::Model>,
    platform: Arc<dyn Platform>,
    engine: Engine,
    query_override: Option<String>,
}

#[derive(Debug)]
pub enum Msg {
    Render {
        records: Vec<RecordLookup>,
    },
    Present {
        target_window: WindowFilter,
        origin: (i32, i32),
        anchor: PopupAnchor,
    },
    #[doc(hidden)]
    Query(String),
    #[doc(hidden)]
    Requery,
}

impl AsyncComponent for Model {
    type Init = (Arc<dyn Platform>, Engine);
    type Input = Msg;
    type Output = ();
    type CommandOutput = AppEvent;
    type Root = ui::Popup;
    type Widgets = ui::Popup;

    fn init_root() -> Self::Root {
        ui::Popup::new()
    }

    async fn init(
        (platform, engine): Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        forward_events(&sender);
        relm4::main_application().add_window(&root);

        let settings = gio::Settings::new(APP_ID);
        settings.bind("popup-width", &root, "default-width").build();
        settings
            .bind("popup-height", &root, "default-height")
            .build();

        let model = Self {
            platform,
            record_view: record_view::Model::builder()
                .launch(engine.clone())
                .forward(sender.input_sender(), |resp| match resp {
                    record_view::Response::Query(query) => Msg::Query(query),
                }),
            engine,
            query_override: None,
        };
        root.content().set_child(Some(model.record_view.widget()));
        root.manager_profiles()
            .connect_clicked(move |_| APP_BROKER.send(AppMsg::Present));
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
            Msg::Render { records } => {
                self.query_override = None;
                self.record_view.sender().emit(record_view::Msg(records));
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
            Msg::Query(query) => {
                self.query_override = Some(query);
                sender.input(Msg::Requery);
            }
            Msg::Requery => {
                let Some(query) = &self.query_override else {
                    return;
                };
                let Ok(records) = self
                    .engine
                    .lookup(query, 0, record_view::SUPPORTED_RECORD_KINDS)
                    .await
                else {
                    return;
                };
                sender.input(Msg::Render { records });
            }
        }
    }

    async fn update_cmd_with_view(
        &mut self,
        _widgets: &mut Self::Widgets,
        event: Self::CommandOutput,
        sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        if !record_view::should_requery(&event) {
            return;
        }
        sender.input(Msg::Requery);
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
