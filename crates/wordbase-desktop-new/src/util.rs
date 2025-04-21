use std::fmt::Debug;

use anyhow::Result;
use relm4::prelude::*;

use crate::AppEvent;

pub trait AppComponent: Sized + AsyncComponent + 'static {
    type Init;
    type Input: Debug + 'static;
    type Root: Debug + Default + Clone;

    async fn init(
        init: <Self as AppComponent>::Init,
        root: <Self as AppComponent>::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self>;

    async fn update(
        &mut self,
        msg: <Self as AppComponent>::Input,
        sender: &AsyncComponentSender<Self>,
        root: &<Self as AppComponent>::Root,
    ) -> Result<()> {
        let (_, _, _) = (msg, sender, root);
        Ok(())
    }

    async fn update_event(
        &mut self,
        event: AppEvent,
        sender: &AsyncComponentSender<Self>,
        root: &<Self as AppComponent>::Root,
    ) -> Result<()> {
        let (_, _, _) = (event, sender, root);
        Ok(())
    }
}

// fucking orphan rule won't let me blanket impl for `T: AppComponent`

macro_rules! impl_component {
    ($T:ty) => {
        impl AsyncComponent for $T {
            type Init = <$T as AppComponent>::Init;
            type Input = <$T as AppComponent>::Input;
            type Output = anyhow::Error;
            type CommandOutput = AppEvent;
            type Root = <$T as AppComponent>::Root;
            type Widgets = ();

            fn init_root() -> Self::Root {
                <$T as AppComponent>::Root::default()
            }

            async fn init(
                init: Self::Init,
                root: Self::Root,
                sender: AsyncComponentSender<Self>,
            ) -> AsyncComponentParts<Self> {
                forward_events(&sender);
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
                    sender.output(err);
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
                    sender.output(err);
                }
            }
        }
    };
}

pub(crate) use impl_component;
