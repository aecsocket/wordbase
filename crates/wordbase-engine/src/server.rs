use anyhow::Result;
use futures::never::Never;
use tokio::sync::mpsc;

use crate::lookup::Lookups;

pub(super) async fn run(lookups: Lookups, send_popup_request: mpsc::Sender<()>) -> Result<Never> {
    loop {}
}
