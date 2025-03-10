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

use anyhow::{Context, Result};
use cfg_if::cfg_if;
use tokio::sync::{broadcast, mpsc};
use wordbase::protocol::{NoRecords, ShowPopupRequest, ShowPopupResponse};

use crate::{CHANNEL_BUF_CAP, ServerEvent, lookup};

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
        self.send_request.send(Request {
            request,
            send_response,
        })?;
        let response = recv_response
            .recv()
            .await
            .context("no popup backend running")?;
        response
    }
}

#[derive(Debug, Clone)]
struct Request {
    request: ShowPopupRequest,
    send_response: mpsc::Sender<Result<Result<ShowPopupResponse, NoRecords>>>,
}
