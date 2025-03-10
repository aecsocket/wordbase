use std::{cell::LazyCell, fmt::Write as _, sync::LazyLock, time::Duration};

use gtk::{gdk, gio, glib, prelude::*};
use tracing::{info, warn};
use webkit::prelude::*;
use wordbase::glossary::{Html, HtmlFallback, PlainText, PlainTextFallback};

use crate::{AddToTermInfo, GlossaryInfo, RecordContext};

impl AddToTermInfo for PlainText {
    fn add_to_term_info(self, cx: RecordContext) {
        cx.glossary_group.push(GlossaryInfo {
            content: vec![plain_text(&self.0).upcast()],
            ..Default::default()
        });
    }
}

impl AddToTermInfo for PlainTextFallback {
    fn add_to_term_info(self, cx: RecordContext) {
        cx.glossary_group.push(GlossaryInfo {
            content: vec![plain_text(&self.0)],
            ..Default::default()
        });
    }
}

impl AddToTermInfo for Html {
    fn add_to_term_info(self, cx: RecordContext) {
        cx.glossary_group.push(GlossaryInfo {
            content: vec![html(|s| {
                _ = write!(s, "{}", self.0);
            })],
            ..Default::default()
        });
    }
}

impl AddToTermInfo for HtmlFallback {
    fn add_to_term_info(self, cx: RecordContext) {
        cx.glossary_group.push(GlossaryInfo {
            content: vec![html(|s| {
                _ = write!(s, "{}", self.0);
            })],
            ..Default::default()
        });
    }
}

pub fn plain_text(text: &str) -> gtk::Widget {
    gtk::Label::builder()
        .label(text)
        .selectable(true)
        .wrap(true)
        .build()
        .upcast()
}

pub fn html(write_html: impl FnOnce(&mut String)) -> gtk::Widget {
    static GLOSSARY_HTML: LazyLock<String> = LazyLock::new(|| {
        let css = include_str!("ui/glossary.css");
        format!("<style>{css}</style>")
    });

    thread_local! {
        static SETTINGS: LazyCell<webkit::Settings> = LazyCell::new(|| {
            webkit::Settings::builder()
                .allow_file_access_from_file_urls(false)
                .enable_back_forward_navigation_gestures(false)
                .enable_caret_browsing(false)
                .enable_developer_extras(false)
                .enable_encrypted_media(false)
                .enable_fullscreen(false)
                .enable_html5_database(false)
                .enable_html5_local_storage(false)
                .enable_javascript(false)
                .enable_javascript_markup(false)
                .enable_media(false)
                .enable_page_cache(false)
                .enable_webaudio(false)
                .enable_webgl(false)
                .enable_webrtc(false)
                .build()
        });
    }

    // why are we wrapping the webview in a container?
    // it's a surprise tool for later ;)
    let container = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        // if the WebView is allocated with either a width or height of 0,
        // we will fail to allocate a GBM buffer, and nothing will render
        .width_request(1)
        .height_request(1)
        .build();

    let view = SETTINGS.with(|settings| {
        webkit::WebView::builder()
            .width_request(1)
            .height_request(1)
            .settings(settings)
            .visible(false)
            .build()
    });
    container.append(&view);

    view.set_background_color(&gdk::RGBA::new(0.0, 0.0, 0.0, 0.0));

    let mut html = GLOSSARY_HTML.to_string();
    write_html(&mut html);
    view.load_html(&html, None);

    // don't allow opening the context menu
    view.connect_context_menu(|_, _, _| true);

    // when attempting to navigate to a URL, open in the user's browser instead
    view.connect_decide_policy(|_, decision, decision_type| {
        if decision_type != webkit::PolicyDecisionType::NavigationAction {
            return false;
        }
        let Some(decision) = decision.downcast_ref::<webkit::NavigationPolicyDecision>() else {
            return false;
        };
        let Some(mut nav_action) = decision.navigation_action() else {
            return false;
        };
        if !nav_action.is_user_gesture() {
            return false;
        }

        if let Some(request) = nav_action.request() {
            if let Some(uri) = request.uri() {
                open_uri(&uri);
            }
        }

        decision.ignore();
        true // inhibit request
    });

    // resize the view height to the content *when the webpage loads*
    view.connect_load_changed(move |view, _| {
        view.set_visible(true);
        resize_view(view.clone());
    });

    // resize the view height to the content *when the container changes size*
    // we use a DrawingArea to detect when the container size changes,
    // by hooking into `draw_func`. it's pretty stupid, I know.
    let height_change_proxy = gtk::DrawingArea::builder()
        .hexpand(true)
        .vexpand(false)
        .build();
    container.append(&height_change_proxy);
    height_change_proxy.set_draw_func(move |_, _, _, _| {
        resize_view(view.clone());
    });

    container.upcast()
}

fn resize_view(view: webkit::WebView) {
    glib::timeout_add_local_once(Duration::from_millis(100), move || {
        view.evaluate_javascript(
            // get the natural height of the content
            // `document.body.scrollHeight` and friends will stay tall
            // even if the natural height is reduced, so we need to do it like this
            "
[...document.body.children].reduce(
    (h, el) => Math.max(h, el.getBoundingClientRect().bottom),
    0
)",
            None,
            None,
            None::<&gio::Cancellable>,
            {
                let view = view.clone();
                move |result| {
                    if let Ok(value) = result {
                        let height = value.to_int32();
                        view.set_height_request(height.max(1));
                    };
                }
            },
        );
    });
}

fn open_uri(uri: &str) {
    if let Some(uri) = uri.strip_prefix('?') {
        if let Some((_, query)) =
            form_urlencoded::parse(uri.as_bytes()).find(|(key, _)| key == "query")
        {
            info!("Looking up {query:?}");
            warn!("TODO: unimplemented");
        } else {
            warn!("Attempted to open {uri:?} which does not contain `query`");
        }
    } else {
        info!("Opening {uri:?}");
        if let Err(err) = open::that(uri) {
            warn!("Failed to open link to {uri:?}: {err:?}");
        }
    }
}
