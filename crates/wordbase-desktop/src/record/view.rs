use {
    super::render::{
        RecordRender, RecordRenderConfig, RecordRenderMsg, RecordRenderResponse,
        SUPPORTED_RECORD_KINDS,
    },
    crate::theme,
    futures::never::Never,
    relm4::prelude::*,
    std::sync::Arc,
    tokio_util::task::AbortOnDropHandle,
    wordbase::RecordLookup,
    wordbase_engine::Engine,
};

#[derive(Debug)]
pub struct RecordView {
    engine: Engine,
    render: Controller<RecordRender>,
    recv_default_theme_task: AbortOnDropHandle<()>,
}

#[derive(Debug)]
pub enum RecordViewMsg {
    SyncDictionaries,
    Records(Arc<Vec<RecordLookup>>),
    #[doc(hidden)]
    Lookup {
        query: String,
    },
}

#[relm4::component(pub, async)]
impl AsyncComponent for RecordView {
    type Init = Engine;
    type Input = RecordViewMsg;
    type Output = ();
    type CommandOutput = ();

    view! {
        adw::Bin {
            model.render.widget(),
        }
    }

    async fn init(
        engine: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let default_theme = theme::default().await;
        let render = RecordRender::builder()
            .launch(RecordRenderConfig {
                default_theme,
                custom_theme: None,
                dictionaries: engine.dictionaries.clone(),
                records: Arc::new(Vec::new()),
            })
            .forward(sender.input_sender(), |resp| match resp {
                RecordRenderResponse::RequestLookup { query } => RecordViewMsg::Lookup { query },
            });

        let mut recv_default_theme_changed = theme::recv_default_changed().await;
        let render_sender = render.sender().clone();
        let recv_default_theme_task = tokio::spawn(async move {
            // TODO: is there a better way to do this?
            let _: Option<Never> = async move {
                loop {
                    let default_theme = recv_default_theme_changed.recv().await.ok()?;
                    render_sender
                        .send(RecordRenderMsg::DefaultTheme(default_theme))
                        .ok()?;
                }
            }
            .await;
        });

        let model = Self {
            engine,
            render,
            recv_default_theme_task: AbortOnDropHandle::new(recv_default_theme_task),
        };
        let widgets = view_output!();
        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        message: Self::Input,
        _sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            RecordViewMsg::SyncDictionaries => {
                _ = self.render.sender().send(RecordRenderMsg::Dictionaries(
                    self.engine.dictionaries.clone(),
                ));
            }
            RecordViewMsg::Records(records) => {
                _ = self.render.sender().send(RecordRenderMsg::Records(records));
            }
            RecordViewMsg::Lookup { query } => {
                let Ok(records) = self.engine.lookup(&query, 0, SUPPORTED_RECORD_KINDS).await
                else {
                    return;
                };

                _ = self
                    .render
                    .sender()
                    .send(RecordRenderMsg::Records(Arc::new(records)));
            }
        }
    }
}
