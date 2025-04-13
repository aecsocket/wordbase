use {
    crate::{APP_ID, AppEvent, forward_events, gettext, record_view, toast_result},
    anyhow::{Context, Result},
    glib::clone,
    maud::Markup,
    relm4::{
        adw::{gdk, gio, prelude::*},
        prelude::*,
    },
    wordbase_engine::Engine,
};

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
    toaster: adw::ToastOverlay,
    engine: Engine,
    last_html: Option<Markup>,
    last_query: String,
}

#[derive(Debug)]
#[doc(hidden)]
pub enum Msg {
    SetAnkiConnectConfig,
    SetTexthookerUrl(String),
    SetQuery(String),
    Query,
    Html(Markup),
    CopyHtml,
}

impl AsyncComponent for Model {
    type Init = (adw::ApplicationWindow, Engine);
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

        let copy_html = gio::ActionEntry::builder("copy-html")
            .activate(clone!(
                #[strong]
                sender,
                move |_, _, _| sender.input(Msg::CopyHtml)
            ))
            .build();
        window.add_action_entries([copy_html]);
        let window = window.upcast::<gtk::Window>();

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
            move |entry| sender.input(Msg::SetQuery(entry.text().into())),
        ));
        root.search_entry().connect_activate(clone!(
            #[strong]
            sender,
            move |entry| sender.input(Msg::SetQuery(entry.text().into()))
        ));

        let record_view = record_view::Model::builder()
            .launch(engine.clone())
            .forward(sender.input_sender(), |resp| match resp {
                record_view::Response::Html(html) => Msg::Html(html),
                record_view::Response::Query(query) => Msg::SetQuery(query),
            });
        root.search_view().set_content(Some(record_view.widget()));

        let toaster = root.toaster();
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
                .launch((engine.clone(), window, toaster.clone()))
                .detach(),
            toaster,
            engine,
            record_view,
            last_html: None,
            last_query: String::new(),
        };

        root.dictionaries()
            .add(model.overview_dictionaries.widget());
        root.search_dictionaries()
            .set_child(Some(model.search_dictionaries.widget()));

        root.themes().add(model.overview_themes.widget());
        root.search_themes()
            .set_child(Some(model.search_themes.widget()));

        root.quit()
            .connect_activated(|_| relm4::main_application().quit());

        AsyncComponentParts { model, widgets: () }
    }

    async fn update(
        &mut self,
        message: Self::Input,
        sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        toast_result(
            &self.toaster,
            match message {
                Msg::SetAnkiConnectConfig => Ok(()),
                Msg::SetTexthookerUrl(url) => self
                    .engine
                    .set_texthooker_url(&url)
                    .await
                    .with_context(|| gettext("Failed to set texthooker URL")),
                Msg::SetQuery(query) => {
                    self.last_query.clone_from(&query);
                    sender.input(Msg::Query);
                    Ok(())
                }
                Msg::Html(html) => {
                    self.last_html = Some(html);
                    Ok(())
                }
                Msg::CopyHtml => {
                    if let Some(html) = &self.last_html {
                        gdk::Display::default()
                            .expect("should have default display")
                            .clipboard()
                            .set_text(&html.0);
                        root.toaster()
                            .add_toast(adw::Toast::new(gettext("Copied HTML to clipboard")));
                    }
                    Ok(())
                }
                Msg::Query => query(self, root)
                    .await
                    .with_context(|| gettext("Failed to perform lookup")),
            },
        );
    }

    async fn update_cmd_with_view(
        &mut self,
        _widgets: &mut Self::Widgets,
        event: Self::CommandOutput,
        sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        if record_view::should_requery(&event) {
            sender.input(Msg::Query);
        }
    }
}

async fn query(model: &Model, root: &ui::Manager) -> Result<()> {
    let query = &model.last_query;
    let records = model
        .engine
        .lookup(query, 0, record_view::SUPPORTED_RECORD_KINDS)
        .await
        .context("failed to perform lookup")?;

    let longest_scan_chars = record_view::longest_scan_chars(query, &records);
    root.search_entry()
        .select_region(0, i32::try_from(longest_scan_chars).unwrap_or(-1));
    model
        .record_view
        .sender()
        .emit(record_view::Msg::Render(records));
    Ok(())
}
