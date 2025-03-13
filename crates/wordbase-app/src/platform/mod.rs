mod wayland;
use wayland as default;

use wordbase::protocol::WindowFilter;

use std::pin::Pin;

use anyhow::Result;

pub trait Platform {
    fn affix_to_focused_window(
        &self,
        overlay: &adw::ApplicationWindow,
    ) -> Pin<Box<dyn Future<Output = Result<()>>>>;

    fn move_to_window(
        &self,
        window: &adw::ApplicationWindow,
        target: WindowFilter,
        offset: (i32, i32),
    ) -> Pin<Box<dyn Future<Output = Result<()>>>>;
}

pub fn default() -> Box<dyn Platform> {
    default::new()
}
