mod noop;
mod wayland;

// use noop as default;
use {
    anyhow::Result,
    futures::future::LocalBoxFuture,
    relm4::gtk,
    std::{any::Any, fmt::Debug},
    wayland as default,
    wordbase::WindowFilter,
};

pub trait Platform: Debug {
    fn init_overlay(&self, overlay: &gtk::Window) -> LocalBoxFuture<Result<OverlayGuard>>;

    fn init_popup(&self, popup: &gtk::Window) -> LocalBoxFuture<Result<()>>;

    fn move_popup_to_window(
        &self,
        popup: &gtk::Window,
        to: WindowFilter,
        offset_nw: (i32, i32),
        offset_se: (i32, i32),
    ) -> LocalBoxFuture<Result<()>>;
}

pub type OverlayGuard = Box<dyn Any>;

pub async fn default() -> Result<Box<dyn Platform>> {
    let platform = default::Platform::new().await?;
    Ok(Box::new(platform))
}
