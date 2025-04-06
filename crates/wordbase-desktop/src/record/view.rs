use {
    super::render::{RecordRender, RecordRenderMsg, RecordRenderResponse, SUPPORTED_RECORD_KINDS},
    crate::theme::{self, Theme},
    relm4::prelude::*,
    std::sync::Arc,
    wordbase::{Lookup, RecordLookup},
    wordbase_engine::Engine,
};

#[derive(Debug)]
pub struct RecordView {
    engine: Engine,
    render: Controller<RecordRender>,
    lookup: Option<Lookup>,
}

#[derive(Debug)]
pub enum RecordViewMsg {
    Lookup(Lookup),
    #[doc(hidden)]
    DoLookup,
}

#[derive(Debug)]
pub struct RecordViewResponse {
    pub records: Arc<Vec<RecordLookup>>,
}

#[derive(Debug)]
#[doc(hidden)]
pub enum RecordCommandMsg {
    ReLookup,
    DefaultTheme(Arc<Theme>),
}

#[relm4::component(pub, async)]
impl AsyncComponent for RecordView {
    type Init = Engine;
    type Input = RecordViewMsg;
    type Output = RecordViewResponse;
    type CommandOutput = RecordCommandMsg;

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
            .launch(RecordRender {
                default_theme,
                custom_theme: None,
                dictionaries: Arc::default(),
                records: Arc::default(),
            })
            .forward(sender.input_sender(), |resp| match resp {
                RecordRenderResponse::RequestLookup { query } => RecordViewMsg::Lookup(Lookup {
                    context: query,
                    cursor: 0,
                }),
            });

        let mut recv_default_theme_changed = theme::recv_default_changed().await;
        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    while let Ok(default_theme) = recv_default_theme_changed.recv().await {
                        _ = out.send(RecordCommandMsg::DefaultTheme(default_theme));
                    }
                })
                .drop_on_shutdown()
        });

        let mut recv_event = engine.recv_event();
        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    while let Ok(_) = recv_event.recv().await {
                        _ = out.send(RecordCommandMsg::ReLookup);
                    }
                })
                .drop_on_shutdown()
        });

        let model = Self {
            engine,
            render,
            lookup: None,
        };
        let widgets = view_output!();
        AsyncComponentParts { model, widgets }
    }

    async fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            RecordCommandMsg::ReLookup => {
                sender.input(RecordViewMsg::DoLookup);
            }
            RecordCommandMsg::DefaultTheme(theme) => {
                _ = self
                    .render
                    .sender()
                    .send(RecordRenderMsg::DefaultTheme(theme));
            }
        }
    }

    async fn update(
        &mut self,
        message: Self::Input,
        sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            RecordViewMsg::Lookup(lookup) => {
                self.lookup = Some(lookup);
                sender.input(RecordViewMsg::DoLookup);
            }
            RecordViewMsg::DoLookup => {
                let Some(lookup) = &self.lookup else {
                    return;
                };
                let Ok(records) = self
                    .engine
                    .lookup(&lookup.context, lookup.cursor, SUPPORTED_RECORD_KINDS)
                    .await
                else {
                    return;
                };

                let records = Arc::new(records);
                let dictionaries = self.engine.dictionaries.load();
                _ = self.render.sender().send(RecordRenderMsg::Render {
                    dictionaries: dictionaries.clone(),
                    records: records.clone(),
                });
                sender.output(RecordViewResponse { records });
            }
        }
    }
}
