use {tokio::sync::mpsc, tracing::info, wordbase::protocol::ShowPopupRequest};

pub fn run(
    _lookups: lookup::Client,
    _recv_server_event: broadcast::Receiver<ServerEvent>,
    _recv_request: broadcast::Receiver<Request>,
) -> Result<()> {
    info!("Running server compiled without popup support");
}
