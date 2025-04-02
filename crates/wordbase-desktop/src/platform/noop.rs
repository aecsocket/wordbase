use anyhow::{Result, bail};
use futures::future::BoxFuture;
use relm4::adw;

use super::WindowFilter;

pub struct Platform;

impl Platform {
    pub async fn new() -> Result<Self> {
        Ok(Self)
    }
}

impl super::Platform for Platform {
    fn affix_to_focused_window(&self, _window: &adw::Window) -> BoxFuture<Result<()>> {
        Box::pin(async move { bail!("unsupported") })
    }

    fn move_to_window(
        &self,
        _window: &adw::Window,
        _target: WindowFilter,
        _offset: (i32, i32),
    ) -> BoxFuture<Result<()>> {
        Box::pin(async move { bail!("unsupported") })
    }
}
