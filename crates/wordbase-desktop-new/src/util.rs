use std::fmt::Debug;

use anyhow::Result;
use relm4::prelude::*;

use crate::AppEvent;

pub trait AppComponent: Sized + AsyncComponent + 'static {
    type Args;
    type Msg: Debug + 'static;
    type Ui: Debug + Default + Clone;

    async fn init(
        args: Self::Args,
        ui: Self::Ui,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self>;

    #[expect(clippy::unused_async, reason = "async for implementors")]
    async fn update(
        &mut self,
        msg: Self::Msg,
        sender: &AsyncComponentSender<Self>,
        ui: &Self::Ui,
    ) -> Result<()> {
        let (_, _, _) = (msg, sender, ui);
        Ok(())
    }

    #[expect(clippy::unused_async, reason = "async for implementors")]
    async fn update_event(
        &mut self,
        event: AppEvent,
        sender: &AsyncComponentSender<Self>,
        ui: &Self::Ui,
    ) -> Result<()> {
        let (_, _, _) = (event, sender, ui);
        Ok(())
    }
}

// fucking orphan rule won't let me blanket impl for `T: AppComponent`

macro_rules! impl_component {
    ($T:ty) => {
        const _: () = {
            use relm4::prelude::*;

            impl AsyncComponent for $T {
                type Init = <$T as AppComponent>::Args;
                type Input = <$T as AppComponent>::Msg;
                type Output = anyhow::Error;
                type CommandOutput = AppEvent;
                type Root = <$T as AppComponent>::Ui;
                type Widgets = ();

                fn init_root() -> Self::Root {
                    <$T as AppComponent>::Ui::default()
                }

                async fn init(
                    init: Self::Init,
                    root: Self::Root,
                    sender: AsyncComponentSender<Self>,
                ) -> AsyncComponentParts<Self> {
                    $crate::forward_events(&sender);
                    <$T as AppComponent>::init(init, root, sender).await
                }

                async fn update_with_view(
                    &mut self,
                    (): &mut Self::Widgets,
                    msg: Self::Input,
                    sender: AsyncComponentSender<Self>,
                    root: &Self::Root,
                ) {
                    let result = <$T as AppComponent>::update(self, msg, &sender, root).await;
                    if let Err(err) = result {
                        sender
                            .output(err)
                            .expect("failed to propagate error to parent");
                    }
                }

                async fn update_cmd_with_view(
                    &mut self,
                    (): &mut Self::Widgets,
                    msg: Self::CommandOutput,
                    sender: AsyncComponentSender<Self>,
                    root: &Self::Root,
                ) {
                    let result = <$T as AppComponent>::update_event(self, msg, &sender, root).await;
                    if let Err(err) = result {
                        sender
                            .output(err)
                            .expect("failed to propagate error to parent");
                    }
                }
            }
        };
    };
}

pub(crate) use impl_component;
