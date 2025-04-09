mod ui;

use gtk4::prelude::{CheckButtonExt, ListBoxRowExt};
use relm4::prelude::*;

use crate::theme::CustomTheme;

use super::theme_row;

#[derive(Debug)]
pub struct Model {
    default_theme: Controller<theme_row::Model>,
    custom_themes: Vec<Controller<theme_row::Model>>,
}

impl Component for Model {
    type Init = adw::Window;
    type Input = ();
    type Output = ();
    type CommandOutput = ();
    type Root = ui::Themes;
    type Widgets = ();

    fn init_root() -> Self::Root {
        ui::Themes::new()
    }

    fn init(
        window: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let default_theme = theme_row::Model::builder()
            .launch((window.clone(), None))
            .detach();
        let group = default_theme.widget().enabled();
        group.set_group(Some(&group));
        root.list()
            .insert(default_theme.widget(), root.import_button().index());

        ComponentParts {
            model: Self {
                default_theme,
                custom_themes: Vec::new(),
            },
            widgets: (),
        }
    }
}
