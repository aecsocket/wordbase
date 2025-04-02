use std::{collections::hash_map::Entry, sync::Arc};

use anyhow::{Context, Result};
use foldhash::{HashMap, HashMapExt};
use futures::never::Never;
use relm4::{
    adw::{self, prelude::*},
    prelude::*,
};
use tokio::sync::mpsc;
use tracing::warn;
use wordbase::TexthookerSentence;

use crate::platform::Platform;

pub async fn run(
    app: adw::Application,
    platform: Arc<dyn Platform>,
    mut recv_sentence: mpsc::Receiver<TexthookerSentence>,
) -> Result<Never> {
    let mut overlays = HashMap::<String, Controller<Overlay>>::new();

    loop {
        let TexthookerSentence {
            process_path,
            sentence,
        } = recv_sentence
            .recv()
            .await
            .context("sentence channel closed")?;

        match overlays.entry(process_path.clone()) {
            Entry::Occupied(entry) => {
                _ = entry
                    .get()
                    .sender()
                    .send(OverlayMsg::NewSentence { sentence });
            }
            Entry::Vacant(entry) => {
                let overlay = Overlay::builder()
                    .launch(OverlayConfig {
                        process_path,
                        sentence,
                    })
                    .detach();
                let window = overlay.widget();
                app.add_window(window);

                if let Err(err) = platform.affix_to_focused_window(window).await {
                    warn!("Failed to affix overlay window to currently focused window: {err:?}");
                }

                entry.insert(overlay);
            }
        }
    }
}

#[derive(Debug)]
struct Overlay {
    sentence: String,
}

#[derive(Debug)]
struct OverlayConfig {
    process_path: String,
    sentence: String,
}

#[derive(Debug)]
enum OverlayMsg {
    NewSentence { sentence: String },
}

#[relm4::component(pub)]
impl Component for Overlay {
    type Init = OverlayConfig;
    type Input = OverlayMsg;
    type Output = ();
    type CommandOutput = ();

    view! {
        adw::Window {
            set_title: Some(&format!("{} — Wordbase", init.process_path)),

            gtk::Label {
                #[watch]
                set_text: &model.sentence,
            },
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self {
            sentence: init.sentence,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match message {
            OverlayMsg::NewSentence { sentence } => {
                self.sentence = sentence;
            }
        }
    }
}
