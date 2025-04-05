mod ui;

use {
    crate::{
        ACTION_PROFILE, gettext,
        platform::Platform,
        record::view::{RecordView, RecordViewMsg, RecordViewResponse},
    },
    anyhow::Result,
    relm4::{
        adw::{self, gdk, prelude::*},
        component::AsyncConnector,
        loading_widgets::LoadingWidgets,
        prelude::*,
        view,
    },
    std::sync::Arc,
    tracing::warn,
    wordbase::{PopupRequest, WindowFilter},
    wordbase_engine::Engine,
};

pub async fn connector(
    app: &adw::Application,
    platform: &Arc<dyn Platform>,
    engine: Engine,
) -> Result<AsyncConnector<Popup>> {
    let connector = Popup::builder().launch(PopupConfig {
        platform: platform.clone(),
        engine,
    });
    let window = connector.widget();
    app.add_window(window);
    platform.init_popup(window.upcast_ref()).await?;
    window.set_visible(false);
    Ok(connector)
}

#[derive(Debug)]
pub struct Popup {
    platform: Arc<dyn Platform>,
    engine: Engine,
    record_view: AsyncController<RecordView>,
    next_move_op: Option<(WindowFilter, (i32, i32))>,
}

#[derive(Debug)]
pub struct PopupConfig {
    platform: Arc<dyn Platform>,
    engine: Engine,
}

#[derive(Debug)]
pub enum PopupMsg {
    Request(PopupRequest),
    #[doc(hidden)]
    View(RecordViewResponse),
}

#[derive(Debug)]
pub enum PopupResponse {
    Hidden,
    OpenSettings,
    View(RecordViewResponse),
}

impl AsyncComponent for Popup {
    type Init = PopupConfig;
    type Input = PopupMsg;
    type Output = PopupResponse;
    type CommandOutput = ();
    type Root = ui::Popup;
    type Widgets = ui::Popup;

    fn init_root() -> Self::Root {
        ui::Popup::new()
    }

    fn init_loading_widgets(root: Self::Root) -> Option<LoadingWidgets> {
        let content = root.content();
        view! {
            #[local]
            content {
                #[name(spinner)]
                adw::Spinner {}
            }
        }
        Some(LoadingWidgets::new(content, spinner))
    }

    async fn init(
        init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let model = Self {
            platform: init.platform,
            engine: init.engine.clone(),
            record_view: RecordView::builder()
                .launch(init.engine)
                .forward(sender.input_sender(), |resp| PopupMsg::View(resp)),
            next_move_op: None,
        };
        root.connect_visible_notify({
            let sender = sender.clone();
            move |root| {
                if !root.is_visible() {
                    sender.output(PopupResponse::Hidden);
                }
            }
        });
        root.content().set_child(Some(model.record_view.widget()));
        root.profiles_button().connect_clicked(move |_| {
            _ = sender.output(PopupResponse::OpenSettings);
        });
        hide_on_lost_focus(root.upcast_ref());
        AsyncComponentParts {
            model,
            widgets: root,
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, _sender: AsyncComponentSender<Self>) {
        widgets.profiles_menu().remove_all();
        let profiles = self.engine.profiles.load();
        for (profile_id, profile) in profiles.by_id.iter() {
            let label = profile
                .meta
                .name
                .as_deref()
                .unwrap_or_else(|| gettext("Default Profile"));
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
            PopupMsg::Request(request) => {
                // TODO compute real origin
                let origin = request.origin;
                self.next_move_op = Some((request.target_window, origin));

                _ = self
                    .record_view
                    .sender()
                    .send(RecordViewMsg::Lookup(request.lookup));
            }
            PopupMsg::View(resp) => {
                if let Some((target_window, origin)) = self.next_move_op.take() {
                    if !resp.records.is_empty() {
                        root.present();
                        if let Err(err) = self
                            .platform
                            .move_popup_to_window(root.upcast_ref(), target_window, origin)
                            .await
                        {
                            warn!("Failed to move popup to target window: {err:?}");
                        }
                    }
                }

                sender.output(PopupResponse::View(resp));
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

    let root = root.clone();
    toplevel.connect_state_notify(move |toplevel| {
        if !toplevel.state().contains(gdk::ToplevelState::FOCUSED) {
            root.set_visible(false);
        }
    });
}
