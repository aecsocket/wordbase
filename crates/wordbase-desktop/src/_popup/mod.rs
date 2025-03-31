use std::sync::Arc;

use anyhow::{Context, Result};
use futures::never::Never;
use tokio::sync::{mpsc, oneshot};
use wordbase::protocol::{ShowPopupError, ShowPopupRequest};
use wordbase_server::CHANNEL_BUF_CAP;

use crate::{gettext, platform, Config};

#[derive(Debug, Clone)]
pub struct Client {
    send_request: mpsc::Sender<Request>,
}

pub struct State {
    pub config: Arc<Config>,
    pub platform: Arc<dyn platform::Platform>,
    pub app: adw::Application,
}

impl Client {
    pub fn new(state: State) -> (Self, impl Future<Output = Result<Never>>) {
        let (send_request, recv_request) = mpsc::channel(CHANNEL_BUF_CAP);
        (Self { send_request }, run(state, recv_request))
    }

    pub async fn show(&self, request: ShowPopupRequest) -> Result<Result<(), ShowPopupError>> {
        let (send_response, recv_response) = oneshot::channel();
        self.send_request
            .send(Request::Show {
                request,
                send_response,
            })
            .await?;
        recv_response.await?
    }
}

#[derive(Debug)]
enum Request {
    Show {
        request: ShowPopupRequest,
        send_response: oneshot::Sender<Result<Result<(), ShowPopupError>>>,
    },
    Hide,
}

#[derive(Debug)]
struct PopupState {
    window: adw::ApplicationWindow,
    web_view: webkit::WebView,
}

async fn run(mut state: State, mut recv_request: mpsc::Receiver<Request>) -> Result<Never> {
    let mut popup = None::<PopupState>;
    loop {
        let request = recv_request
            .recv()
            .await
            .context("popup request channel closed")?;

        match request {
            Request::Show {
                request,
                send_response,
            } => {
                drop(popup.take());
            }
        }
    }
}

async fn on_show(
    state: &State,
    popup: &mut Option<PopupState>,
    request: ShowPopupRequest,
) -> Result<Result<(), ShowPopupError>> {
    let popup = match popup {
        Some(popup) => popup,
        None => {
            let window = adw::ApplicationWindow::builder()
                .application(&state.app)
                .title(gettext("Wordbase Dictionary"))
                .build();
            popup.insert(PopupState { window: (), web_view: () })
        }
    }

    state.platform.move_to_window(&window, request.target_window, request.origin)

}
