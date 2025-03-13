mod ui;

use std::sync::Arc;

use adw::{gtk, prelude::*};
use anyhow::{Context, Result};
use derive_more::Display;
use foldhash::{HashMap, HashMapExt};
use futures::{FutureExt, StreamExt, never::Never, stream::FuturesUnordered};
use tokio::sync::{Notify, broadcast, mpsc};
use tracing::info;
use wordbase::hook::HookSentence;
use wordbase_server::{CHANNEL_BUF_CAP, Event};

use crate::{Config, gettext, platform};

#[derive(Debug, Clone)]
pub struct Client {
    send_request: mpsc::Sender<Request>,
}

impl Client {
    pub fn new(
        config: Arc<Config>,
        app: adw::Application,
        platform: Arc<dyn platform::Client>,
        recv_event: broadcast::Receiver<Event>,
    ) -> (Self, impl Future<Output = Result<Never>>) {
        let (send_request, recv_request) = mpsc::channel(CHANNEL_BUF_CAP);
        let task = run(config, app, platform, recv_event, recv_request);
        (Self { send_request }, task)
    }

    pub async fn set_text_size(&self, text_size: TextSize) -> Result<()> {
        self.send_request
            .send(Request::SetTextSize(text_size))
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
    SetTextSize(TextSize),
}

pub async fn run(
    config: Arc<Config>,
    app: adw::Application,
    platform: Arc<dyn platform::Client>,
    mut recv_event: broadcast::Receiver<Event>,
    mut recv_request: mpsc::Receiver<Request>,
) -> Result<Never> {
    let mut processes = ProcessMap::new();

    loop {
        let mut destroyed = processes
            .iter()
            .map(|(process_path, process)| process.destroyed.notified().map(move |_| process_path))
            .collect::<FuturesUnordered<_>>();

        tokio::select! {
            event = recv_event.recv() => {
                drop(destroyed);
                let event = event.context("event channel closed")?;
                match event {
                    Event::HookSentence(hook_sentence) => {
                        on_hook_sentence(&config, &app, &*platform, &mut processes, hook_sentence);
                    }
                    Event::SyncDictionaries(_) => {}
                }
            }
            Some(process_path) = destroyed.next() => {
                info!("Removing overlay for {process_path:?}");
                let process_path = process_path.clone();
                drop(destroyed);
                processes.remove(&process_path);
            }
            Some(request) = recv_request.recv() => {
                drop(destroyed);
                match request {
                    Request::SetTextSize(text_size) => {
                        set_text_size(&processes, text_size);
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
struct ProcessOverlay {
    window: adw::ApplicationWindow,
    sentence: gtk::Label,
    destroyed: Arc<Notify>,
}

type ProcessMap = HashMap<String, ProcessOverlay>;

fn on_hook_sentence(
    config: &Config,
    app: &adw::Application,
    platform: &dyn platform::Client,
    processes: &mut ProcessMap,
    hook_sentence: HookSentence,
) {
    let overlay = processes
        .entry(hook_sentence.process_path)
        .or_insert_with_key(|process_path| {
            info!("Creating overlay for {process_path:?}");

            let window = adw::ApplicationWindow::builder()
                .application(app)
                .title(format!("{} - {process_path}", gettext("Wordbase Overlay")))
                .build();
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
                .set_css_classes(&[&config.overlay_text_size.to_string()]);

            platform.stick_to_focused_window(&window);

            ProcessOverlay {
                window,
                sentence: content.sentence(),
                destroyed,
            }
        });

    overlay.sentence.set_text(&hook_sentence.sentence);
}

fn set_text_size(processes: &ProcessMap, text_size: TextSize) {
    for (_, overlay) in processes {
        overlay.sentence.set_css_classes(&[&text_size.to_string()]);
    }
}
