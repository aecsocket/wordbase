mod noop;
mod wayland;

// use noop as default;
use {
    anyhow::Result,
    futures::{future::LocalBoxFuture, stream::BoxStream},
    relm4::adw,
    std::fmt::Debug,
    wayland as default,
    wordbase::WindowFilter,
};

pub trait Platform: Debug {
    fn init_overlay(&self, overlay: &adw::Window) -> LocalBoxFuture<Result<OverlayId>>;

    fn init_popup(&self, popup: &adw::Window) -> LocalBoxFuture<Result<()>>;

    fn move_popup_to_window(
        &self,
        popup: &adw::Window,
        to: WindowFilter,
        offset: (i32, i32),
    ) -> LocalBoxFuture<Result<()>>;

    fn overlays_closed(&self) -> LocalBoxFuture<Result<BoxStream<Result<OverlayId>>>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OverlayId(pub u64);

pub async fn default() -> Result<Box<dyn Platform>> {
    let platform = default::Platform::new().await?;
    Ok(Box::new(platform))
}
