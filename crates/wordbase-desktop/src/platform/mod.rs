mod wayland;
use wayland as default;
// mod noop;
// use noop as default;

use futures::future::LocalBoxFuture;

use anyhow::Result;
use relm4::adw;

pub trait Platform {
    fn affix_to_focused_window(&self, window: &adw::Window) -> LocalBoxFuture<Result<()>>;

    fn move_to_window(
        &self,
        window: &adw::Window,
        target: WindowFilter,
        offset: (i32, i32),
    ) -> LocalBoxFuture<Result<()>>;
}

pub async fn default() -> Result<Box<dyn Platform>> {
    let platform = default::Platform::new().await?;
    Ok(Box::new(platform))
}

#[derive(Debug, Clone)]
pub struct WindowFilter {}
