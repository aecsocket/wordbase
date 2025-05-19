mod ui;

use {
    crate::{
        APP_BROKER, APP_ID, AppEvent, AppMsg, CURRENT_PROFILE, CURRENT_PROFILE_ID, SignalHandler,
        forward_events, gettext, platform::Platform, popup, record_view,
    },
    foldhash::{HashMap, HashMapExt},
    glib::clone,
    gtk4::pango::ffi::PANGO_SCALE,
    relm4::{
        adw::{
            self, gdk, gio,
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
    wordbase::{TexthookerSentence, WindowFilter},
    wordbase_engine::{Engine, EngineEvent},
};

#[derive(Debug)]
pub struct Overlays {
    engine: Engine,
    platform: Arc<dyn Platform>,
    to_popup: relm4::Sender<popup::Msg>,
    by_process_path: HashMap<String, AsyncController<Overlay>>,
}

#[derive(Debug)]
#[doc(hidden)]
pub enum OverlaysMsg {
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
                        if let EngineEvent::TexthookerSentence(event) = event {
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
    sentence: String,
    _window_guard: Box<dyn Any>,
    _font_size_handler: SignalHandler,
    _opacity_idle_handler: SignalHandler,
    _opacity_hover_handler: SignalHandler,
}

#[derive(Debug)]
enum OverlayMsg {
    Sentence(String),
    Copy,
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
    type CommandOutput = AppEvent;
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
        forward_events(&sender);
        root.connect_maximized_notify(|root| {
            root.unmaximize();
        });
        root.connect_fullscreened_notify(|root| {
            root.unfullscreen();
        });
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

        root.copy().connect_clicked(clone!(
            #[strong]
            sender,
            move |_| sender.input(OverlayMsg::Copy)
        ));
        root.manager()
            .connect_clicked(move |_| APP_BROKER.send(AppMsg::Present));

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

        settings
            .bind(OVERLAY_SCAN_TRIGGER, &root.scan_trigger(), "selected")
            .mapping(|from, _ty| {
                Some(
                    match from.str() {
                        Some("hover") => 0u32,
                        Some("click") => 1u32,
                        Some("shift") => 2u32,
                        Some("ctrl") => 3u32,
                        Some("alt") => 4u32,
                        _ => todo!(),
                    }
                    .to_value(),
                )
            })
            .set_mapping(|from, _ty| {
                Some(
                    match from.get::<u32>() {
                        Ok(0) => "hover",
                        Ok(1) => "click",
                        Ok(2) => "shift",
                        Ok(3) => "ctrl",
                        Ok(4) => "alt",
                        _ => todo!(),
                    }
                    .to_variant(),
                )
            })
            .build();

        let font_size_handler = SignalHandler::new(&settings, |it| {
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
        let (opacity_idle_handler, opacity_hover_handler) =
            setup_root_opacity_animation(root.upcast_ref(), &settings);

        let model = Self {
            engine,
            to_popup,
            settings,
            sentence: String::new(),
            _window_guard: window_guard,
            _font_size_handler: font_size_handler,
            _opacity_idle_handler: opacity_idle_handler,
            _opacity_hover_handler: opacity_hover_handler,
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
            OverlayMsg::Copy => {
                gdk::Display::default()
                    .expect("should have default display")
                    .clipboard()
                    .set_text(&self.sentence);

                root.toaster()
                    .add_toast(adw::Toast::new(gettext("Copied sentence to clipboard")));
            }
            OverlayMsg::Sentence(sentence) => {
                root.sentence().set_text(&sentence);
                // for some reason the label doesn't persist its font after new text is set
                self.update_font(root);
                self.sentence = sentence;
            }
            OverlayMsg::FontSize => {
                self.update_font(root);
            }
            OverlayMsg::ScanSentence { byte_index_i32 } => {
                let sentence = root.sentence();
                let text = &self.sentence;
                let Ok(byte_index) = usize::try_from(byte_index_i32) else {
                    return;
                };

                let Some((before, _)) = text.split_at_checked(byte_index) else {
                    return;
                };
                let char_index = before.chars().count();
                let Ok(char_index_i32) = i32::try_from(char_index) else {
                    return;
                };

                let Ok(records) = self
                    .engine
                    .lookup(
                        CURRENT_PROFILE_ID.read().unwrap(),
                        text,
                        byte_index,
                        record_view::SUPPORTED_RECORD_KINDS,
                    )
                    .await
                else {
                    return;
                };
                if records.is_empty() {
                    return;
                }

                let longest_scan_chars = record_view::longest_scan_chars(text, &records);
                let selection_end =
                    char_index_i32 + i32::try_from(longest_scan_chars).unwrap_or(i32::MAX);
                sentence.select_region(char_index_i32, selection_end);

                let local_rect = sentence.layout().index_to_pos(byte_index_i32);
                let local_origin_nw = (
                    local_rect.x() as f32 / pango::SCALE as f32,
                    local_rect.y() as f32 / pango::SCALE as f32,
                );
                let local_origin_se = (
                    (local_rect.x() + local_rect.width()) as f32 / pango::SCALE as f32,
                    (local_rect.y() + local_rect.height()) as f32 / pango::SCALE as f32,
                );

                let abs_origin_nw = sentence
                    .compute_point(
                        root,
                        &graphene::Point::new(local_origin_nw.0, local_origin_nw.1),
                    )
                    .expect("`root` is an ancestor of `sentence`");
                let abs_origin_se = sentence
                    .compute_point(
                        root,
                        &graphene::Point::new(local_origin_se.0, local_origin_se.1),
                    )
                    .expect("`root` is an ancestor of `sentence`");

                #[expect(clippy::cast_possible_truncation, reason = "no other way to convert")]
                let (origin_nw, origin_se) = (
                    (abs_origin_nw.x() as i32, abs_origin_nw.y() as i32),
                    (abs_origin_se.x() as i32, abs_origin_se.y() as i32),
                );

                self.to_popup.emit(popup::Msg::Present {
                    target_window: WindowFilter {
                        id: None,
                        title: root.title().map(|s| s.to_string()),
                        wm_class: Some(APP_ID.to_owned()),
                    },
                    origin_nw,
                    origin_se,
                });
                self.to_popup.emit(popup::Msg::Render {
                    records,
                    sentence: text.clone(),
                    cursor: byte_index,
                });
            }
        }
    }

    async fn update_cmd_with_view(
        &mut self,
        _widgets: &mut Self::Widgets,
        message: Self::CommandOutput,
        _sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        if matches!(message, AppEvent::FontSet) {
            self.update_font(root);
        }
    }
}

impl Overlay {
    fn update_font(&self, root: &ui::Overlay) {
        let mut font_desc = pango::FontDescription::new();

        let profile = CURRENT_PROFILE.read().as_ref().cloned().unwrap();
        if let Some(family) = &profile.font_family {
            font_desc.set_family(family);
        }

        let font_size = self.settings.double(OVERLAY_FONT_SIZE);
        #[expect(clippy::cast_possible_truncation, reason = "no other way to convert")]
        let font_size_pango = (font_size * f64::from(PANGO_SCALE)) as i32;
        font_desc.set_size(font_size_pango);

        root.sentence()
            .layout()
            .set_font_description(Some(&font_desc));

        // todo
        root.sentence()
            .pango_context()
            .set_language(Some(&pango::Language::from_string("ja")));

        root.sentence().queue_draw();
    }
}

fn setup_root_opacity_animation(
    root: &gtk::Window,
    settings: &gio::Settings,
) -> (SignalHandler, SignalHandler) {
    let opacity_target = adw::PropertyAnimationTarget::new(root, "opacity");
    let animation = adw::TimedAnimation::builder()
        .widget(root)
        .duration(100)
        .target(&opacity_target)
        .build();

    settings
        .bind(OVERLAY_OPACITY_IDLE, &animation, "value-from")
        .build();
    let idle_handler = SignalHandler::new(settings, |it| {
        it.connect_changed(
            Some(OVERLAY_OPACITY_IDLE),
            clone!(
                #[strong]
                animation,
                move |_, _| {
                    animation.set_reverse(true);
                    animation.play();
                    animation.skip();
                }
            ),
        )
    });

    settings
        .bind(OVERLAY_OPACITY_HOVER, &animation, "value-to")
        .build();
    let hover_handler = SignalHandler::new(settings, |it| {
        it.connect_changed(
            Some(OVERLAY_OPACITY_HOVER),
            clone!(
                #[strong]
                animation,
                move |_, _| {
                    animation.set_reverse(false);
                    animation.play();
                    animation.skip();
                }
            ),
        )
    });

    let controller = gtk::EventControllerMotion::new();
    root.add_controller(controller.clone());

    controller.connect_enter(clone!(
        #[strong]
        animation,
        move |_, _, _| {
            animation.set_reverse(false);
            animation.play();
        }
    ));
    controller.connect_leave(clone!(
        #[strong]
        animation,
        move |_| {
            animation.set_reverse(true);
            animation.play();
        }
    ));
    animation.set_reverse(true);
    animation.play();

    (idle_handler, hover_handler)
}

fn setup_sentence_scan(root: &ui::Overlay, sender: &AsyncComponentSender<Overlay>) {
    let controller = gtk::EventControllerMotion::new();
    root.sentence().add_controller(controller.clone());

    let last_scan_byte_index = Arc::new(AtomicI32::new(-1));
    let try_scan = {
        let root = root.clone();
        let sender = sender.clone();
        let last_scan_byte_index = last_scan_byte_index.clone();
        move |rel_x: f64, rel_y: f64| {
            let modifiers = gdk::Display::default()
                .and_then(|display| display.default_seat())
                .and_then(|seat| seat.keyboard())
                .map(|pointer| pointer.modifier_state());
            // if modifiers.contains(gdk::ModifierType::BUTTON1_MASK) {
            //     return;
            // }

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
    };

    // we specifically need this to run on BOTH enter and leave events,
    // because we aren't guaranteed to get a leave event!
    // e.g. if the window is unfocused without the pointer leaving
    controller.connect_enter(clone!(
        #[strong]
        try_scan,
        #[strong]
        last_scan_byte_index,
        move |_, rel_x, rel_y| {
            last_scan_byte_index.store(-1, atomic::Ordering::SeqCst);
            try_scan(rel_x, rel_y);
        }
    ));
    controller.connect_leave(clone!(
        #[strong]
        last_scan_byte_index,
        move |_| {
            last_scan_byte_index.store(-1, atomic::Ordering::SeqCst);
        }
    ));
    controller.connect_motion(clone!(
        #[strong]
        try_scan,
        move |_, rel_x, rel_y| {
            try_scan(rel_x, rel_y);
        }
    ));
}

const OVERLAY_FONT_SIZE: &str = "overlay-font-size";
const OVERLAY_OPACITY_IDLE: &str = "overlay-opacity-idle";
const OVERLAY_OPACITY_HOVER: &str = "overlay-opacity-hover";
const OVERLAY_SCAN_TRIGGER: &str = "overlay-scan-trigger";
