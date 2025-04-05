use {
    super::render::{
        RecordRender, RecordRenderConfig, RecordRenderMsg, RecordRenderResponse, Records,
        SUPPORTED_RECORD_KINDS,
    },
    crate::{Dictionaries, theme},
    futures::never::Never,
    relm4::prelude::*,
    std::sync::Arc,
    tokio_util::task::AbortOnDropHandle,
    wordbase_engine::Engine,
};

#[derive(Debug)]
pub struct RecordView {
    engine: Engine,
    dictionaries: Arc<Dictionaries>,
    render: Controller<RecordRender>,
    recv_default_theme_task: AbortOnDropHandle<()>,
}

#[derive(Debug)]
pub struct RecordViewConfig {
    pub engine: Engine,
    pub dictionaries: Arc<Dictionaries>,
}

#[derive(Debug)]
pub enum RecordViewMsg {
    Dictionaries(Arc<Dictionaries>),
    Records(Arc<Records>),
    #[doc(hidden)]
    Lookup {
        query: String,
    },
}

#[relm4::component(pub, async)]
impl AsyncComponent for RecordView {
    type Init = RecordViewConfig;
    type Input = RecordViewMsg;
    type Output = ();
    type CommandOutput = ();

    view! {
        adw::Bin {
            model.render.widget(),
        }
    }

    async fn init(
        config: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let default_theme = theme::default().await;
        let render = RecordRender::builder()
            .launch(RecordRenderConfig {
                default_theme,
                custom_theme: None,
                dictionaries: config.dictionaries.clone(),
                records: Arc::<Records>::default(),
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
            engine: config.engine,
            dictionaries: config.dictionaries,
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
            RecordViewMsg::Dictionaries(dictionaries) => {
                self.dictionaries = dictionaries.clone();
                _ = self
                    .render
                    .sender()
                    .send(RecordRenderMsg::Dictionaries(dictionaries));
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
