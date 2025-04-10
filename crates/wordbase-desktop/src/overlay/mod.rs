mod ui;

use {
    crate::{APP_ID, SignalHandler, platform::Platform, popup, record_view},
    foldhash::{HashMap, HashMapExt},
    glib::clone,
    gtk4::pango::ffi::PANGO_SCALE,
    relm4::{
        adw::{
            self, gio,
            gtk::{graphene, pango},
            prelude::*,
        },
        prelude::*,
    },
    std::{
        any::Any,
        collections::hash_map::Entry,
        sync::{
            Arc,
            atomic::{self, AtomicI32},
        },
    },
    tracing::trace,
    wordbase::{PopupAnchor, TexthookerSentence, WindowFilter},
    wordbase_engine::{Engine, Event},
};

#[derive(Debug)]
pub struct Overlays {
    engine: Engine,
    platform: Arc<dyn Platform>,
    to_popup: relm4::Sender<popup::Msg>,
    by_process_path: HashMap<String, AsyncController<Overlay>>,
}

#[derive(Debug)]
pub enum OverlaysMsg {
    #[doc(hidden)]
    Remove(String),
}

impl AsyncComponent for Overlays {
    type Init = (Engine, Arc<dyn Platform>, relm4::Sender<popup::Msg>);
    type Input = OverlaysMsg;
    type Output = ();
    type CommandOutput = TexthookerSentence;
    type Root = ();
    type Widgets = ();

    fn init_root() -> Self::Root {}

    async fn init(
        (engine, platform, to_popup): Self::Init,
        _root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let mut recv_event = engine.recv_event();
        sender.command(move |out, shutdown| {
            shutdown
                .register(async move {
                    while let Ok(event) = recv_event.recv().await {
                        if let Event::TexthookerSentence(event) = event {
                            out.emit(event);
                        }
                    }
                })
                .drop_on_shutdown()
        });

        let model = Self {
            engine,
            platform,
            to_popup,
            by_process_path: HashMap::new(),
        };
        AsyncComponentParts { model, widgets: () }
    }

    async fn update(
        &mut self,
        message: Self::Input,
        _sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            OverlaysMsg::Remove(process_path) => {
                self.by_process_path.remove(&process_path);
            }
        }
    }

    async fn update_cmd(
        &mut self,
        event: Self::CommandOutput,
        sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        trace!(
            "New sentence for {:?}: {:?}",
            event.process_path, event.sentence
        );

        let overlay = match self.by_process_path.entry(event.process_path) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let process_path = entry.key().clone();
                let overlay = Overlay::builder()
                    .launch((
                        self.engine.clone(),
                        self.platform.clone(),
                        self.to_popup.clone(),
                        process_path.clone(),
                    ))
                    .forward(sender.input_sender(), move |resp| match resp {
                        OverlayResponse::Closed => OverlaysMsg::Remove(process_path.clone()),
                    });
                entry.insert(overlay)
            }
        };
        overlay.emit(OverlayMsg::Sentence(event.sentence));
    }
}

#[derive(Debug)]
struct Overlay {
    engine: Engine,
    to_popup: relm4::Sender<popup::Msg>,
    settings: gio::Settings,
    _window_guard: Box<dyn Any>,
    _font_size_signal_handler: SignalHandler,
}

#[derive(Debug)]
enum OverlayMsg {
    Sentence(String),
    ScanSentence { byte_index_i32: i32 },
    FontSize,
}

#[derive(Debug)]
enum OverlayResponse {
    Closed,
}

impl AsyncComponent for Overlay {
    type Init = (Engine, Arc<dyn Platform>, relm4::Sender<popup::Msg>, String);
    type Input = OverlayMsg;
    type Output = OverlayResponse;
    type CommandOutput = ();
    type Root = ui::Overlay;
    type Widgets = ();

    fn init_root() -> Self::Root {
        ui::Overlay::new()
    }

    async fn init(
        (engine, platform, to_popup, process_path): Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        root.set_title(Some(&format!("{process_path} — Wordbase")));
        relm4::main_application().add_window(&root);

        let window_guard = platform.init_overlay(root.upcast_ref()).await.unwrap();
        let settings = gio::Settings::new(APP_ID);

        root.connect_close_request(clone!(
            #[strong]
            sender,
            move |_| {
                _ = sender.output(OverlayResponse::Closed);
                glib::Propagation::Proceed
            }
        ));

        settings
            .bind(OVERLAY_FONT_SIZE, &root.font_size(), "value")
            .build();

        let default_opacity_idle = settings
            .default_value(OVERLAY_OPACITY_IDLE)
            .expect("should have default value")
            .get::<f64>()
            .expect("should be double");
        root.opacity_idle_scale().add_mark(
            default_opacity_idle,
            gtk::PositionType::Bottom,
            Some(&format!("{:.0}%", default_opacity_idle * 100.0)),
        );
        settings
            .bind(OVERLAY_OPACITY_IDLE, &root.opacity_idle(), "value")
            .build();

        let default_opacity_hover = settings
            .default_value(OVERLAY_OPACITY_HOVER)
            .expect("should have default value")
            .get::<f64>()
            .expect("should be double");
        root.opacity_hover_scale().add_mark(
            default_opacity_hover,
            gtk::PositionType::Bottom,
            Some(&format!("{:.0}%", default_opacity_hover * 100.0)),
        );
        settings
            .bind(OVERLAY_OPACITY_HOVER, &root.opacity_hover(), "value")
            .build();

        let font_size_signal_handler = SignalHandler::new(&settings, |it| {
            it.connect_changed(
                Some(OVERLAY_FONT_SIZE),
                clone!(
                    #[strong]
                    sender,
                    move |_, _| {
                        sender.input(OverlayMsg::FontSize);
                    }
                ),
            )
        });
        setup_root_opacity_animation(root.upcast_ref(), &settings);

        let model = Self {
            engine,
            to_popup,
            settings,
            _window_guard: window_guard,
            _font_size_signal_handler: font_size_signal_handler,
        };
        setup_sentence_scan(&root, &sender);
        AsyncComponentParts { model, widgets: () }
    }

    async fn update_with_view(
        &mut self,
        _widgets: &mut Self::Widgets,
        message: Self::Input,
        _sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            OverlayMsg::Sentence(sentence) => {
                root.sentence().set_text(&sentence);
                // for some reason the label doesn't persist its font after new text is set
                self.update_font(root);
            }
            OverlayMsg::FontSize => {
                self.update_font(root);
            }
            OverlayMsg::ScanSentence { byte_index_i32 } => {
                let sentence = root.sentence();
                let text = &sentence.text();
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

                let char_rect = sentence.layout().index_to_pos(byte_index_i32);
                let (char_rel_x, char_rel_y) = (
                    // anchor to bottom-right of character
                    (char_rect.x() + char_rect.width()) as f32 / pango::SCALE as f32,
                    (char_rect.y() + char_rect.height()) as f32 / pango::SCALE as f32,
                );
                let abs_point = sentence
                    .compute_point(root, &graphene::Point::new(char_rel_x, char_rel_y))
                    .expect("`root` is an ancestor of `sentence`");

                // TODO variable offset
                #[expect(clippy::cast_possible_truncation, reason = "no other way to convert")]
                let origin = (
                    abs_point.x() as i32 + POPUP_OFFSET.0,
                    abs_point.y() as i32 + POPUP_OFFSET.1,
                );

                let Ok(records) = self
                    .engine
                    .lookup(text, byte_index, record_view::SUPPORTED_RECORD_KINDS)
                    .await
                else {
                    return;
                };

                let longest_scan_chars = record_view::longest_scan_chars(text, &records);
                sentence.select_region(
                    char_index_i32,
                    char_index_i32 + i32::try_from(longest_scan_chars).unwrap_or(i32::MAX),
                );

                self.to_popup.emit(popup::Msg::Present {
                    target_window: WindowFilter {
                        id: None,
                        title: root.title().map(|s| s.to_string()),
                        wm_class: Some(APP_ID.to_owned()),
                    },
                    origin,
                    anchor: PopupAnchor::TopLeft,
                });
                self.to_popup.emit(popup::Msg::Render { records });
            }
        }
    }
}

impl Overlay {
    fn update_font(&self, root: &ui::Overlay) {
        let mut font_desc = pango::FontDescription::new();
        let font_size = self.settings.double(OVERLAY_FONT_SIZE);
        let font_size_pango = (font_size * PANGO_SCALE as f64) as i32;
        font_desc.set_size(font_size_pango);
        root.sentence()
            .layout()
            .set_font_description(Some(&font_desc));
        root.sentence().queue_draw();
    }
}

fn setup_root_opacity_animation(root: &gtk::Window, settings: &gio::Settings) {
    let opacity_target = adw::PropertyAnimationTarget::new(root, "opacity");
    let animation = adw::TimedAnimation::builder()
        .widget(root)
        .duration(100)
        .target(&opacity_target)
        .build();
    settings
        .bind(OVERLAY_OPACITY_IDLE, &animation, "value-from")
        .build();
    settings
        .bind(OVERLAY_OPACITY_HOVER, &animation, "value-to")
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

fn setup_sentence_scan(root: &ui::Overlay, sender: &AsyncComponentSender<Overlay>) {
    let controller = gtk::EventControllerMotion::new();
    root.sentence().add_controller(controller.clone());

    let last_scan_byte_index = Arc::new(AtomicI32::new(-1));
    controller.connect_leave(clone!(
        #[strong]
        last_scan_byte_index,
        move |_| {
            last_scan_byte_index.store(-1, atomic::Ordering::Relaxed);
        }
    ));
    controller.connect_motion(clone!(
        #[strong]
        root,
        #[strong]
        sender,
        move |_, rel_x, rel_y| {
            #[expect(clippy::cast_possible_truncation, reason = "no other way to convert")]
            let (ch_x, ch_y) = (rel_x as i32 * pango::SCALE, rel_y as i32 * pango::SCALE);

            let sentence = root.sentence();
            let (valid, byte_index_i32, _grapheme_pos) = sentence.layout().xy_to_index(ch_x, ch_y);
            if !valid {
                return;
            }
            if byte_index_i32 == last_scan_byte_index.swap(byte_index_i32, atomic::Ordering::SeqCst)
            {
                // we're scanning the same character as in the last motion event
                return;
            }

            sender.input(OverlayMsg::ScanSentence { byte_index_i32 });
        }
    ));
}

const POPUP_OFFSET: (i32, i32) = (0, 10);

const OVERLAY_FONT_SIZE: &str = "overlay-font-size";
const OVERLAY_OPACITY_IDLE: &str = "overlay-opacity-idle";
const OVERLAY_OPACITY_HOVER: &str = "overlay-opacity-hover";
