use tokio::sync::mpsc;
use tracing::info;
use wordbase::protocol::ShowPopupRequest;

pub fn run(_: mpsc::Receiver<ShowPopupRequest>) {
    info!("Running server compiled without popup support");
}
