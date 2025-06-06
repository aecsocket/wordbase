#![expect(missing_docs, reason = "build crate")]

fn main() {
    relm4_icons_build::bundle_icons(
        "icon_names.rs",
        Some("io.github.aecsocket.Wordbase"),
        None,
        None::<&str>,
        [
            "settings",
            "library",
            "larger-brush",
            "check-plain",
            "chain-link-loose",
            "globe-alt2",
            "sad-computer",
            "people",
        ],
    );
}
