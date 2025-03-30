#![doc = include_str!("../README.md")]

use relm4::{ComponentParts, RelmApp, SimpleComponent, adw};

struct Model {}

struct Widgets {}

impl SimpleComponent for Model {
    type Input = ();
    type Output = ();
    type Init = ();
    type Root = adw::Window;
    type Widgets = Widgets;

    fn init_root() -> Self::Root {
        adw::Window::builder().title("Wordbase").build()
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: relm4::ComponentSender<Self>,
    ) -> relm4::ComponentParts<Self> {
        ComponentParts {
            model: Self {},
            widgets: Widgets {},
        }
    }
}

fn main() {
    RelmApp::new("io.github.aecsocket.Wordbase").run::<Model>(());
}
