extern crate gtk4 as gtk;
extern crate libadwaita as adw;
extern crate webkit6 as webkit;

use {
    super::Request,
    crate::{ServerEvent, lookup},
    anyhow::{Context, Result, bail},
    foldhash::{HashMap, HashMapExt},
    futures::never::Never,
    gtk4::{
        gdk,
        gio::{self, prelude::*},
        prelude::*,
    },
    std::{cell::LazyCell, sync::Arc},
    tokio::sync::broadcast,
    tracing::{error, info, warn},
    webkit6::prelude::{PolicyDecisionExt, WebViewExt},
    wordbase::{
        DictionaryId,
        protocol::{LookupRequest, NoRecords, PopupAnchor, ShowPopupRequest, ShowPopupResponse},
    },
};

const POPUP_APP_ID: &str = "com.github.aecsocket.WordbasePopup";
const BUS_NAME: &str = "com.github.aecsocket.WordbaseIntegration";
const OBJECT_PATH: &str = "/com/github/aecsocket/WordbaseIntegration";
const INTERFACE_NAME: &str = "com.github.aecsocket.WordbaseIntegration";
const SET_POPUP_POSITION: &str = "SetPopupPosition";

pub fn run(
    lookups: lookup::Client,
    recv_server_event: broadcast::Receiver<ServerEvent>,
    recv_request: broadcast::Receiver<Request>,
) -> Result<()> {
    info!("Using Wayland popup backend via GTK/Adwaita");
    glib::log_set_default_handler(glib::rust_log_handler);

    let app = adw::Application::builder()
        .application_id(POPUP_APP_ID)
        .build();
    // make the app not close when all its windows are closed
    let _hold_guard = app.hold();

    app.connect_startup(|_| {
        let provider = gtk::CssProvider::new();
        // provider.load_from_string(wordbase_gtk::STYLESHEET);

        gtk::style_context_add_provider_for_display(
            &gdk::Display::default().expect("failed to get display"),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    });
    app.connect_activate(move |app| {
        let state = State {
            lookups: lookups.clone(),
            recv_server_event: recv_server_event.resubscribe(),
            recv_request: recv_request.resubscribe(),
            app: app.clone().upcast(),
        };
        glib::spawn_future_local(async move {
            let Err(err) = backend(state).await;
            error!("GTK app backend closed: {err:?}");
        });
    });

    app.run();
    bail!("GTK application closed")
}

#[derive(Debug)]
struct State {
    lookups: lookup::Client,
    recv_server_event: broadcast::Receiver<ServerEvent>,
    recv_request: broadcast::Receiver<Request>,
    app: gtk::Application,
}

async fn backend(mut state: State) -> Result<Never> {
    let mut popup = None::<PopupInfo>;
    let mut dictionary_names = HashMap::<DictionaryId, Arc<str>>::new();
    loop {
        let request = tokio::select! {
            request = state.recv_request.recv() => request,
            Ok(ServerEvent::SyncDictionaries(new_dictionaries)) = state.recv_server_event.recv() => {
                dictionary_names = new_dictionaries
                    .into_iter()
                    .map(|state| (state.id, Arc::from(state.meta.name)))
                    .collect();
                continue;
            }
        };
        let request = match request {
            Ok(request) => request,
            Err(broadcast::error::RecvError::Closed) => bail!("popup request channel closed"),
            Err(broadcast::error::RecvError::Lagged(n)) => {
                warn!("Popup thread lagged by {n} requests");
                continue;
            }
        };

        match request {
            Request::Show {
                request,
                send_response,
            } => {
                let result = show(&state, &mut popup, &dictionary_names, request).await;
                _ = send_response.send(result).await;
            }
            Request::Hide { send_response } => {
                if let Some(popup) = &popup {
                    popup.window.set_visible(false);
                }
                _ = send_response.send(()).await;
            }
        }
    }
}

async fn show(
    state: &State,
    popup: &mut Option<PopupInfo>,
    dictionary_names: &HashMap<DictionaryId, Arc<str>>,
    request: ShowPopupRequest,
) -> Result<Result<ShowPopupResponse, NoRecords>> {
    let records = state
        .lookups
        .lookup(LookupRequest {
            text: request.text,
            record_kinds: wordbase_html::SUPPORTED_RECORD_KINDS.to_vec(),
        })
        .await
        .context("failed to perform lookup")?;
    if records.is_empty() {
        return Ok(Err(NoRecords));
    }

    let chars_scanned = records
        .iter()
        .map(|record| record.lemma.chars().count())
        .max()
        .and_then(|c| u64::try_from(c).ok())
        .unwrap_or_default();

    let popup = popup.get_or_insert_with(|| create_popup(&state.app));
    let unknown_source = Arc::<str>::from("?");
    let dictionary_html = wordbase_html::to_html(
        |source| {
            dictionary_names
                .get(&source)
                .unwrap_or(&unknown_source)
                .clone()
        },
        records,
    );
    let html = format!("<style>{STYLE}</style>{}", dictionary_html.0);
    popup.web_view.load_html(&html, None);
    popup.window.set_visible(true);

    let (request_x, request_y) = request.origin;
    let origin_x = match request.anchor {
        PopupAnchor::TopLeft | PopupAnchor::MiddleLeft | PopupAnchor::BottomLeft => request_x,
        PopupAnchor::TopMiddle | PopupAnchor::BottomMiddle => {
            request_x.saturating_sub(popup.window.width() / 2)
        }
        PopupAnchor::TopRight | PopupAnchor::MiddleRight | PopupAnchor::BottomRight => {
            request_x.saturating_sub(popup.window.width())
        }
    };
    let origin_y = match request.anchor {
        PopupAnchor::TopLeft | PopupAnchor::TopMiddle | PopupAnchor::TopRight => request_y,
        PopupAnchor::MiddleLeft | PopupAnchor::MiddleRight => {
            request_y.saturating_sub(popup.window.height() / 2)
        }
        PopupAnchor::BottomLeft | PopupAnchor::BottomMiddle | PopupAnchor::BottomRight => {
            request_y.saturating_sub(popup.window.height())
        }
    };

    let params = glib::Variant::tuple_from_iter([
        request.target_id.unwrap_or_default().into(),
        request.target_pid.unwrap_or_default().into(),
        request.target_title.unwrap_or_default().into(),
        request.target_wm_class.unwrap_or_default().into(),
        glib::Variant::from(origin_x),
        glib::Variant::from(origin_y),
    ]);

    state
        .app
        .dbus_connection()
        .context("no dbus connection")?
        .call_future(
            Some(BUS_NAME),
            OBJECT_PATH,
            INTERFACE_NAME,
            SET_POPUP_POSITION,
            Some(&params),
            None,
            gio::DBusCallFlags::NONE,
            -1,
        )
        .await
        .context("failed to send popup position request")?;

    Ok(Ok(ShowPopupResponse { chars_scanned }))
}

struct PopupInfo {
    window: gtk::Window,
    web_view: webkit::WebView,
}

fn create_popup(app: &gtk::Application) -> PopupInfo {
    thread_local! {
        static SETTINGS: LazyCell<webkit::Settings> = LazyCell::new(|| {
            webkit::Settings::builder()
                .enable_page_cache(false)
                .enable_smooth_scrolling(false)
                .build()
        });
    }

    let web_view = SETTINGS.with(|settings| webkit::WebView::builder().settings(settings).build());
    web_view
        .context()
        .expect("should have web context")
        .set_cache_model(webkit::CacheModel::DocumentViewer);

    // don't allow opening the context menu
    web_view.connect_context_menu(|_, _, _| true);

    // when attempting to navigate to a URL, open in the user's browser instead
    web_view.connect_decide_policy(|_, decision, decision_type| {
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

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .default_width(600)
        .default_height(300)
        .hide_on_close(true)
        .content(&web_view)
        .build();
    window.present();

    let controller = gtk::EventControllerMotion::new();
    window.add_controller(controller.clone());
    controller.connect_leave({
        let window = window.clone();
        move |_| {
            window.set_visible(false);
        }
    });

    PopupInfo {
        window: window.upcast(),
        web_view,
    }
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
        if let Err(err) = open::that_detached(uri) {
            warn!("Failed to open link to {uri:?}: {err:?}");
        }
    }
}

const STYLE: &str = r##"
/* Adwaita-like styling */
body {
    font-family: "Inter", sans-serif;
    margin: 0;
    padding: 20px;
    background-color: #fafafa;
    color: #2e3436;
}

.terms {
    display: flex;
    flex-direction: column;
    gap: 20px;
}

.header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background-color: #ffffff;
    border-radius: 8px;
    padding: 20px;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

.term {
    font-size: 2rem;
    color: #1c71d8; /* Adwaita accent blue */
}

.term ruby {
    font-size: 1.5rem;
    color: #2e3436;
}

.pitches {
    font-size: 0.9rem;
    color: #777777;
    margin-top: 5px;
}

.meta {
    display: flex;
    align-items: center;
}

.frequencies {
    display: flex;
    flex-wrap: wrap; /* Allow frequency groups to wrap */
    gap: 10px;
}

.frequencies .group {
    display: flex;
    align-items: center;
    background-color: #e0e0e0;
    border-radius: 20px;
    padding: 5px 10px;
    font-size: 0.9rem;
    color: #2e3436;
}

.frequencies .source {
    margin-right: 5px;
    font-weight: bold;
    color: #1c71d8; /* Adwaita accent blue */
}

.frequencies .values {
    display: flex;
    gap: 5px;
}

.frequencies .value {
    font-weight: bold;
    color: #2e3436;
}

.glossary-page {
    display: flex;
    flex-direction: column;
    gap: 20px;
}

/* Card styling for each dictionary's glossary set */
.glossary-page .group {
    background-color: #ffffff;
    border-radius: 8px;
    padding: 20px;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

.glossary-page .source {
    font-size: 0.9rem;
    color: #777777;
    margin-bottom: 10px;
    display: block;
}

.glossary-page .rows {
    display: flex;
    flex-direction: column;
    gap: 10px;
}

.glossary-page .row {
    background-color: #f9f9f9;
    border-radius: 8px;
    padding: 10px;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.05);
}

.content[data-count="1"] {
    padding-left: 0;
    list-style-type: none;
}

.glossary-page .tags {
    display: flex;
    flex-wrap: wrap; /* Allow tags to wrap */
    gap: 5px;
    margin-bottom: 10px;
}

.glossary-page .tags div {
    background-color: #e0e0e0;
    border-radius: 20px;
    padding: 5px 10px;
    font-size: 0.8rem;
    color: #2e3436;
}

.glossary-page ul {
    margin: 0;
    padding-left: 20px;
    font-size: 0.9rem;
    color: #2e3436;
}

.glossary-page ul li {
    margin-bottom: 5px;
}

/* Add middle dot between frequency values */
.frequencies .values .value:not(:last-child)::after {
    content: "·";
    margin: 0 5px;
    color: #2e3436;
}

/* Remove card styling from the term meta */
.header {
    background-color: transparent;
    box-shadow: none;
    border-radius: 0;
    padding: 20px 0; /* Adjust padding as needed */
}

.meta {
    background-color: transparent;
    box-shadow: none;
    border-radius: 0;
}

/* Ensure only glossary cards have card styling */
.glossary-page .group {
    background-color: #ffffff;
    border-radius: 8px;
    padding: 20px;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}
"##;
