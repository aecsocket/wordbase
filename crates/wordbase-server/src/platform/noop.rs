use anyhow::{Result, bail};
use wordbase::protocol::ShowPopupRequest;

use super::Platform;

pub struct NoopPlatform;

impl Platform for NoopPlatform {
    fn spawn_popup(&self, _: ShowPopupRequest) -> Result<()> {
        bail!("unsupported")
    }
}
