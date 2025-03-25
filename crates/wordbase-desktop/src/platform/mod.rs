mod wayland;
use wayland as default;

use std::pin::Pin;

use anyhow::Result;
use wordbase::protocol::WindowFilter;

pub trait Platform {
    fn affix_to_focused_window(
        &self,
        window: &adw::ApplicationWindow,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + '_>>;

    fn move_to_window(
        &self,
        window: &adw::ApplicationWindow,
        target: WindowFilter,
        offset: (i32, i32),
    ) -> Pin<Box<dyn Future<Output = Result<()>> + '_>>;
}

pub async fn default() -> Result<Box<dyn Platform>> {
    let platform = default::Platform::new().await?;
    Ok(Box::new(platform))
}
