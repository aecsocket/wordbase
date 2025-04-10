use {
    super::render::{self, RecordRenderMsg, SUPPORTED_RECORD_KINDS},
    crate::record::render::RecordRenderResponse,
    relm4::prelude::*,
    std::sync::Arc,
    wordbase::{Lookup, RecordLookup},
    wordbase_engine::Engine,
};

#[derive(Debug)]
pub struct RecordView {
    engine: Engine,
    render: Controller<render::RecordRender>,
    lookup: Option<Lookup>,
}

#[derive(Debug)]
pub enum RecordViewMsg {
    Lookup(Lookup),
    ReLookup,
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

        let model = Self {
            engine,
            render,
            lookup: None,
        };
        let widgets = view_output!();
        AsyncComponentParts { model, widgets }
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
                sender.input(RecordViewMsg::ReLookup);
            }
            RecordViewMsg::ReLookup => {
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
