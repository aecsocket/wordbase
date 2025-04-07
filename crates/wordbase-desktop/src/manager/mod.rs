use dictionary_row::DictionaryRow;
use glib::clone;
use relm4::{
    adw::{gio, prelude::*},
    prelude::*,
};
use tracing::{error, info};
use wordbase::{DictionaryKind, DictionaryMeta, Lookup};
use wordbase_engine::{Engine, Event};

use crate::{
    gettext,
    record::view::{RecordView, RecordViewMsg},
};

mod dictionary_row;
mod error_dialog;
mod ui;

#[derive(Debug)]
pub struct Manager {
    toasts: adw::ToastOverlay,
    record_view: AsyncController<RecordView>,
    texthooker_connected: bool,
    engine: Engine,
}

#[derive(Debug)]
pub enum ManagerMsg {
    #[doc(hidden)]
    ImportDictionaries(gio::ListModel),
    #[doc(hidden)]
    SetAnkiConnectConfig,
    #[doc(hidden)]
    SetTexthookerUrl(String),
    #[doc(hidden)]
    Search(String),
    #[doc(hidden)]
    Error { title: String, err: anyhow::Error },
}

#[derive(Debug)]
pub enum ManagerCommandMsg {
    #[doc(hidden)]
    TexthookerConnected,
    #[doc(hidden)]
    TexthookerDisconnected,
}

impl AsyncComponent for Manager {
    type Init = (Engine, gio::Settings);
    type Input = ManagerMsg;
    type Output = ();
    type CommandOutput = ManagerCommandMsg;
    type Root = ui::Manager;
    type Widgets = ui::Manager;

    fn init_root() -> Self::Root {
        ui::Manager::new()
    }

    async fn init(
        (engine, settings): Self::Init,
        widgets: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        settings
            .bind("manager-width", &widgets, "default-width")
            .build();
        settings
            .bind("manager-height", &widgets, "default-height")
            .build();
        settings
            .bind(
                "manager-search-sidebar-open",
                &widgets.search_sidebar_toggle(),
                "active",
            )
            .build();

        let parent = widgets
            .import_dictionary()
            .parent()
            .unwrap()
            .downcast::<gtk::ListBox>()
            .unwrap();

        parent.append(
            DictionaryRow::builder()
                .launch(DictionaryRow::ImportingStart {
                    file_path: "jitendex.zip".into(),
                })
                .detach()
                .widget(),
        );
        parent.append(
            DictionaryRow::builder()
                .launch(DictionaryRow::Importing {
                    meta: DictionaryMeta::new(DictionaryKind::YomichanAudio, "foo"),
                    progress: 0.2,
                })
                .detach()
                .widget(),
        );

        for dict in engine.dictionaries.load().by_id.values() {
            let row = DictionaryRow::builder().launch(DictionaryRow::Imported(dict.clone()));
            parent.append(row.widget());

            // row.widget()
            // .insert_before(&parent, Some(&widgets.import_dictionary()));
        }

        widgets.import_dictionary().connect_activated(clone!(
            #[strong]
            widgets,
            #[strong]
            sender,
            move |_| {
                widgets.import_dictionary_dialog().open_multiple(
                    Some(&widgets),
                    None::<&gio::Cancellable>,
                    clone!(
                        #[strong]
                        sender,
                        move |result| {
                            if let Ok(files) = result {
                                sender.input(ManagerMsg::ImportDictionaries(files));
                            }
                        }
                    ),
                );
            },
        ));

        widgets.ankiconnect_server_url().connect_changed(clone!(
            #[strong]
            sender,
            move |_| sender.input(ManagerMsg::SetAnkiConnectConfig),
        ));
        widgets.ankiconnect_api_key().connect_changed(clone!(
            #[strong]
            sender,
            move |_| sender.input(ManagerMsg::SetAnkiConnectConfig),
        ));

        widgets.texthooker_url().set_text(&engine.texthooker_url());
        widgets.texthooker_url().connect_changed(clone!(
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
                        _ = out.send(if engine.texthooker_connected() {
                            ManagerCommandMsg::TexthookerConnected
                        } else {
                            ManagerCommandMsg::TexthookerDisconnected
                        });

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

        widgets.search_entry().connect_search_changed(clone!(
            #[strong]
            sender,
            move |entry| sender.input(ManagerMsg::Search(entry.text().into())),
        ));

        let record_view = RecordView::builder().launch(engine.clone()).detach();
        widgets
            .search_view()
            .set_content(Some(record_view.widget()));

        let model = Self {
            toasts: widgets.toast_overlay(),
            record_view,
            texthooker_connected: engine.texthooker_connected(),
            engine,
        };
        AsyncComponentParts { model, widgets }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, _sender: AsyncComponentSender<Self>) {
        widgets
            .texthooker_connected()
            .set_visible(self.texthooker_connected);
        widgets
            .texthooker_disconnected()
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
            ManagerMsg::SetAnkiConnectConfig => {}
            ManagerMsg::SetTexthookerUrl(url) => {
                if let Err(err) = self.engine.set_texthooker_url(&url).await {
                    info!("Set texthooker URL to {url:?}");
                    sender.input(ManagerMsg::Error {
                        title: gettext("Failed to set texthooker URL").into(),
                        err,
                    });
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
            ManagerMsg::Error { title, err } => {
                error!("{title}: {err:?}");
                let toast = adw::Toast::builder().title(title).build();
                self.toasts.add_toast(toast);
            }
        }
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
