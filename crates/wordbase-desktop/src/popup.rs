use std::sync::Arc;

use anyhow::{Context, Result};
use futures::never::Never;
use relm4::{
    adw::{self, prelude::*},
    prelude::*,
};
use tokio::sync::mpsc;
use wordbase::{Lookup, PopupRequest};
use wordbase_engine::Engine;

use crate::{
    platform::Platform,
    record::view::{RecordView, RecordViewMsg},
};

pub async fn run(
    engine: Engine,
    app: adw::Application,
    _platform: Arc<dyn Platform>,
    mut recv_popup_request: mpsc::Receiver<PopupRequest>,
) -> Result<Never> {
    let popup = Popup::builder().launch(engine).detach();
    let window = popup.widget();
    window.set_hide_on_close(true);
    app.add_window(window);

    loop {
        let request = recv_popup_request
            .recv()
            .await
            .context("popup request channel closed")?;

        _ = popup.sender().send(request.lookup);
    }
}

#[derive(Debug)]
struct Popup {
    record_view: AsyncController<RecordView>,
}

#[relm4::component(pub)]
impl SimpleComponent for Popup {
    type Init = Engine;
    type Input = Lookup;
    type Output = ();

    view! {
        adw::Window {
            set_title: Some("TODO"),

            gtk::Label {
                set_text: "HELLO WORLD",
            }
        }
    }

    fn init(
        engine: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self {
            record_view: RecordView::builder().launch(engine).detach(),
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, lookup: Self::Input, _sender: ComponentSender<Self>) {
        _ = self
            .record_view
            .sender()
            .send(RecordViewMsg::Lookup(lookup));
    }
}
