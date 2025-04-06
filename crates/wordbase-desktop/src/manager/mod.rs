use glib::clone;
use relm4::{
    adw::{gio, prelude::*},
    loading_widgets::LoadingWidgets,
    prelude::*,
};
use tracing::info;
use wordbase::Lookup;
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

    fn init_loading_widgets(root: Self::Root) -> Option<LoadingWidgets> {
        // let root = root.upcast::<gtk::Window>();
        // view! {
        //     #[local]
        //     root {
        //         #[name(spinner)]
        //         adw::Spinner {}
        //     }
        // }
        None
        // Some(LoadingWidgets::new(root, spinner))
    }

    async fn init(
        (engine, settings): Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        settings
            .bind("manager-width", &root, "default-width")
            .build();
        settings
            .bind("manager-height", &root, "default-height")
            .build();
        settings
            .bind(
                "manager-search-sidebar-open",
                &root.search_sidebar_toggle(),
                "active",
            )
            .build();

        root.import_dictionary().connect_activated(clone!(
            #[strong]
            root,
            #[strong]
            sender,
            move |_| {
                root.import_dictionary_dialog().open_multiple(
                    Some(&root),
                    None::<&gio::Cancellable>,
                    clone!(
                        #[strong]
                        sender,
                        move |result| {
                            sender.input(match result {
                                Ok(files) => ManagerMsg::ImportDictionaries(files),
                                Err(err) => ManagerMsg::Error {
                                    title: gettext("Failed to select dictionaries to import")
                                        .into(),
                                    err: err.into(),
                                },
                            });
                        }
                    ),
                );
            },
        ));

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

        root.search_entry().connect_search_changed(clone!(
            #[strong]
            sender,
            move |entry| sender.input(ManagerMsg::Search(entry.text().into())),
        ));

        let record_view = RecordView::builder().launch(engine.clone()).detach();
        root.search_view().set_content(Some(record_view.widget()));

        let model = Self {
            toasts: root.toast_overlay(),
            record_view,
            texthooker_connected: engine.texthooker_connected(),
            engine,
        };
        AsyncComponentParts {
            model,
            widgets: root,
        }
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
                let toast = adw::Toast::builder()
                    .title(title)
                    .button_label(gettext("Details"))
                    .build();
                toast.connect_button_clicked(move |_| {
                    todo!();
                });
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
