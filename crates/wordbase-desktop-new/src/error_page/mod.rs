use relm4::prelude::*;

mod ui;

#[derive(Debug)]
pub struct ErrorPage;

impl AsyncComponent for ErrorPage {
    type Init = anyhow::Error;
    type Input = ();
    type Output = ();
    type CommandOutput = ();
    type Root = ui::ErrorPage;
    type Widgets = ();

    fn init_root() -> Self::Root {
        ui::ErrorPage::new()
    }

    async fn init(
        err: Self::Init,
        root: Self::Root,
        _sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        root.message().set_text(&format!("{err:?}"));
        AsyncComponentParts {
            model: Self,
            widgets: (),
        }
    }
}
