use {
    crate::{Dictionaries, theme::Theme},
    maud::html,
    relm4::{
        adw::{gdk, prelude::*},
        prelude::*,
    },
    std::sync::Arc,
    tracing::{debug, info, warn},
    webkit6::prelude::*,
    wordbase::{RecordKind, LookupResult},
    wordbase_engine::html,
};

#[derive(Debug)]
pub struct RecordRender {
    default_theme: Arc<Theme>,
    custom_theme: Option<Arc<Theme>>,
    web_view: webkit6::WebView,
    dictionaries: Arc<Dictionaries>,
    records: Arc<Records>,
}

pub type Records = Vec<LookupResult>;

pub const SUPPORTED_RECORD_KINDS: &[RecordKind] = RecordKind::ALL;

#[derive(Debug)]
pub struct RecordRenderConfig {
    pub default_theme: Arc<Theme>,
    pub custom_theme: Option<Arc<Theme>>,
    pub dictionaries: Arc<Dictionaries>,
    pub records: Arc<Records>,
}

#[derive(Debug)]
pub enum RecordRenderMsg {
    DefaultTheme(Arc<Theme>),
    CustomTheme(Option<Arc<Theme>>),
    Dictionaries(Arc<Dictionaries>),
    Records(Arc<Records>),
}

#[derive(Debug)]
pub enum RecordRenderResponse {
    RequestLookup { query: String },
}

#[relm4::component(pub)]
impl SimpleComponent for RecordRender {
    type Init = RecordRenderConfig;
    type Input = RecordRenderMsg;
    type Output = RecordRenderResponse;

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
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self {
            default_theme: init.default_theme,
            custom_theme: init.custom_theme,
            web_view: root.clone(),
            dictionaries: init.dictionaries,
            records: init.records,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            RecordRenderMsg::DefaultTheme(theme) => {
                self.default_theme = theme;
                self.update_web_view();
            }
            RecordRenderMsg::CustomTheme(theme) => {
                self.custom_theme = theme;
                self.update_web_view();
            }
            RecordRenderMsg::Dictionaries(dictionaries) => {
                self.dictionaries = dictionaries;
                self.update_web_view();
            }
            RecordRenderMsg::Records(records) => {
                self.records = records;
                self.update_web_view();
            }
        }
    }
}

impl RecordRender {
    fn update_web_view(&self) {
        let records_html = html::render_records(&self.dictionaries, &self.records);
        let full_html = html! {
            style {
                (self.default_theme.style)
            }

            style {
                (self.custom_theme.as_ref().map(|theme| theme.style.as_str()).unwrap_or_default())
            }

            .records {
                (records_html)
            }
        };
        self.web_view.load_html(&full_html.0, None);
    }
}

fn on_decide_policy(decision: &webkit6::PolicyDecision, sender: &ComponentSender<RecordRender>) {
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
            _ = sender.output(RecordRenderResponse::RequestLookup { query });
        }
        return;
    }

    info!("Opening {uri:?} in browser");
    if let Err(err) = open::that_detached(&uri) {
        warn!("Failed to open {uri:?}: {err:?}");
    }
}
