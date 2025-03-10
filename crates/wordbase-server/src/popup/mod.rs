cfg_if! {
    if #[cfg(feature = "popup")] {
        if #[cfg(unix, not(target_vendor = "apple"), not(target_os = "emscripten"))] {
            mod wayland;
            pub type DefaultPopups = wayland::WaylandPopups;
        }
    } else {
        mod noop;
        pub type DefaultPopups = noop::NoopPopups;
    }
}

use anyhow::Result;
use cfg_if::cfg_if;
use wordbase::protocol::ShowPopupRequest;

pub trait Popups: Send + Sync + 'static {
    fn show(&self, request: ShowPopupRequest) -> Result<()>;
}
