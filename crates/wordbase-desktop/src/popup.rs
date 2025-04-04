use {
    crate::{
        platform::Platform,
        record::{
            render::Records,
            view::{RecordView, RecordViewConfig, RecordViewMsg},
        },
    },
    anyhow::Result,
    relm4::{
        adw::{self, prelude::*},
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
) -> Result<AsyncConnector<Popup>> {
    let connector = Popup::builder().launch(PopupConfig {
        platform: platform.clone(),
        record_view,
    });
    let window = connector.widget();
    app.add_window(window);
    platform.init_popup(window).await?;
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
}

#[derive(Debug, Clone)]
pub struct AppPopupRequest {
    pub target_window: WindowFilter,
    pub origin: (i32, i32),
    pub anchor: PopupAnchor,
    pub records: Arc<Records>,
}

#[relm4::component(pub, async)]
impl AsyncComponent for Popup {
    type Init = PopupConfig;
    type Input = AppPopupRequest;
    type Output = ();
    type CommandOutput = ();

    fn init_loading_widgets(root: Self::Root) -> Option<LoadingWidgets> {
        view! {
            #[local]
            root {
                set_title: Some("Wordbase Popup"),
                set_width_request: 180,
                set_height_request: 100,
                set_hide_on_close: true,

                #[name(spinner)]
                adw::Spinner {}
            }
        }
        Some(LoadingWidgets::new(root, spinner))
    }

    view! {
        adw::Window {
            model.record_view.widget(),
        }
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
        let widgets = view_output!();
        hide_on_lost_focus(&root);
        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        request: Self::Input,
        _sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        root.set_visible(true);
        _ = self
            .record_view
            .sender()
            .send(RecordViewMsg::Records(request.records));

        // TODO compute it
        let origin = request.origin;

        if let Err(err) = self
            .platform
            .move_popup_to_window(root, request.target_window, origin)
            .await
        {
            warn!("Failed to move popup to target window: {err:?}");
        }
    }
}

fn hide_on_lost_focus(root: &adw::Window) {
    let root = root.clone();

    // todo
    root.connect_is_active_notify(move |root| {
        if !root.is_active() {
            root.set_visible(false);
        }
    });
}
