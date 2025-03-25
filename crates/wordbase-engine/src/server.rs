use anyhow::Result;
use futures::never::Never;
use tokio::sync::mpsc;

pub(super) async fn run(send_popup_request: mpsc::Sender<()>) -> Result<Never> {
    loop {}
}
