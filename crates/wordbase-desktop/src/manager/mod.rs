use foldhash::{HashMap, HashMapExt};
use glib::clone;
use relm4::{
    adw::{gio, prelude::*},
    prelude::*,
};
use tracing::info;
use wordbase::{DictionaryId, Lookup};
use wordbase_engine::{Engine, Event};

use crate::{
    APP_ID, gettext,
    record::view::{RecordView, RecordViewMsg},
};

mod dictionaries;
mod dictionary_row;
mod theme_row;
mod themes;
mod ui;

#[derive(Debug)]
pub struct Manager {
    window: adw::Window,
    record_view: AsyncController<RecordView>,
    texthooker_connected: bool,
    engine: Engine,
    overview_dictionaries: Controller<dictionaries::Model>,
    search_dictionaries: Controller<dictionaries::Model>,
    overview_themes: Controller<themes::Model>,
    search_themes: Controller<themes::Model>,
}

#[derive(Debug)]
#[doc(hidden)]
pub enum ManagerMsg {
    OverviewDictionaries(dictionaries::Response),
    SearchDictionaries(dictionaries::Response),
    ImportDictionaries(gio::ListModel),
    SetAnkiConnectConfig,
    SetTexthookerUrl(String),
    Search(String),
    Error(anyhow::Error),
}

#[derive(Debug)]
pub enum ManagerCommandMsg {
    TexthookerConnected,
    TexthookerDisconnected,
}

#[derive(Debug)]
pub struct Widgets {
    root: ui::Manager,
}

impl AsyncComponent for Manager {
    type Init = (adw::Window, Engine);
    type Input = ManagerMsg;
    type Output = ();
    type CommandOutput = ManagerCommandMsg;
    type Root = ui::Manager;
    type Widgets = Widgets;

    fn init_root() -> Self::Root {
        ui::Manager::new()
    }

    async fn init(
        (window, engine): Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let settings = gio::Settings::new(APP_ID);
        settings
            .bind(
                "manager-search-sidebar-open",
                &root.search_sidebar_toggle(),
                "active",
            )
            .build();

        root.ankiconnect_server_url().connect_changed(clone!(
            #[strong]
            sender,
            move |_| sender.input(ManagerMsg::SetAnkiConnectConfig),
        ));
        root.ankiconnect_api_key().connect_changed(clone!(
            #[strong]
            sender,
            move |_| sender.input(ManagerMsg::SetAnkiConnectConfig),
        ));

        root.texthooker_url().set_text(&engine.texthooker_url());
        root.texthooker_url().connect_changed(clone!(
            #[strong]
            sender,
            move |entry| sender.input(ManagerMsg::SetTexthookerUrl(entry.text().into())),
        ));
        sender.command(clone!(
            #[strong]
            engine,
            move |out, shutdown| {
                shutdown
                    .register(async move {
                        let mut recv_event = engine.recv_event();
                        while let Ok(event) = recv_event.recv().await {
                            match event {
                                Event::PullTexthookerConnected => {
                                    _ = out.send(ManagerCommandMsg::TexthookerConnected);
                                }
                                Event::PullTexthookerDisconnected => {
                                    _ = out.send(ManagerCommandMsg::TexthookerDisconnected);
                                }
                                _ => {}
                            }
                        }
                    })
                    .drop_on_shutdown()
            }
        ));

        root.search_entry().connect_search_changed(clone!(
            #[strong]
            sender,
            move |entry| sender.input(ManagerMsg::Search(entry.text().into())),
        ));

        let record_view = RecordView::builder().launch(engine.clone()).detach();
        root.search_view().set_content(Some(record_view.widget()));

        let model = Self {
            window: window.clone(),
            record_view,
            texthooker_connected: engine.texthooker_connected(),
            engine: engine.clone(),
            overview_dictionaries: dictionaries::Model::builder()
                .launch((window.clone(), engine.dictionaries()))
                .forward(sender.input_sender(), ManagerMsg::OverviewDictionaries),
            search_dictionaries: dictionaries::Model::builder()
                .launch((window.clone(), engine.dictionaries()))
                .forward(sender.input_sender(), ManagerMsg::SearchDictionaries),
            overview_themes: themes::Model::builder().launch(window.clone()).detach(),
            search_themes: themes::Model::builder().launch(window.clone()).detach(),
        };
        let mut widgets = Widgets { root: root.clone() };

        root.dictionaries()
            .add(model.overview_dictionaries.widget());
        root.search_dictionaries()
            .set_child(Some(model.search_dictionaries.widget()));

        root.themes().add(model.overview_themes.widget());
        root.search_themes()
            .set_child(Some(model.search_themes.widget()));

        model.update_view(&mut widgets, sender);
        AsyncComponentParts { model, widgets }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, sender: AsyncComponentSender<Self>) {
        let root = &widgets.root;

        root.texthooker_connected()
            .set_visible(self.texthooker_connected);
        root.texthooker_disconnected()
            .set_visible(!self.texthooker_connected);
    }

    async fn update(
        &mut self,
        message: Self::Input,
        sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            ManagerMsg::ImportDictionaries(files) => {
                todo!();
            }
            ManagerMsg::OverviewDictionaries(resp) => {
                // _ = self.search_dictionaries.sender().send(msg.clone());
                // if let Err(err) = dictionaries::apply(msg, &self.engine).await {
                //     sender.input(ManagerMsg::Error(err));
                // }
            }
            ManagerMsg::SearchDictionaries(resp) => {
                // _ = self.overview_dictionaries.sender().send(msg.clone());
                if let Err(err) = dictionaries::apply(resp, &self.engine).await {
                    sender.input(ManagerMsg::Error(err));
                }
            }
            ManagerMsg::SetAnkiConnectConfig => {}
            ManagerMsg::SetTexthookerUrl(url) => {
                if let Err(err) = self.engine.set_texthooker_url(&url).await {
                    info!("Set texthooker URL to {url:?}");
                    sender.input(ManagerMsg::Error(
                        err.context(gettext("Failed to set texthooker URL")),
                    ));
                }
            }
            ManagerMsg::Search(query) => {
                _ = self
                    .record_view
                    .sender()
                    .send(RecordViewMsg::Lookup(Lookup {
                        context: query,
                        cursor: 0,
                    }));
            }
            ManagerMsg::Error(_) => {}
        }
    }

    async fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        match &message {
            ManagerMsg::Error(err) => {
                let toast = adw::Toast::builder().title(err.to_string()).build();
                root.toast_overlay().add_toast(toast);
            }
            _ => {}
        }

        self.update(message, sender.clone(), root).await;
        self.update_view(widgets, sender);
    }

    async fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            ManagerCommandMsg::TexthookerConnected => {
                self.texthooker_connected = true;
            }
            ManagerCommandMsg::TexthookerDisconnected => {
                self.texthooker_connected = false;
            }
        }
    }
}
