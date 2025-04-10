use {
    crate::theme::{DEFAULT_THEME, Theme},
    maud::html,
    relm4::{
        adw::{gdk, gio, prelude::*},
        prelude::*,
    },
    std::sync::Arc,
    tracing::{debug, info},
    webkit6::prelude::*,
    wordbase::{RecordKind, RecordLookup},
    wordbase_engine::{dictionary::Dictionaries, html},
};

#[derive(Debug)]
pub struct Model {
    pub custom_theme: Option<Arc<Theme>>,
    pub dictionaries: Arc<Dictionaries>,
    pub records: Vec<RecordLookup>,
}

pub const SUPPORTED_RECORD_KINDS: &[RecordKind] = RecordKind::ALL;

#[derive(Debug)]
pub enum Msg {
    CustomTheme(Option<Arc<Theme>>),
    Render {
        dictionaries: Arc<Dictionaries>,
        records: Vec<RecordLookup>,
    },
}

#[derive(Debug)]
pub struct Config {
    pub custom_theme: Option<Arc<Theme>>,
}

#[derive(Debug)]
pub enum Response {
    Query(String),
}

#[relm4::component(pub)]
impl Component for Model {
    type Init = Config;
    type Input = Msg;
    type Output = Response;
    type CommandOutput = ();

    view! {
        webkit6::WebView {
            set_hexpand: true,
            set_vexpand: true,
            set_settings = &webkit6::Settings {
                set_enable_smooth_scrolling: false,
            },
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

    fn init(
        config: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self {
            custom_theme: config.custom_theme,
            dictionaries: Arc::default(),
            records: Vec::new(),
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            Msg::CustomTheme(theme) => {
                self.custom_theme = theme;
            }
            Msg::Render {
                dictionaries,
                records,
            } => {
                self.dictionaries = dictionaries;
                self.records = records;
            }
        }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        self.update(message, sender.clone(), root);
        self.update_view(widgets, sender);

        let records_html = html::render_records(&self.dictionaries.by_id, &self.records);
        let full_html = html! {
            style {
                (DEFAULT_THEME.style)
            }

            style {
                (self.custom_theme.as_ref().map(|theme| theme.style.as_str()).unwrap_or_default())
            }

            .records {
                (records_html)
            }
        };
        root.load_html(&full_html.0, None);
    }
}

fn on_decide_policy(decision: &webkit6::PolicyDecision, sender: &ComponentSender<Model>) {
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
