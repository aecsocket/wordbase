use {
    anyhow::{Context, Result, bail},
    futures::future::LocalBoxFuture,
    relm4::adw::{self, prelude::*},
    wordbase::WindowFilter,
};

/*
implementation notes:
- `Meta.Window`'s `get_id()` and `get_stable_sequence()` are not for us,
  since we show/hide the `gtk::Window`, which creates/destroys `Meta.Window`s
*/

pub struct Platform {
    integration: IntegrationProxy<'static>,
}

impl Platform {
    pub async fn new() -> Result<Self> {
        let dbus = zbus::Connection::session()
            .await
            .context("failed to establish session bus connection")?;
        let integration = IntegrationProxy::new(&dbus)
            .await
            .context("failed to create integration dbus proxy")?;
        Ok(Self { integration })
    }
}

impl super::Platform for Platform {
    fn init_overlay(&self, overlay: &adw::Window) -> LocalBoxFuture<Result<()>> {
        let overlay = overlay.clone();
        Box::pin(async move {
            let focused_window = self
                .integration
                .get_focused_window_id()
                .await
                .context("failed to get focused window ID")?;
            if focused_window == 0 {
                bail!("no focused window");
            }

            let window_id = get_window_id(&self.integration, &overlay).await?;
            self.integration
                .affix_to_window(focused_window, window_id)
                .await
                .context("failed to affix overlay to focused window")?;
            Ok(())
        })
    }

    fn init_popup(&self, popup: &adw::Window) -> LocalBoxFuture<Result<()>> {
        let popup = popup.clone();
        Box::pin(async move {
            popup.present();
            Ok(())
        })
    }

    fn move_popup_to_window(
        &self,
        popup: &adw::Window,
        to: WindowFilter,
        offset: (i32, i32),
    ) -> LocalBoxFuture<Result<()>> {
        let popup = popup.clone();
        Box::pin(async move {
            let popup_window_id = get_window_id(&self.integration, &popup).await?;
            self.integration
                .move_to_window(
                    popup_window_id,
                    to.id.unwrap_or_default(),
                    to.title.as_deref().unwrap_or_default(),
                    to.wm_class.as_deref().unwrap_or_default(),
                    offset.0,
                    offset.1,
                )
                .await
                .context("failed to request to move popup window")?;
            Ok(())
        })
    }
}

async fn get_window_id(integration: &IntegrationProxy<'_>, window: &adw::Window) -> Result<u64> {
    let window_token = format!("{:016x}", rand::random::<u128>());
    let old_title = window.title();
    window.set_title(Some(&window_token));
    window.present();
    let window_id = integration
        .get_app_window_id(&window_token)
        .await
        .context("failed to get app window ID")?;
    window.set_title(old_title.as_deref());
    Ok(window_id)
}

#[zbus::proxy(
    interface = "io.github.aecsocket.WordbaseIntegration",
    default_service = "io.github.aecsocket.WordbaseIntegration",
    default_path = "/io/github/aecsocket/WordbaseIntegration"
)]
trait Integration {
    async fn get_focused_window_id(&self) -> zbus::Result<u64>;

    async fn get_app_window_id(&self, title: &str) -> zbus::Result<u64>;

    async fn affix_to_window(&self, parent: u64, child_id: u64) -> zbus::Result<()>;

    async fn move_to_window(
        &self,
        moved_id: u64,
        to_id: u64,
        to_title: &str,
        to_wm_class: &str,
        offset_x: i32,
        offset_y: i32,
    ) -> zbus::Result<()>;
}
