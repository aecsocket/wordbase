use std::sync::Arc;

use anyhow::Result;
use futures::never::Never;
use relm4::{
    adw::{self, prelude::*},
    prelude::*,
};
use tokio::sync::mpsc;

use crate::platform::Platform;

pub async fn run(
    app: adw::Application,
    platform: Arc<dyn Platform>,
    recv_popup_request: mpsc::Receiver<()>,
) -> Result<Never> {
    loop {}
}

pub struct PopupRequest {}

#[derive(Debug)]
struct Popup {}

#[derive(Debug)]
enum PopupMsg {
    Lookup { query: String },
}

#[relm4::component(pub)]
impl SimpleComponent for Popup {
    type Init = ();
    type Input = PopupMsg;
    type Output = ();

    view! {
        adw::Window {
            set_title: Some("Wordbase"),

            webkit6::WebView {}
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self {};
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }
}
