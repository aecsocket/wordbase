use {
    crate::{
        APP_ID, AppEvent, CURRENT_PROFILE, CURRENT_PROFILE_ID, SignalHandler, forward_events, html,
        theme::{CUSTOM_THEMES, DEFAULT_THEME, ThemeName},
    },
    glib::clone,
    maud::{Markup, PreEscaped, html},
    relm4::{
        adw::{gdk, gio, prelude::*},
        prelude::*,
    },
    std::sync::Arc,
    tracing::{debug, info},
    webkit6::prelude::*,
    wordbase::{RecordKind, RecordLookup, Term},
    wordbase_engine::Engine,
};

#[derive(Debug)]
pub struct Model {
    engine: Engine,
    settings: gio::Settings,
    records: Vec<RecordLookup>,
    sentence: String,
    cursor: usize,
    _custom_theme_handler: SignalHandler,
    _accent_color_handler: SignalHandler,
}

pub const SUPPORTED_RECORD_KINDS: &[RecordKind] = RecordKind::ALL;

#[derive(Debug)]
pub enum Msg {
    Render {
        records: Vec<RecordLookup>,
        sentence: String,
        cursor: usize,
    },
    #[doc(hidden)]
    AddAnkiNote(Term),
    #[doc(hidden)]
    Rerender,
}

#[derive(Debug)]
pub enum Response {
    Html(Markup),
    Query(String),
}

fn new_web_view() -> webkit6::WebView {
    // if we don't do this, then the app will make some files in ~/.local/share:
    // ├── mediakeys
    // │   └── v1
    // │       └── salt
    // └── storage
    //     └── salt
    webkit6::WebView::builder()
        .network_session(&webkit6::NetworkSession::new_ephemeral())
        .build()
}

#[relm4::component(pub, async)]
impl AsyncComponent for Model {
    type Init = Engine;
    type Input = Msg;
    type Output = Response;
    type CommandOutput = AppEvent;

    view! {
        new_web_view() -> webkit6::WebView {
            set_hexpand: true,
            set_vexpand: true,
            set_background_color: &gdk::RGBA::new(0.0, 0.0, 0.0, 0.0),
            connect_context_menu => |_, _, _| {
                // prevent opening context menu
                true
            },
            connect_decide_policy => move |_, decision, _| {
                on_decide_policy(decision, &sender);
                true
            },
        }
    }

    async fn init(
        engine: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        forward_events(&sender);
        let settings = gio::Settings::new(APP_ID);
        CUSTOM_THEMES.subscribe(sender.input_sender(), |_| Msg::Rerender);

        let custom_theme_handler = SignalHandler::new(&settings, |it| {
            it.connect_changed(
                Some(CUSTOM_THEME),
                clone!(
                    #[strong]
                    sender,
                    move |_, _| sender.input(Msg::Rerender)
                ),
            )
        });

        let accent_color_handler = SignalHandler::new(&adw::StyleManager::default(), |it| {
            it.connect_accent_color_rgba_notify(clone!(
                #[strong]
                sender,
                move |_| sender.input(Msg::Rerender)
            ))
        });

        let content_manager = root.user_content_manager().unwrap();
        content_manager.connect_script_message_received(
            Some("add_note"),
            clone!(
                #[strong]
                sender,
                move |_, value| {
                    let json = value.to_json(0).unwrap();
                    let term = serde_json::from_str(&json).unwrap();
                    sender.input(Msg::AddAnkiNote(term));
                }
            ),
        );
        content_manager.register_script_message_handler("add_note", None);

        let model = Self {
            engine,
            settings,
            records: Vec::new(),
            sentence: String::new(),
            cursor: 0,
            _custom_theme_handler: custom_theme_handler,
            _accent_color_handler: accent_color_handler,
        };
        let widgets = view_output!();

        AsyncComponentParts { model, widgets }
    }

    async fn update_cmd_with_view(
        &mut self,
        _widgets: &mut Self::Widgets,
        message: Self::CommandOutput,
        sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        if matches!(message, AppEvent::FontSet) {
            sender.input(Msg::Rerender);
        }
    }

    async fn update_with_view(
        &mut self,
        _widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            Msg::Render {
                records,
                sentence,
                cursor,
            } => {
                self.records = records;
                self.sentence = sentence;
                self.cursor = cursor;
            }
            Msg::AddAnkiNote(term) => {
                self.engine
                    .add_anki_note(
                        CURRENT_PROFILE_ID.read().unwrap(),
                        &self.sentence,
                        self.cursor,
                        &term,
                        None,
                        None,
                    )
                    .await;
            }
            Msg::Rerender => {}
        }
        update_view(self, root, &sender);
    }
}

fn update_view(model: &Model, root: &webkit6::WebView, sender: &AsyncComponentSender<Model>) {
    let profile = CURRENT_PROFILE.read().as_ref().cloned().unwrap();
    let settings = webkit6::Settings::new();
    settings.set_enable_page_cache(false);
    settings.set_enable_smooth_scrolling(false);
    if let Some(family) = &profile.config.font_family {
        settings.set_default_font_family(family);
    }
    root.set_settings(&settings);

    let dictionaries = model.engine.dictionaries();
    let records_html = html::render_records(
        &|id| dictionaries.get(&id).map(|dict| &**dict),
        &model.records,
    );

    let custom_theme_name = model.settings.string(CUSTOM_THEME);
    let custom_theme = CUSTOM_THEMES
        .read()
        .get(&ThemeName(Arc::from(custom_theme_name.to_string())))
        .map(|theme| theme.theme.clone());

    let accent_color = adw::StyleManager::default().accent_color_rgba();
    let root_style = format!(
        ":root {{
            --accent-color: rgb({} {} {});
        }}",
        accent_color.red() * 255.0,
        accent_color.green() * 255.0,
        accent_color.blue() * 255.0
    );

    let full_html = html! {
        style {
            (escape_style(&root_style))
        }

        style {
            (escape_style(&DEFAULT_THEME.style))
        }

        style {
            (escape_style(
                custom_theme
                    .as_ref()
                    .map(|theme| theme.style.as_str())
                    .unwrap_or_default(),
            ))
        }

        .records {
            (records_html)
        }
    };
    root.load_html(&full_html.0, None);

    _ = sender.output(Response::Html(records_html));
}

fn on_decide_policy(decision: &webkit6::PolicyDecision, sender: &AsyncComponentSender<Model>) {
    let Some(decision) = decision.downcast_ref::<webkit6::NavigationPolicyDecision>() else {
        return;
    };
    let Some(mut action) = decision.navigation_action() else {
        return;
    };
    if !action.is_user_gesture() {
        return;
    };
    decision.ignore();

    let Some(request) = action.request() else {
        return;
    };
    let Some(uri) = request.uri() else {
        return;
    };
    debug!("Figuring out how to open {uri:?}");

    if let Some(form_uri) = uri.strip_prefix("?") {
        if let Some((_, query)) =
            form_urlencoded::parse(form_uri.as_bytes()).find(|(key, _)| key == "query")
        {
            let query = query.into_owned();
            info!("Opening {query:?} as query");
            _ = sender.output(Response::Query(query));
        }
        return;
    }

    info!("Opening {uri:?} in browser");
    gtk::UriLauncher::new(&uri).launch(None::<&gtk::Window>, None::<&gio::Cancellable>, |_| {});
}

pub fn longest_scan_chars(query: &str, records: &[RecordLookup]) -> usize {
    records
        .iter()
        .map(|record| record.bytes_scanned)
        .max()
        .and_then(|longest_scan_bytes| query.get(..longest_scan_bytes).map(|s| s.chars().count()))
        .unwrap_or(0)
}

pub fn should_requery(event: &AppEvent) -> bool {
    matches!(
        event,
        AppEvent::DictionaryEnabledSet(_, _)
            | AppEvent::DictionarySortingSet(_)
            | AppEvent::DictionaryRemoved(_)
    )
}

pub const CUSTOM_THEME: &str = "custom-theme";

fn escape_style(input: &str) -> PreEscaped<String> {
    let mut s = String::new();
    escape_style_to_string(input, &mut s);
    PreEscaped(s)
}

// copied from
// https://github.com/lambda-fairy/maud/blob/c0df34f1b685fdffcb2bf08884629e4576b5748b/maud/src/escape.rs
fn escape_style_to_string(input: &str, output: &mut String) {
    for b in input.bytes() {
        match b {
            b'&' => output.push_str("&amp;"),
            b'<' => output.push_str("&lt;"),
            b'>' => output.push_str("&gt;"),
            // modified: escaping `"` breaks CSS
            // b'"' => output.push_str("&quot;"),
            _ => unsafe { output.as_mut_vec().push(b) },
        }
    }
}
