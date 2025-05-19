use {
    super::{OverlayGuard, WindowFilter},
    anyhow::{Result, bail},
    futures::future::LocalBoxFuture,
    relm4::gtk,
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
    fn init_overlay(&self, _overlay: &gtk::Window) -> LocalBoxFuture<Result<OverlayGuard>> {
        Box::pin(async move { bail!("unsupported") })
    }

    fn init_popup(&self, _popup: &gtk::Window) -> LocalBoxFuture<Result<()>> {
        Box::pin(async move { bail!("unsupported") })
    }

    fn move_popup_to_window(
        &self,
        _popup: &gtk::Window,
        _target: WindowFilter,
        _offset_nw: (i32, i32),
        _offset_se: (i32, i32),
    ) -> LocalBoxFuture<Result<()>> {
        Box::pin(async move { bail!("unsupported") })
    }
}
