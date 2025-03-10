use anyhow::Result;
use wordbase::protocol::ShowPopupRequest;

use super::Popups;

#[derive(Debug)]
pub struct WaylandPlatform {}

impl WaylandPlatform {
    pub fn new() -> Self {
        todo!()
    }
}

impl Popups for WaylandPlatform {
    fn show(&self, request: ShowPopupRequest) -> Result<()> {
        todo!()
    }
}
