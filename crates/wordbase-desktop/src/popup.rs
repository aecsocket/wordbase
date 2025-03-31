use relm4::{
    adw::{self, prelude::*},
    prelude::*,
};
use wordbase::RecordLookup;

pub struct Popup {}

#[derive(Debug)]
pub enum PopupMsg {
    NewRecords(Vec<RecordLookup>),
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

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            PopupMsg::NewRecords(records) => {
                self.
            }
        }
    }
}
