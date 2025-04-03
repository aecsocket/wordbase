use std::{collections::hash_map::Entry, process, sync::Arc};

use anyhow::{Context, Result};
use foldhash::{HashMap, HashMapExt};
use futures::never::Never;
use relm4::{
    adw::{
        self,
        gtk::{graphene, pango},
        prelude::*,
    },
    css::classes,
    prelude::*,
};
use tokio::sync::mpsc;
use tracing::{info, trace, warn};
use wordbase::{Lookup, PopupAnchor, PopupRequest, TexthookerSentence, WindowFilter};

use crate::platform::Platform;

pub async fn run(
    app: adw::Application,
    platform: Arc<dyn Platform>,
    mut recv_sentence: mpsc::Receiver<TexthookerSentence>,
    send_popup_request: mpsc::Sender<PopupRequest>,
) -> Result<Never> {
    let mut overlays = HashMap::<String, Controller<Overlay>>::new();

    loop {
        let event = recv_sentence
            .recv()
            .await
            .context("sentence channel closed")?;
        trace!(
            "New sentence for {:?}: {:?}",
            event.process_path, event.sentence
        );

        if let Err(err) = handle(&app, &*platform, &send_popup_request, &mut overlays, event).await
        {
            warn!("Failed to handle new sentence event: {err:?}");
        }
    }
}

async fn handle(
    app: &adw::Application,
    platform: &dyn Platform,
    send_popup_request: &mpsc::Sender<PopupRequest>,
    overlays: &mut HashMap<String, Controller<Overlay>>,
    TexthookerSentence {
        process_path,
        sentence,
    }: TexthookerSentence,
) -> Result<()> {
    match overlays.entry(process_path.clone()) {
        Entry::Occupied(entry) => {
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
                .connect_receiver(|_sender, scan| {
                    // tokio.spawn(on_scan(, send_popup_request, scan))
                });
            let window = overlay.widget();
            app.add_window(window);

            platform
                .init_overlay(window)
                .await
                .context("failed to initialize window as overlay")?;

            entry.insert(overlay);
        }
    }
    Ok(())
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

#[derive(Debug)]
struct OverlayScan {
    lookup: Lookup,
    origin: (f32, f32),
}

#[relm4::component(pub)]
impl Component for Overlay {
    type Init = OverlayConfig;
    type Input = OverlayMsg;
    type Output = OverlayScan;
    type CommandOutput = ();

    view! {
        adw::Window {
            set_title: Some(&format!("{} — Wordbase", init.process_path)),
            set_width_request: 180,
            set_height_request: 100,

            gtk::ScrolledWindow {
                set_hexpand: true,
                set_vexpand: true,

                #[name(sentence_label)]
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
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self {
            sentence: init.sentence,
        };
        let widgets = view_output!();
        setup_root_opacity_animation(&root);
        setup_sentence_scan(&root, &widgets.sentence_label, &sender);
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

fn setup_root_opacity_animation(root: &adw::Window) {
    let opacity_target = adw::PropertyAnimationTarget::new(root, "opacity");
    let animation = adw::TimedAnimation::builder()
        .widget(root)
        .duration(100)
        .target(&opacity_target)
        .value_from(0.5)
        .value_to(0.95)
        .build();

    let controller = gtk::EventControllerMotion::new();
    root.add_controller(controller.clone());

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
}

fn setup_sentence_scan(root: &adw::Window, label: &gtk::Label, sender: &ComponentSender<Overlay>) {
    let controller = gtk::EventControllerMotion::new();
    label.add_controller(controller.clone());

    let root = root.clone();
    let label = label.clone();
    let sender = sender.clone();
    controller.connect_motion(move |_, x, y| {
        #[expect(clippy::cast_possible_truncation, reason = "no other way to convert")]
        let (x, y) = (x as i32 * pango::SCALE, y as i32 * pango::SCALE);

        let (valid, byte_index, _grapheme_pos) = label.layout().xy_to_index(x, y);
        if !valid {
            return;
        }
        let Ok(byte_index) = usize::try_from(byte_index) else {
            return;
        };

        let text = &label.text();
        let lookup = Lookup {
            // TODO: add some scrollback to context
            context: text.to_string(),
            cursor: byte_index,
        };

        let abs_point = label
            .compute_point(&root, &graphene::Point::new(x as f32, y as f32))
            .expect("should be able to compute point transform from `label` space to `root` space");

        _ = sender.output_sender().send(OverlayScan {
            lookup,
            origin: (abs_point.x(), abs_point.y()),
        });
    });
}

async fn on_scan(
    root: adw::Window,
    send_popup_request: mpsc::Sender<PopupRequest>,
    scan: OverlayScan,
) {
    send_popup_request
        .send(PopupRequest {
            target_window: WindowFilter {
                id: None,
                pid: Some(process::id()),
                title: root.title().map(|s| s.to_string()),
                wm_class: None, // todo
            },
            origin: (scan.origin.0 as i32, scan.origin.1 as i32),
            anchor: PopupAnchor::BottomCenter,
            lookup: scan.lookup,
        })
        .await;
}
