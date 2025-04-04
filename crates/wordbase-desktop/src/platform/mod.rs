mod noop;
mod wayland;

// use noop as default;
use {
    anyhow::Result, futures::future::LocalBoxFuture, relm4::adw, std::fmt::Debug,
    wayland as default, wordbase::WindowFilter,
};

pub trait Platform: Debug {
    fn init_overlay(&self, overlay: &adw::Window) -> LocalBoxFuture<Result<()>>;

    fn init_popup(&self, popup: &adw::Window) -> LocalBoxFuture<Result<()>>;

    fn move_popup_to_window(
        &self,
        popup: &adw::Window,
        to: WindowFilter,
        offset: (i32, i32),
    ) -> LocalBoxFuture<Result<()>>;
}

pub async fn default() -> Result<Box<dyn Platform>> {
    let platform = default::Platform::new().await?;
    Ok(Box::new(platform))
}
