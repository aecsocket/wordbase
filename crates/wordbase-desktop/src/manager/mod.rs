use glib::clone;
use relm4::{
    adw::{gio, prelude::*},
    prelude::*,
};
use tracing::info;
use wordbase_engine::Engine;

use crate::{APP_ID, AppEvent, forward_events, gettext, record_view};

mod dictionary_list;
mod dictionary_row;
mod theme_list;
mod theme_row;
mod ui;

#[derive(Debug)]
pub struct Model {
    overview_dictionaries: AsyncController<dictionary_list::Model>,
    search_dictionaries: AsyncController<dictionary_list::Model>,
    overview_themes: AsyncController<theme_list::Model>,
    search_themes: AsyncController<theme_list::Model>,
    record_view: AsyncController<record_view::Model>,
    engine: Engine,
    last_query: String,
}

#[derive(Debug)]
#[doc(hidden)]
pub enum Msg {
    SetAnkiConnectConfig,
    SetTexthookerUrl(String),
    Query(String),
    Requery,
    Error(anyhow::Error),
}

impl AsyncComponent for Model {
    type Init = (adw::Window, Engine);
    type Input = Msg;
    type Output = ();
    type CommandOutput = AppEvent;
    type Root = ui::Manager;
    type Widgets = ();

    fn init_root() -> Self::Root {
        ui::Manager::new()
    }

    async fn init(
        (window, engine): Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        forward_events(&sender);

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
            move |_| sender.input(Msg::SetAnkiConnectConfig),
        ));
        root.ankiconnect_api_key().connect_changed(clone!(
            #[strong]
            sender,
            move |_| sender.input(Msg::SetAnkiConnectConfig),
        ));

        root.texthooker_url().set_text(&engine.texthooker_url());
        root.texthooker_url().connect_changed(clone!(
            #[strong]
            sender,
            move |entry| sender.input(Msg::SetTexthookerUrl(entry.text().into())),
        ));

        root.search_entry().connect_search_changed(clone!(
            #[strong]
            sender,
            move |entry| sender.input(Msg::Query(entry.text().into())),
        ));
        root.search_entry().connect_activate(clone!(
            #[strong]
            sender,
            move |entry| sender.input(Msg::Query(entry.text().into()))
        ));

        let record_view = record_view::Model::builder()
            .launch(engine.clone())
            .forward(sender.input_sender(), |resp| match resp {
                record_view::Response::Query(query) => Msg::Query(query),
            });
        root.search_view().set_content(Some(record_view.widget()));

        let toaster = root.toast_overlay();
        let model = Self {
            overview_dictionaries: dictionary_list::Model::builder()
                .launch((engine.clone(), window.clone(), toaster.clone()))
                .detach(),
            search_dictionaries: dictionary_list::Model::builder()
                .launch((engine.clone(), window.clone(), toaster.clone()))
                .detach(),
            overview_themes: theme_list::Model::builder()
                .launch((engine.clone(), window.clone(), toaster.clone()))
                .detach(),
            search_themes: theme_list::Model::builder()
                .launch((engine.clone(), window, toaster))
                .detach(),
            engine,
            record_view,
            last_query: String::new(),
        };

        root.dictionaries()
            .add(model.overview_dictionaries.widget());
        root.search_dictionaries()
            .set_child(Some(model.search_dictionaries.widget()));

        root.themes().add(model.overview_themes.widget());
        root.search_themes()
            .set_child(Some(model.search_themes.widget()));

        AsyncComponentParts { model, widgets: () }
    }

    async fn update(
        &mut self,
        message: Self::Input,
        sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            Msg::SetAnkiConnectConfig => {}
            Msg::SetTexthookerUrl(url) => {
                if let Err(err) = self.engine.set_texthooker_url(&url).await {
                    info!("Set texthooker URL to {url:?}");
                    sender.input(Msg::Error(
                        err.context(gettext("Failed to set texthooker URL")),
                    ));
                }
            }
            Msg::Query(query) => {
                self.last_query.clone_from(&query);
                sender.input(Msg::Requery);
            }
            Msg::Requery => {
                let query = &self.last_query;
                let Ok(records) = self
                    .engine
                    .lookup(query, 0, record_view::SUPPORTED_RECORD_KINDS)
                    .await
                else {
                    return;
                };

                let longest_scan_chars = record_view::longest_scan_chars(query, &records);
                root.search_entry()
                    .select_region(0, i32::try_from(longest_scan_chars).unwrap_or(-1));

                self.record_view.sender().emit(record_view::Msg(records));
            }
            Msg::Error(_) => {}
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
            Msg::Error(err) => {
                let toast = adw::Toast::builder().title(err.to_string()).build();
                root.toast_overlay().add_toast(toast);
            }
            _ => {}
        }

        self.update(message, sender.clone(), root).await;
        self.update_view(widgets, sender);
    }

    async fn update_cmd_with_view(
        &mut self,
        _widgets: &mut Self::Widgets,
        event: Self::CommandOutput,
        sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        if record_view::should_requery(&event) {
            sender.input(Msg::Requery);
        }
    }
}
