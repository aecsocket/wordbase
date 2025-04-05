mod ui;

use {
    crate::{
        AppMsg,
        platform::Platform,
        record::{
            render::Records,
            view::{RecordView, RecordViewConfig, RecordViewMsg},
        },
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
    wordbase::{PopupAnchor, WindowFilter},
};

pub async fn connector(
    app: &adw::Application,
    platform: &Arc<dyn Platform>,
    record_view: RecordViewConfig,
    to_manager: relm4::Sender<AppMsg>,
) -> Result<AsyncConnector<Popup>> {
    let connector = Popup::builder().launch(PopupConfig {
        platform: platform.clone(),
        record_view,
        to_manager,
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
    record_view: AsyncController<RecordView>,
}

#[derive(Debug)]
pub struct PopupConfig {
    platform: Arc<dyn Platform>,
    record_view: RecordViewConfig,
    to_manager: relm4::Sender<AppMsg>,
}

#[derive(Debug, Clone)]
pub struct AppPopupRequest {
    pub target_window: WindowFilter,
    pub origin: (i32, i32),
    pub anchor: PopupAnchor,
    pub records: Arc<Records>,
}

impl AsyncComponent for Popup {
    type Init = PopupConfig;
    type Input = AppPopupRequest;
    type Output = ();
    type CommandOutput = ();
    type Root = ui::Popup;
    type Widgets = ();

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
        _sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let model = Self {
            platform: init.platform,
            record_view: RecordView::builder().launch(init.record_view).detach(),
        };
        root.content().set_child(Some(model.record_view.widget()));
        root.settings().connect_clicked(move |_| {
            _ = init.to_manager.send(AppMsg::Show);
        });

        hide_on_lost_focus(root.upcast_ref());
        AsyncComponentParts { model, widgets: () }
    }

    async fn update(
        &mut self,
        request: Self::Input,
        _sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        _ = self
            .record_view
            .sender()
            .send(RecordViewMsg::Records(request.records));

        // TODO compute it
        let target_window = request.target_window;
        let origin = request.origin;

        let root = root.clone();
        let platform = self.platform.clone();
        root.set_visible(true);
        if let Err(err) = platform
            .move_popup_to_window(root.upcast_ref(), target_window, origin)
            .await
        {
            warn!("Failed to move popup to target window: {err:?}");
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
