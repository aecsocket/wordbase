use {
    crate::{APP_ID, platform::Platform},
    anyhow::{Context, Result},
    foldhash::{HashMap, HashMapExt},
    futures::never::Never,
    relm4::{
        adw::{
            self,
            gtk::{graphene, pango},
            prelude::*,
        },
        css::classes,
        prelude::*,
    },
    std::{
        collections::hash_map::Entry,
        sync::{
            Arc,
            atomic::{self, AtomicI32},
        },
    },
    tokio::sync::mpsc,
    tracing::{info, trace, warn},
    wordbase::{Lookup, PopupAnchor, PopupRequest, TexthookerSentence, WindowFilter},
};

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

            let overlay = Overlay::builder().launch(OverlayConfig {
                process_path,
                sentence,
            });
            let window = overlay.widget().clone();
            app.add_window(&window);
            let overlay = overlay.connect_receiver({
                let window = window.clone();
                let send_popup_request = send_popup_request.clone();
                move |_sender, scan| {
                    glib::spawn_future_local(on_scan(
                        window.clone(),
                        send_popup_request.clone(),
                        scan,
                    ));
                }
            });

            platform
                .init_overlay(&window)
                .await
                .context("failed to initialize overlay window")?;

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
    origin: (i32, i32),
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

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
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
    let last_scan_byte_index = Arc::new(AtomicI32::new(-1));
    controller.connect_leave({
        let last_scan_char_idx = last_scan_byte_index.clone();
        move |_| {
            last_scan_char_idx.store(-1, atomic::Ordering::Relaxed);
        }
    });
    controller.connect_motion(move |_, rel_x, rel_y| {
        #[expect(clippy::cast_possible_truncation, reason = "no other way to convert")]
        let (ch_x, ch_y) = (rel_x as i32 * pango::SCALE, rel_y as i32 * pango::SCALE);

        let (valid, byte_index_i32, _grapheme_pos) = label.layout().xy_to_index(ch_x, ch_y);
        if !valid {
            return;
        }
        if byte_index_i32 == last_scan_byte_index.swap(byte_index_i32, atomic::Ordering::SeqCst) {
            // we're scanning the same character as in the last motion event
            return;
        }
        let Ok(byte_index) = usize::try_from(byte_index_i32) else {
            return;
        };

        let text = &label.text();
        let lookup = Lookup {
            // TODO: add some scrollback to context
            context: text.to_string(),
            cursor: byte_index,
        };

        let ch_rect = label.layout().index_to_pos(byte_index_i32);
        let (ch_rel_x, ch_rel_y) = (
            // anchor to bottom-right of character
            (ch_rect.x() + ch_rect.width()) as f32 / pango::SCALE as f32,
            (ch_rect.y() + ch_rect.height()) as f32 / pango::SCALE as f32,
        );

        let abs_point = label
            .compute_point(&root, &graphene::Point::new(ch_rel_x, ch_rel_y))
            .expect("should be able to compute point transform from `label` space to `root` space");

        let scan = OverlayScan {
            lookup,
            origin: (abs_point.x() as i32 + 16, abs_point.y() as i32 + 16),
        };
        _ = sender.output_sender().send(scan);
    });
}

async fn on_scan(
    window: adw::Window,
    send_popup_request: mpsc::Sender<PopupRequest>,
    scan: OverlayScan,
) {
    _ = send_popup_request
        .send(PopupRequest {
            target_window: WindowFilter {
                id: None,
                title: window.title().map(|s| s.to_string()),
                wm_class: Some(APP_ID.to_owned()),
            },
            origin: scan.origin,
            anchor: PopupAnchor::BottomCenter,
            lookup: scan.lookup,
        })
        .await;
}
