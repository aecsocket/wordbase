use std::sync::Arc;

use foldhash::HashMap;
use futures::never::Never;
use relm4::prelude::*;
use tokio::task::JoinHandle;
use tracing::warn;
use wordbase::RecordKind;
use wordbase_engine::Engine;

use crate::theme;

use super::render::{RecordRender, RecordRenderConfig, RecordRenderMsg, RecordRenderResponse};

#[derive(Debug)]
pub struct RecordView {
    engine: Engine,
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
}

#[derive(Debug)]
pub enum RecordViewMsg {
    Lookup { query: String },
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
        init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let default_theme = theme::default().await;
        let render = RecordRender::builder()
            .launch(RecordRenderConfig {
                default_theme,
                custom_theme: None,
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
                        .send(RecordRenderMsg::SetDefaultTheme(default_theme))
                        .ok()?;
                }
            }
            .await;
        });

        let model = Self {
            engine: init.engine,
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
        root: &Self::Root,
    ) {
        match message {
            RecordViewMsg::Lookup { query } => {
                // TODO cache this
                let dictionaries = self
                    .engine
                    .dictionaries()
                    .await
                    .unwrap()
                    .into_iter()
                    .map(|dict| (dict.id, dict))
                    .collect::<HashMap<_, _>>();

                let records = match self.engine.lookup_lemma(&query, RecordKind::ALL).await {
                    Ok(records) => records,
                    Err(err) => {
                        warn!("Failed to fetch records for {query:?}: {err:?}");
                        return;
                    }
                };

                _ = self.render.sender().send(RecordRenderMsg::Lookup {
                    dictionaries: Arc::new(dictionaries),
                    records,
                });
            }
        }
    }
}
