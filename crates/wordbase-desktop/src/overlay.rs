use {
    crate::{
        APP_ID, platform::Platform, popup::AppPopupRequest, record::render::SUPPORTED_RECORD_KINDS,
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
        component::AsyncConnector,
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
    wordbase::{Lookup, PopupAnchor, TexthookerSentence, WindowFilter},
    wordbase_engine::Engine,
};

pub async fn run(
    app: adw::Application,
    platform: Arc<dyn Platform>,
    engine: Engine,
    mut recv_sentence: mpsc::Receiver<TexthookerSentence>,
    popup: relm4::Sender<AppPopupRequest>,
) -> Result<Never> {
    let mut overlays = HashMap::<String, AsyncController<Overlay>>::new();

    loop {
        let event = recv_sentence
            .recv()
            .await
            .context("sentence channel closed")?;
        trace!(
            "New sentence for {:?}: {:?}",
            event.process_path, event.sentence
        );

        if let Err(err) = handle(&app, &*platform, &engine, &popup, &mut overlays, event).await {
            warn!("Failed to handle new sentence event: {err:?}");
        }
    }
}

async fn handle(
    app: &adw::Application,
    platform: &dyn Platform,
    engine: &Engine,
    popup: &relm4::Sender<AppPopupRequest>,
    overlays: &mut HashMap<String, AsyncController<Overlay>>,
    TexthookerSentence {
        process_path,
        sentence,
    }: TexthookerSentence,
) -> Result<()> {
    match overlays.entry(process_path.clone()) {
        Entry::Occupied(entry) => {
            _ = entry.get().sender().send(OverlayMsg::Sentence { sentence });
        }
        Entry::Vacant(entry) => {
            info!("Creating overlay for new process {process_path:?}");

            let overlay = connector(
                app,
                platform,
                OverlayConfig {
                    engine: engine.clone(),
                    popup: popup.clone(),
                    process_path,
                    sentence,
                },
            )
            .await
            .context("failed to create overlay window")?
            .detach();

            entry.insert(overlay);
        }
    }
    Ok(())
}

async fn connector(
    app: &adw::Application,
    platform: &dyn Platform,
    config: OverlayConfig,
) -> Result<AsyncConnector<Overlay>> {
    let connector = Overlay::builder().launch(config);
    let window = connector.widget();
    app.add_window(window);
    platform.init_overlay(window).await?;
    Ok(connector)
}

#[derive(Debug)]
struct Overlay {
    engine: Engine,
    popup: relm4::Sender<AppPopupRequest>,
    sentence: String,
}

#[derive(Debug)]
struct OverlayConfig {
    engine: Engine,
    popup: relm4::Sender<AppPopupRequest>,
    process_path: String,
    sentence: String,
}

#[derive(Debug)]
enum OverlayMsg {
    Sentence { sentence: String },
    Scan { lookup: Lookup, origin: (i32, i32) },
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

    async fn init(
        init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let model = Self {
            engine: init.engine,
            sentence: init.sentence,
            popup: init.popup,
        };
        let widgets = view_output!();
        setup_root_opacity_animation(&root);
        setup_sentence_scan(&root, &widgets.sentence_label, &sender);
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
                self.sentence = sentence;
            }
            OverlayMsg::Scan { lookup, origin } => {
                let Ok(records) = self.engine.lookup(&lookup, SUPPORTED_RECORD_KINDS).await else {
                    return;
                };
                if records.is_empty() {
                    return;
                }

                _ = self.popup.send(AppPopupRequest {
                    target_window: WindowFilter {
                        id: None,
                        title: root.title().map(|s| s.to_string()),
                        wm_class: Some(APP_ID.to_owned()),
                    },
                    origin,
                    anchor: PopupAnchor::BottomCenter,
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

fn setup_sentence_scan(
    root: &adw::Window,
    label: &gtk::Label,
    sender: &AsyncComponentSender<Overlay>,
) {
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
            .expect("`label` and `root` should share a common ancestor");
        #[expect(clippy::cast_possible_truncation, reason = "no other way to convert")]
        let origin = (abs_point.x() as i32 + 16, abs_point.y() as i32 + 16);
        _ = sender
            .input_sender()
            .send(OverlayMsg::Scan { lookup, origin });
    });
}
