use anyhow::{Result, bail};
use futures::future::LocalBoxFuture;
use gtk4::prelude::GtkWindowExt;
use relm4::adw;

use super::WindowFilter;

pub struct Platform;

impl Platform {
    #[expect(clippy::unused_async, reason = "matches signature of other `fn new`s")]
    #[allow(dead_code, reason = "optional platform implementation")]
    pub async fn new() -> Result<Self> {
        Ok(Self)
    }
}

impl super::Platform for Platform {
    fn init_overlay(&self, window: &adw::Window) -> LocalBoxFuture<Result<()>> {
        window.present();
        Box::pin(async move { Ok(()) })
    }

    fn move_to_window(
        &self,
        _window: &adw::Window,
        _target: WindowFilter,
        _offset: (i32, i32),
    ) -> LocalBoxFuture<Result<()>> {
        Box::pin(async move { bail!("unsupported") })
    }
}
