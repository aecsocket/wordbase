cfg_if! {
    if #[cfg(all(
        feature = "popup",
        unix,
        not(target_vendor = "apple"),
        not(target_os = "emscripten"),
    ))] {
        pub mod wayland;
        pub use wayland as default;
    } else {
        pub mod noop;
        pub use noop as default;
    }
}

use cfg_if::cfg_if;
