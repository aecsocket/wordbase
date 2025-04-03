use std::{collections::hash_map::Entry, sync::Arc};

use anyhow::{Context, Result};
use foldhash::{HashMap, HashMapExt};
use futures::never::Never;
use relm4::{
    adw::{self, prelude::*},
    css::classes,
    prelude::*,
};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};
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
                debug!("New sentence for {process_path:?}: {sentence:?}");

                _ = entry
                    .get()
                    .sender()
                    .send(OverlayMsg::NewSentence { sentence });
            }
            Entry::Vacant(entry) => {
                info!("Creating overlay for new process {process_path:?}");

                let overlay = Overlay::builder()
                    .launch(OverlayConfig {
                        process_path,
                        sentence,
                    })
                    .detach();
                let window = overlay.widget();
                app.add_window(window);
                window.present();

                if let Err(err) = platform.affix_to_focused_window(window).await {
                    warn!("Failed to affix overlay window to currently focused window: {err:?}");
                }

                entry.insert(overlay);
            }
        };
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
            set_width_request: 180,
            set_height_request: 100,

            gtk::ScrolledWindow {
                set_hexpand: true,
                set_vexpand: true,

                gtk::Label {
                    set_margin_start: 16,
                    set_margin_end: 16,
                    set_margin_top: 16,
                    set_margin_bottom: 16,
                    set_halign: gtk::Align::Start,
                    set_valign: gtk::Align::Start,
                    set_hexpand: true,
                    set_vexpand: true,
                    set_xalign: 0.0,
                    set_yalign: 0.0,
                    set_wrap: true,
                    set_selectable: true,
                    add_css_class: classes::BODY,

                    #[watch]
                    set_text: &model.sentence,
                },
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

        let opacity_target = adw::PropertyAnimationTarget::new(&root, "opacity");
        let animation = adw::TimedAnimation::builder()
            .widget(&root)
            .duration(100)
            .target(&opacity_target)
            .value_from(0.5)
            .value_to(0.95)
            .build();

        let controller = gtk::EventControllerMotion::new();
        controller.connect_enter({
            let animation = animation.clone();
            move |_, _, _| {
                animation.set_reverse(false);
                animation.play();
            }
        });
        controller.connect_leave({
            let animation = animation.clone();
            move |_| {
                animation.set_reverse(true);
                animation.play();
            }
        });
        animation.set_reverse(true);
        animation.play();
        root.add_controller(controller);

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
