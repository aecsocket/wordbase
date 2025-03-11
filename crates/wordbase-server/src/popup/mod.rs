cfg_if! {
    if #[cfg(all(
        feature = "popup",
        unix,
        not(target_vendor = "apple"),
        not(target_os = "emscripten"),
    ))] {
        mod wayland;
        use wayland as platform;
    } else {
        mod noop;
        use noop as platform;
    }
}

use {
    crate::{CHANNEL_BUF_CAP, ServerEvent, lookup},
    anyhow::{Context, Result},
    cfg_if::cfg_if,
    tokio::sync::{broadcast, mpsc},
    wordbase::protocol::{NoRecords, ShowPopupRequest, ShowPopupResponse},
};

#[derive(Debug, Clone)]
pub struct Client {
    send_request: broadcast::Sender<Request>,
}

impl Client {
    pub fn new(
        lookups: lookup::Client,
        recv_server_event: broadcast::Receiver<ServerEvent>,
    ) -> Self {
        let (send_request, recv_request) = broadcast::channel(CHANNEL_BUF_CAP);
        std::thread::spawn(move || platform::run(lookups, recv_server_event, recv_request));
        Self { send_request }
    }

    pub async fn show(
        &self,
        request: ShowPopupRequest,
    ) -> Result<Result<ShowPopupResponse, NoRecords>> {
        let (send_response, mut recv_response) = mpsc::channel(CHANNEL_BUF_CAP);
        self.send_request.send(Request::Show {
            request,
            send_response,
        })?;
        let response = recv_response
            .recv()
            .await
            .context("no popup backend running")?;
        response
    }

    pub async fn hide(&self) -> Result<()> {
        let (send_response, mut recv_response) = mpsc::channel(CHANNEL_BUF_CAP);
        self.send_request.send(Request::Hide { send_response })?;
        recv_response
            .recv()
            .await
            .context("no popup backend running")?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
enum Request {
    Show {
        request: ShowPopupRequest,
        send_response: mpsc::Sender<Result<Result<ShowPopupResponse, NoRecords>>>,
    },
    Hide {
        send_response: mpsc::Sender<()>,
    },
}
