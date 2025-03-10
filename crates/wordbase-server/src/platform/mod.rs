mod noop;
mod wayland;

use std::sync::Arc;

use anyhow::Result;
use wordbase::protocol::ShowPopupRequest;

pub trait Platform: Send + Sync + 'static {
    fn spawn_popup(&self, request: ShowPopupRequest) -> Result<()>;
}

pub fn default() -> Arc<dyn Platform> {
    Arc::new(noop::NoopPlatform)
    // Arc::new(wayland::WaylandPlatform::new())
}
