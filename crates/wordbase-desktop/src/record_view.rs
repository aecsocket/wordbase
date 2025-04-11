use {
    crate::{AppEvent, forward_events, theme::DEFAULT_THEME},
    maud::html,
    relm4::{
        adw::{gdk, gio, prelude::*},
        prelude::*,
    },
    tracing::{debug, info},
    webkit6::prelude::*,
    wordbase::{RecordKind, RecordLookup},
    wordbase_engine::{Engine, html},
};

#[derive(Debug)]
pub struct Model {
    engine: Engine,
    records: Vec<RecordLookup>,
}

pub const SUPPORTED_RECORD_KINDS: &[RecordKind] = RecordKind::ALL;

#[derive(Debug)]
pub struct Msg(pub Vec<RecordLookup>);

#[derive(Debug)]
pub enum Response {
    Query(String),
}

#[relm4::component(pub, async)]
impl AsyncComponent for Model {
    type Init = Engine;
    type Input = Msg;
    type Output = Response;
    type CommandOutput = AppEvent;

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

    async fn init(
        engine: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        forward_events(&sender);
        let model = Self {
            engine,
            records: Vec::new(),
        };
        let widgets = view_output!();
        AsyncComponentParts { model, widgets }
    }

    async fn update_cmd_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::CommandOutput,
        sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
    }

    async fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        records: Self::Input,
        sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        self.records = records.0;
        let profile = self.engine.profiles().current.clone();

        root.set_settings(
            &webkit6::Settings::builder()
                .default_font_family(profile.config.font_family.as_deref().unwrap_or_default())
                .enable_page_cache(false)
                .build(),
        );

        let records_html = html::render_records(&self.engine.dictionaries().by_id, &self.records);
        let full_html = html! {
            style {
                (DEFAULT_THEME.style)
            }

            // TODO custom theme
            // style {
            //     (self.custom_theme.as_ref().map(|theme| theme.style.as_str()).unwrap_or_default())
            // }

            .records {
                (records_html)
            }
        };
        root.load_html(&full_html.0, None);
    }
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
