use anyhow::{Result, bail};
use wordbase::protocol::ShowPopupRequest;

pub trait EnginePlatform {
    fn show_popup(&self, request: ShowPopupRequest) -> Result<()>;
}

pub struct NoopPlatform;

impl EnginePlatform for NoopPlatform {
    fn show_popup(&self, _: ShowPopupRequest) -> Result<()> {
        bail!("unsupported")
    }
}
