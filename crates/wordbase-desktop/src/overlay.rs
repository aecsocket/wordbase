use {
    crate::{
        APP_ID,
        platform::{OverlayGuard, Platform},
        popup::PopupMsg,
        record::render::SUPPORTED_RECORD_KINDS,
    },
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
        loading_widgets::LoadingWidgets,
        prelude::*,
        view,
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
    wordbase::{PopupAnchor, TexthookerSentence, WindowFilter},
    wordbase_engine::Engine,
};

pub async fn run(
    app: adw::Application,
    platform: Arc<dyn Platform>,
    engine: Engine,
    mut recv_sentence: mpsc::Receiver<TexthookerSentence>,
    to_popup: relm4::Sender<PopupMsg>,
) -> Result<Never> {
    let mut overlays = HashMap::<String, OverlayState>::new();
    let (send_closed, mut recv_closed) = mpsc::unbounded_channel::<String>();

    loop {
        tokio::select! {
            event = recv_sentence.recv() => {
                let event = event.context("sentence channel closed")?;
                trace!(
                    "New sentence for {:?}: {:?}",
                    event.process_path, event.sentence
                );

                if let Err(err) = handle(
                    &app,
                    &*platform,
                    &engine,
                    &to_popup,
                    &mut overlays,
                    &send_closed,
                    event,
                )
                .await {
                    warn!("Failed to handle new sentence event: {err:?}");
                }
            }
            Some(process_path) = recv_closed.recv() => {
                info!("Overlay window {process_path:?} closed, removing");
                overlays.remove(&process_path);
            }
        }
    }
}

const POPUP_OFFSET: (i32, i32) = (0, 10);

struct OverlayState {
    _guard: OverlayGuard,
    controller: AsyncController<Overlay>,
}

async fn handle(
    app: &adw::Application,
    platform: &dyn Platform,
    engine: &Engine,
    to_popup: &relm4::Sender<PopupMsg>,
    overlays: &mut HashMap<String, OverlayState>,
    send_closed: &mpsc::UnboundedSender<String>,
    TexthookerSentence {
        process_path,
        sentence,
    }: TexthookerSentence,
) -> Result<()> {
    let OverlayState { controller, .. } = match overlays.entry(process_path.clone()) {
        Entry::Occupied(entry) => entry.into_mut(),
        Entry::Vacant(entry) => {
            let overlay = Overlay::builder().launch(OverlayConfig {
                engine: engine.clone(),
                to_popup: to_popup.clone(),
                process_path: process_path.clone(),
            });
            let window = overlay.widget();
            app.add_window(window);
            let guard = platform
                .init_overlay(window)
                .await
                .context("failed to create overlay window")?;

            window.connect_close_request({
                let send_closed = send_closed.clone();
                let process_path = process_path.clone();
                move |_| {
                    _ = send_closed.send(process_path.clone());
                    glib::Propagation::Proceed
                }
            });

            info!("Created overlay for new process {process_path:?}");
            entry.insert(OverlayState {
                controller: overlay.detach(),
                _guard: guard,
            })
        }
    };
    _ = controller.sender().send(OverlayMsg::Sentence { sentence });
    Ok(())
}

#[derive(Debug)]
struct Overlay {
    engine: Engine,
    to_popup: relm4::Sender<PopupMsg>,
    sentence: gtk::Label,
}

#[derive(Debug)]
struct OverlayConfig {
    engine: Engine,
    to_popup: relm4::Sender<PopupMsg>,
    process_path: String,
}

#[derive(Debug)]
enum OverlayMsg {
    Sentence { sentence: String },
    ScanSentence { byte_index_i32: i32 },
}

#[relm4::component(pub, async)]
impl AsyncComponent for Overlay {
    type Init = OverlayConfig;
    type Input = OverlayMsg;
    type Output = ();
    type CommandOutput = ();

    fn init_loading_widgets(root: Self::Root) -> Option<LoadingWidgets> {
        view! {
            #[local]
            root {
                set_title: Some("Wordbase Overlay"),
                set_width_request: 180,
                set_height_request: 100,

                #[name(spinner)]
                adw::Spinner {},
            }
        }
        Some(LoadingWidgets::new(root, spinner))
    }

    view! {
        adw::Window {
            set_title: Some(&format!("{} — Wordbase", init.process_path)),

            gtk::ScrolledWindow {
                set_hexpand: true,
                set_vexpand: true,

                #[name(sentence)]
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
                },
            },
        }
    }

    async fn init(
        init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let widgets = view_output!();
        let model = Self {
            engine: init.engine,
            to_popup: init.to_popup,
            sentence: widgets.sentence.clone(),
        };
        setup_root_opacity_animation(&root);
        setup_sentence_scan(&widgets.sentence, &sender);
        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        message: Self::Input,
        _sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            OverlayMsg::Sentence { sentence } => {
                self.sentence.set_text(&sentence);
            }
            OverlayMsg::ScanSentence { byte_index_i32 } => {
                let text = &self.sentence.text();
                let Ok(byte_index) = usize::try_from(byte_index_i32) else {
                    return;
                };
                let Some((before, after)) = text.split_at_checked(byte_index) else {
                    return;
                };
                let char_index = before.chars().count();
                let Ok(char_index_i32) = i32::try_from(char_index) else {
                    return;
                };

                let char_rect = self.sentence.layout().index_to_pos(byte_index_i32);
                let (char_rel_x, char_rel_y) = (
                    // anchor to bottom-right of character
                    (char_rect.x() + char_rect.width()) as f32 / pango::SCALE as f32,
                    (char_rect.y() + char_rect.height()) as f32 / pango::SCALE as f32,
                );
                let abs_point = self
                    .sentence
                    .compute_point(root, &graphene::Point::new(char_rel_x, char_rel_y))
                    .expect("`sentence` and `root` should share a common ancestor");

                // TODO variable offset
                #[expect(clippy::cast_possible_truncation, reason = "no other way to convert")]
                let origin = (
                    abs_point.x() as i32 + POPUP_OFFSET.0,
                    abs_point.y() as i32 + POPUP_OFFSET.1,
                );

                let Ok(records) = self
                    .engine
                    .lookup(
                        // TODO: add some scrollback to context
                        text,
                        byte_index,
                        SUPPORTED_RECORD_KINDS,
                    )
                    .await
                else {
                    return;
                };
                if records.is_empty() {
                    return;
                }

                let bytes_scanned = records
                    .iter()
                    .map(|record| record.bytes_scanned)
                    .max()
                    .unwrap_or_default();
                let chars_scanned_i32 = after
                    .get(..bytes_scanned)
                    .map(|s| s.chars().count())
                    .and_then(|n| i32::try_from(n).ok())
                    .unwrap_or_default();
                self.sentence
                    .select_region(char_index_i32, char_index_i32 + chars_scanned_i32);

                _ = self.to_popup.send(PopupMsg::Request {
                    target_window: WindowFilter {
                        id: None,
                        title: root.title().map(|s| s.to_string()),
                        wm_class: Some(APP_ID.to_owned()),
                    },
                    origin,
                    anchor: PopupAnchor::TopLeft,
                    records: Arc::new(records),
                });
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

fn setup_sentence_scan(label: &gtk::Label, sender: &AsyncComponentSender<Overlay>) {
    let controller = gtk::EventControllerMotion::new();
    label.add_controller(controller.clone());

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
        _ = sender
            .input_sender()
            .send(OverlayMsg::ScanSentence { byte_index_i32 });
    });
}
