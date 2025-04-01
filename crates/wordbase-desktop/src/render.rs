use {
    crate::theme::Theme,
    foldhash::{HashMap, HashMapExt},
    maud::html,
    relm4::{
        adw::{gdk, prelude::*},
        prelude::*,
    },
    std::sync::Arc,
    tracing::warn,
    webkit6::prelude::*,
    wordbase::{Dictionary, DictionaryId, RecordLookup},
    wordbase_engine::html,
};

#[derive(Debug)]
pub struct RecordRender {
    default_theme: Arc<Theme>,
    custom_theme: Option<Arc<Theme>>,
    web_view: webkit6::WebView,
    dictionaries: Arc<HashMap<DictionaryId, Dictionary>>,
    records: Vec<RecordLookup>,
}

#[derive(Debug)]
pub struct RecordRenderConfig {
    pub default_theme: Arc<Theme>,
    pub custom_theme: Option<Arc<Theme>>,
}

#[derive(Debug)]
pub enum RecordRenderMsg {
    SetDefaultTheme(Arc<Theme>),
    SetCustomTheme(Option<Arc<Theme>>),
    Lookup {
        dictionaries: Arc<HashMap<DictionaryId, Dictionary>>,
        records: Vec<RecordLookup>,
    },
}

#[derive(Debug)]
pub enum RecordRenderResponse {}

#[relm4::component(pub)]
impl SimpleComponent for RecordRender {
    type Init = RecordRenderConfig;
    type Input = RecordRenderMsg;
    type Output = RecordRenderResponse;

    view! {
        webkit6::WebView {
            set_hexpand: true,
            set_vexpand: true,
            set_background_color: &gdk::RGBA::new(0.0, 0.0, 0.0, 0.0),
            connect_context_menu => |_, _, _| {
                // prevent opening context menu
                true
            },
            connect_decide_policy => |_, decision, _| {
                on_decide_policy(decision);
                true
            },
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let widgets = view_output!();
        let model = Self {
            default_theme: init.default_theme,
            custom_theme: init.custom_theme,
            web_view: root,
            dictionaries: Arc::new(HashMap::new()),
            records: Vec::new(),
        };
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            RecordRenderMsg::SetDefaultTheme(theme) => {
                self.default_theme = theme;
                self.update_web_view();
            }
            RecordRenderMsg::SetCustomTheme(theme) => {
                self.custom_theme = theme;
                self.update_web_view();
            }
            RecordRenderMsg::Lookup {
                dictionaries,
                records,
            } => {
                self.dictionaries = dictionaries;
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

            (records_html)
        };
        self.web_view.load_html(&full_html.0, None);
    }
}

fn on_decide_policy(decision: &webkit6::PolicyDecision) {
    if let Some(decision) = decision.downcast_ref::<webkit6::NavigationPolicyDecision>() {
        if let Some(mut action) = decision.navigation_action() {
            if action.is_user_gesture() {
                decision.ignore();
                if let Some(request) = action.request() {
                    if let Some(uri) = request.uri() {
                        if let Err(err) = open::that_detached(&uri) {
                            warn!("Failed to open {uri:?}: {err:?}");
                        }
                    }
                }
            }
        }
    }
}
