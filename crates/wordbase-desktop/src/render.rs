use {
    crate::theme::Theme,
    foldhash::{HashMap, HashMapExt},
    maud::html,
    relm4::{
        adw::{gdk, prelude::*},
        prelude::*,
    },
    std::sync::Arc,
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
