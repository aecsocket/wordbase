use {
    crate::{
        Dictionaries,
        platform::Platform,
        record::view::{RecordView, RecordViewConfig, RecordViewMsg},
    },
    anyhow::{Context, Result},
    futures::never::Never,
    relm4::{
        adw::{self, prelude::*},
        loading_widgets::LoadingWidgets,
        prelude::*,
        view,
    },
    std::sync::Arc,
    tokio::sync::mpsc,
    tracing::warn,
    wordbase::PopupRequest,
    wordbase_engine::Engine,
};

pub async fn run(
    engine: Engine,
    platform: Arc<dyn Platform>,
    dictionaries: Dictionaries,
    app: adw::Application,
    mut recv_popup_request: mpsc::Receiver<PopupRequest>,
) -> Result<Never> {
    let popup = Popup::builder().launch(PopupConfig {
        platform: platform.clone(),
        record_view: RecordViewConfig {
            engine,
            dictionaries,
        },
    });
    let window = popup.widget().clone();
    app.add_window(&window);
    let popup = popup.detach();

    platform
        .init_popup(&window)
        .await
        .context("failed to initialize popup window")?;
    window.set_visible(false);

    loop {
        let request = recv_popup_request
            .recv()
            .await
            .context("popup request channel closed")?;
        _ = popup.sender().send(request);
    }
}

struct Popup {
    platform: Arc<dyn Platform>,
    record_view: AsyncController<RecordView>,
}

struct PopupConfig {
    platform: Arc<dyn Platform>,
    record_view: RecordViewConfig,
}

#[relm4::component(pub, async)]
impl AsyncComponent for Popup {
    type Init = PopupConfig;
    type Input = PopupRequest;
    type Output = ();
    type CommandOutput = ();

    fn init_loading_widgets(root: Self::Root) -> Option<LoadingWidgets> {
        view! {
            #[local]
            root {
                set_title: Some("Wordbase Popup"),
                set_default_width: 180,
                set_default_height: 100,
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
        hide_on_leave(&root);
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
            .send(RecordViewMsg::Lookup(request.lookup));

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

fn hide_on_leave(root: &adw::Window) {
    let controller = gtk::EventControllerMotion::new();
    root.add_controller(controller.clone());
    let root = root.clone();
    controller.connect_leave(move |_| {
        root.set_visible(false);
    });
}
