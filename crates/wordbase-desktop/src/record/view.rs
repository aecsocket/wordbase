use {
    super::render,
    relm4::prelude::*,
    std::sync::Arc,
    tokio::sync::mpsc,
    wordbase::{Lookup, RecordLookup},
    wordbase_engine::{Engine, Event},
};

#[derive(Debug)]
pub struct RecordView {
    engine: Engine,
    render: Controller<render::RecordRender>,
    lookup: Option<Lookup>,
}

#[derive(Debug)]
pub enum RecordViewMsg {
    Lookup {
        lookup: Lookup,
        send_records: mpsc::Sender<Arc<Vec<RecordLookup>>>,
    },
    ReLookup,
    #[doc(hidden)]
    DoLookup,
}

#[derive(Debug)]
pub struct RecordViewResponse {
    pub records: Arc<Vec<RecordLookup>>,
}

#[relm4::component(pub, async)]
impl AsyncComponent for RecordView {
    type Init = Engine;
    type Input = RecordViewMsg;
    type Output = RecordViewResponse;
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
        let render = render::RecordRender::builder()
            .launch(render::RecordRender {
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

        let mut recv_event = engine.recv_event();
        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    while let Ok(event) = recv_event.recv().await {
                        if matches!(event, Event::SyncDictionaries | Event::SyncProfiles) {
                            _ = out.send(RecordCommandMsg::ReLookup);
                        }
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
                _ = self.render.sender().send(RecordRenderMsg::Render {
                    dictionaries: self.engine.dictionaries(),
                    records: records.clone(),
                });
                _ = sender.output(RecordViewResponse { records });
            }
        }
    }
}
