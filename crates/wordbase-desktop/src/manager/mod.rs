use glib::clone;
use relm4::{
    adw::{gio, prelude::*},
    loading_widgets::LoadingWidgets,
    prelude::*,
};
use wordbase::Lookup;
use wordbase_engine::Engine;

use crate::{
    gettext,
    record::view::{RecordView, RecordViewMsg},
};

mod dictionary_row;
mod error_dialog;
mod ui;

#[derive(Debug)]
pub struct Manager {
    engine: Engine,
    toasts: adw::ToastOverlay,
    record_view: AsyncController<RecordView>,
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

impl AsyncComponent for Manager {
    type Init = (Engine, gio::Settings);
    type Input = ManagerMsg;
    type Output = ();
    type CommandOutput = ();
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

        root.texthooker_url()
            .set_text(&engine.texthooker_url().await.unwrap_or_default());
        root.texthooker_url().connect_changed(clone!(
            #[strong]
            sender,
            move |entry| sender.input(ManagerMsg::SetTexthookerUrl(entry.text().into())),
        ));

        root.search_entry().connect_search_changed(clone!(
            #[strong]
            sender,
            move |entry| sender.input(ManagerMsg::Search(entry.text().into())),
        ));

        let record_view = RecordView::builder().launch(engine.clone()).detach();
        root.search_view().set_content(Some(record_view.widget()));

        let model = Self {
            engine,
            toasts: root.toast_overlay(),
            record_view,
        };
        AsyncComponentParts {
            model,
            widgets: root,
        }
    }

    async fn update(
        &mut self,
        message: Self::Input,
        sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            ManagerMsg::ImportDictionaries(files) => {
                todo!();
            }
            ManagerMsg::SetAnkiConnectConfig => {}
            ManagerMsg::SetTexthookerUrl(url) => {
                if let Err(err) = self.engine.set_texthooker_url(url).await {
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
}
