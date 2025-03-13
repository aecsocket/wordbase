mod ui;

use std::{collections::hash_map::Entry, sync::Arc};

use adw::{gtk, prelude::*};
use anyhow::{Context, Result};
use derive_more::Display;
use foldhash::{HashMap, HashMapExt};
use futures::{FutureExt, StreamExt, never::Never, stream::FuturesUnordered};
use tokio::sync::{Notify, broadcast, mpsc};
use tracing::{info, warn};
use wordbase::hook::HookSentence;
use wordbase_server::{CHANNEL_BUF_CAP, Event};

use crate::{Config, gettext, platform};

#[derive(Debug, Clone)]
pub struct Client {
    send_request: mpsc::Sender<Request>,
}

pub struct State {
    pub config: Arc<Config>,
    pub platform: Arc<dyn platform::Platform>,
    pub app: adw::Application,
    pub recv_event: broadcast::Receiver<Event>,
}

impl Client {
    pub fn new(state: State) -> (Self, impl Future<Output = Result<Never>>) {
        let (send_request, recv_request) = mpsc::channel(CHANNEL_BUF_CAP);
        let task = run(state, recv_request);
        (Self { send_request }, task)
    }

    pub async fn set_text_size(&self, text_size: TextSize) -> Result<()> {
        self.send_request
            .send(Request::SetTextSize { text_size })
            .await?;
        Ok(())
    }
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextSize {
    #[display("title-1")]
    Title1,
    #[display("title-2")]
    Title2,
    #[display("title-3")]
    Title3,
    #[display("title-4")]
    Title4,
    #[display("body")]
    Body,
}

#[derive(Debug)]
enum Request {
    SetTextSize { text_size: TextSize },
}

#[derive(Debug)]
struct OverlayState {
    window: adw::ApplicationWindow,
    sentence: gtk::Label,
    destroyed: Arc<Notify>,
}

type OverlayMap = HashMap<String, OverlayState>;

pub async fn run(mut state: State, mut recv_request: mpsc::Receiver<Request>) -> Result<Never> {
    let mut overlays = OverlayMap::new();

    loop {
        let mut destroyed = overlays
            .iter()
            .map(|(process_path, process)| process.destroyed.notified().map(move |_| process_path))
            .collect::<FuturesUnordered<_>>();

        tokio::select! {
            event = state.recv_event.recv() => {
                drop(destroyed);
                let Event::HookSentence(hook_sentence) = event.context("event channel closed")? else {
                    continue;
                };

                let process_path = hook_sentence.process_path.clone();
                if let Err(err) = on_hook_sentence(&state, &mut overlays, hook_sentence).await {
                    warn!("Failed to update overlay for {process_path:?}: {err:?}");
                }
            }
            Some(process_path) = destroyed.next() => {
                info!("Removing overlay for {process_path:?}");
                let process_path = process_path.clone();
                drop(destroyed);
                overlays.remove(&process_path);
            }
            request = recv_request.recv() => {
                let request = request.context("overlay request channel closed")?;
                drop(destroyed);
                match request {
                    Request::SetTextSize { text_size } => {
                        set_text_size(&overlays, text_size);
                    }
                }
            }
        }
    }
}

async fn on_hook_sentence(
    state: &State,
    overlays: &mut OverlayMap,
    hook_sentence: HookSentence,
) -> Result<()> {
    let overlay = match overlays.entry(hook_sentence.process_path) {
        Entry::Occupied(entry) => entry.into_mut(),
        Entry::Vacant(entry) => {
            let process_path = entry.key();
            info!("Creating overlay for {process_path:?}");

            let window = adw::ApplicationWindow::builder()
                .application(&state.app)
                .title(format!("{} - {process_path}", gettext("Wordbase Overlay")))
                .build();
            state
                .platform
                .affix_to_focused_window(&window)
                .await
                .context("failed to affix overlay to focused window")?;
            window.present();

            let destroyed = Arc::new(Notify::new());
            window.connect_destroy({
                let destroyed = destroyed.clone();
                move |_| {
                    destroyed.notify_waiters();
                }
            });
            window.connect_close_request({
                let destroyed = destroyed.clone();
                move |_| {
                    destroyed.notify_waiters();
                    glib::Propagation::Proceed
                }
            });

            let content = ui::Overlay::new();
            window.set_content(Some(&content));
            content
                .sentence()
                .set_css_classes(&[&state.config.overlay_text_size.to_string()]);

            entry.insert(OverlayState {
                window,
                sentence: content.sentence(),
                destroyed,
            })
        }
    };

    overlay.sentence.set_text(&hook_sentence.sentence);
    Ok(())
}

fn set_text_size(processes: &OverlayMap, text_size: TextSize) {
    for (_, overlay) in processes {
        overlay.sentence.set_css_classes(&[&text_size.to_string()]);
    }
}
