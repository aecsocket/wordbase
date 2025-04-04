use {
    super::WindowFilter,
    anyhow::{Result, bail},
    futures::future::LocalBoxFuture,
    relm4::adw,
};

#[derive(Debug)]
pub struct Platform;

impl Platform {
    #[expect(clippy::unused_async, reason = "matches signature of other `fn new`s")]
    #[allow(dead_code, reason = "optional platform implementation")]
    pub async fn new() -> Result<Self> {
        Ok(Self)
    }
}

impl super::Platform for Platform {
    fn init_overlay(&self, _overlay: &adw::Window) -> LocalBoxFuture<Result<()>> {
        Box::pin(async move { bail!("unsupported") })
    }

    fn init_popup(&self, _popup: &adw::Window) -> LocalBoxFuture<Result<()>> {
        Box::pin(async move { bail!("unsupported") })
    }

    fn move_popup_to_window(
        &self,
        _popup: &adw::Window,
        _target: WindowFilter,
        _offset: (i32, i32),
    ) -> LocalBoxFuture<Result<()>> {
        Box::pin(async move { bail!("unsupported") })
    }
}
