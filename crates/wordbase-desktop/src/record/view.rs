use {
    super::render::{RecordRender, RecordRenderConfig, RecordRenderMsg, RecordRenderResponse},
    crate::{Dictionaries, theme},
    futures::never::Never,
    relm4::prelude::*,
    tokio::task::JoinHandle,
    tracing::warn,
    wordbase::{Lookup, RecordKind},
    wordbase_engine::Engine,
};

#[derive(Debug)]
pub struct RecordView {
    engine: Engine,
    dictionaries: Dictionaries,
    render: Controller<RecordRender>,
    recv_default_theme_task: JoinHandle<()>,
}

impl Drop for RecordView {
    fn drop(&mut self) {
        self.recv_default_theme_task.abort();
    }
}

#[derive(Debug)]
pub struct RecordViewConfig {
    pub engine: Engine,
    pub dictionaries: Dictionaries,
}

#[derive(Debug)]
pub enum RecordViewMsg {
    Dictionaries(Dictionaries),
    Lookup(Lookup),
}

#[derive(Debug)]
pub enum RecordViewResponse {
    GotRecords { bytes_scanned: usize },
}

#[relm4::component(pub, async)]
impl AsyncComponent for RecordView {
    type Init = RecordViewConfig;
    type Input = RecordViewMsg;
    type Output = RecordViewResponse;
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
                records: Vec::new(),
            })
            .forward(sender.input_sender(), |resp| match resp {
                RecordRenderResponse::RequestLookup { query } => RecordViewMsg::Lookup(Lookup {
                    context: query,
                    cursor: 0,
                }),
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
            recv_default_theme_task,
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
            RecordViewMsg::Dictionaries(dictionaries) => {
                self.dictionaries = dictionaries.clone();
                _ = self
                    .render
                    .sender()
                    .send(RecordRenderMsg::Dictionaries(dictionaries));
            }
            RecordViewMsg::Lookup(lookup) => {
                let records = match self.engine.lookup(&lookup, RecordKind::ALL).await {
                    Ok(records) => records,
                    Err(err) => {
                        warn!("Failed to fetch records: {err:?}");
                        return;
                    }
                };
                if records.is_empty() {
                    return;
                }

                _ = self.render.sender().send(RecordRenderMsg::Records(records));

                // TODO
                let bytes_scanned = lookup.context.len() - lookup.cursor;
                _ = sender
                    .output_sender()
                    .send(RecordViewResponse::GotRecords { bytes_scanned });
            }
        }
    }
}
