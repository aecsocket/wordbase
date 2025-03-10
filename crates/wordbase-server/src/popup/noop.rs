use anyhow::{Result, bail};
use wordbase::protocol::ShowPopupRequest;

use super::Popups;

pub struct NoopPopups;

impl NoopPopups {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Popups for NoopPopups {
    fn show(&self, _: ShowPopupRequest) -> Result<()> {
        bail!("unsupported")
    }
}
