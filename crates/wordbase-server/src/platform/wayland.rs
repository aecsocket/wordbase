use anyhow::Result;
use wordbase::protocol::ShowPopupRequest;

use super::Platform;

#[derive(Debug)]
pub struct WaylandPlatform {}

impl WaylandPlatform {
    pub fn new() -> Self {
        todo!()
    }
}

impl Platform for WaylandPlatform {
    fn spawn_popup(&self, request: ShowPopupRequest) -> Result<()> {
        todo!()
    }
}
