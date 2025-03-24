#![doc = include_str!("../README.md")]
#![allow(missing_docs, clippy::missing_errors_doc)]

mod db;
pub mod lookup;

use anyhow::Result;
use futures::{channel::mpsc, never::Never};
use wordbase::protocol::ShowPopupRequest;

#[derive(Debug)]
#[non_exhaustive]
pub struct Engine {
    pub recv_popup_request: mpsc::Receiver<ShowPopupRequest>,
}

const CHANNEL_BUF_CAP: usize = 4;

pub fn run() -> (Engine, impl Future<Output = Result<Never>>) {
    let (send_popup_request, recv_popup_request) = mpsc::channel(CHANNEL_BUF_CAP);
    (Engine { recv_popup_request }, run_task(send_popup_request))
}

async fn run_task(send_popup_request: mpsc::Sender<ShowPopupRequest>) -> Result<Never> {
    loop {}
}
