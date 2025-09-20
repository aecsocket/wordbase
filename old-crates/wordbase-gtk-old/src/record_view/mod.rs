use glib::clone;
use maud::{Markup, PreEscaped, html};
use relm4::prelude::*;
use tracing::{debug, info};
use webkit::prelude::*;
use wordbase::{RecordKind, RecordLookup};
use wordbase_engine::EngineEvent;

use crate::{
    AppEvent, current_profile, current_profile_id, engine, forward_events, html,
    theme::DEFAULT_THEME,
};

#[derive(Debug)]
pub struct RecordView {
    records: Vec<RecordLookup>,
    query_override: Option<String>,
}

pub const SUPPORTED_RECORD_KINDS: &[RecordKind] = RecordKind::ALL;

#[derive(Debug)]
pub enum Msg {
    Render {
        records: Vec<RecordLookup>,
    },
    #[doc(hidden)]
    QueryOverride(String),
    #[doc(hidden)]
    Rerender,
    #[doc(hidden)]
    Requery,
}

#[derive(Debug)]
pub enum Response {
    HtmlRendered(Markup),
}

pub fn should_requery(event: &AppEvent) -> bool {
    match event {
        AppEvent::ProfileIdSet | AppEvent::Engine(EngineEvent::Dictionary(_)) => true,
        AppEvent::Engine(
            EngineEvent::FontFamilySet { profile_id }
            | EngineEvent::SortingDictionarySet { profile_id, .. },
        ) if *profile_id == current_profile_id() => true,
        _ => false,
    }
}

impl AsyncComponent for RecordView {
    type Init = ();
    type Input = Msg;
    type Output = Response;
    type CommandOutput = AppEvent;
    type Root = webkit::WebView;
    type Widgets = ();

    fn init_root() -> Self::Root {
        // if we don't do this, then the app will make some files in ~/.local/share:
        // ├── mediakeys
        // │   └── v1
        // │       └── salt
        // └── storage
        //     └── salt
        webkit::WebView::builder()
            .network_session(&webkit::NetworkSession::new_ephemeral())
            .build()
    }

    async fn init(
        (): Self::Init,
        ui: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        forward_events(&sender);
        ui.connect_context_menu(move |_, _, _| {
            // prevent opening context menu
            true
        });
        ui.connect_decide_policy(clone!(
            #[strong]
            sender,
            move |_, decision, _| {
                on_decide_policy(decision, &sender);
                true
            }
        ));

        adw::StyleManager::default().connect_accent_color_rgba_notify(clone!(
            #[strong]
            sender,
            move |_| sender.input(Msg::Rerender)
        ));

        AsyncComponentParts {
            model: Self {
                records: Vec::new(),
                query_override: None,
            },
            widgets: (),
        }
    }

    async fn update(
        &mut self,
        msg: Self::Input,
        sender: AsyncComponentSender<Self>,
        ui: &Self::Root,
    ) {
        match msg {
            Msg::Render { records } => {
                self.query_override = None;
                self.records = records;
                self.render(ui, &sender);
            }
            Msg::QueryOverride(query) => {
                self.query_override = Some(query);

                self.render(ui, &sender);
            }
            Msg::Requery => {
                // if let Some(query) = &self.query_override {
                //     engine().lookup(current_profile_id(), query, cursor, record_kinds)
                // }
            }
            Msg::Rerender => {
                self.render(ui, &sender);
            }
        }
    }

    async fn update_cmd_with_view(
        &mut self,
        (): &mut Self::Widgets,
        event: Self::CommandOutput,
        sender: AsyncComponentSender<Self>,
        ui: &Self::Root,
    ) {
        if should_requery(&event) {
            self.render(ui, &sender);
        }
    }
}

impl RecordView {
    fn render(&self, ui: &webkit::WebView, sender: &AsyncComponentSender<Self>) {
        let profile = current_profile();
        let settings = webkit::Settings::new();
        settings.set_enable_page_cache(false);
        settings.set_enable_smooth_scrolling(false);
        if let Some(family) = &profile.font_family {
            settings.set_default_font_family(family);
        }
        ui.set_settings(&settings);

        let dictionaries = engine().dictionaries();
        let records_html = html::render_records(
            &|id| dictionaries.get(&id).map(|dict| &**dict),
            &self.records,
        );

        // let custom_theme_name = self.settings.string(CUSTOM_THEME);
        // let custom_theme = CUSTOM_THEMES
        //     .read()
        //     .get(&ThemeName(Arc::from(custom_theme_name.to_string())))
        //     .map(|theme| theme.theme.clone());

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

            // style {
            //     (escape_style(
            //         custom_theme
            //             .as_ref()
            //             .map(|theme| theme.style.as_str())
            //             .unwrap_or_default(),
            //     ))
            // }

            .records {
                (records_html)
            }
        };
        ui.load_html(&full_html.0, None);

        _ = sender.output(Response::HtmlRendered(records_html));
    }
}

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

fn on_decide_policy(decision: &webkit::PolicyDecision, sender: &AsyncComponentSender<RecordView>) {
    let Some(decision) = decision.downcast_ref::<webkit::NavigationPolicyDecision>() else {
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
            sender.input(Msg::QueryOverride(query));
        }
        return;
    }

    info!("Opening {uri:?} in browser");
    gtk::UriLauncher::new(&uri).launch(None::<&gtk::Window>, None::<&gio::Cancellable>, |_| {});
}
