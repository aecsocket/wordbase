use anyhow::{Context, Result, bail};
use futures::future::LocalBoxFuture;
use relm4::adw::{self, prelude::*};
use tracing::{debug, info};
use wordbase::WindowFilter;

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

const WINDOW_ID_KEY: &str = "wordbase_window_id";

impl super::Platform for Platform {
    fn init_overlay(&self, window: &adw::Window) -> LocalBoxFuture<Result<()>> {
        let window = window.clone();
        Box::pin(async move {
            let focused_window = self
                .integration
                .get_focused_window_id()
                .await
                .context("failed to get focused window ID")?;
            if focused_window == 0 {
                bail!("no focused window");
            }

            let window_token = format!("{:016x}", rand::random::<u128>());
            let old_title = window.title();
            window.set_title(Some(&window_token));
            window.present();
            let window_id = self
                .integration
                .get_app_window_id(&window_token)
                .await
                .context("failed to get app window ID")?;
            window.set_title(old_title.as_deref());

            self.integration
                .affix_to_window(focused_window, window_id)
                .await
                .context("failed to affix overlay to focused window")?;
            Ok(())
        })
    }

    fn move_to_window(
        &self,
        window: &adw::Window,
        to: WindowFilter,
        offset: (i32, i32),
    ) -> LocalBoxFuture<Result<()>> {
        let window = window.clone();
        Box::pin(async move {
            let window_id = read_window_id(&window)?;
            self.integration
                .move_to_window(
                    window_id,
                    to.id.unwrap_or_default(),
                    to.pid.unwrap_or_default(),
                    to.title.as_deref().unwrap_or_default(),
                    to.wm_class.as_deref().unwrap_or_default(),
                    offset.0,
                    offset.1,
                )
                .await
                .context("failed to send request to integration")?;
            Ok(())
        })
    }
}

fn read_window_id(window: &adw::Window) -> Result<u64> {
    let window_id_ptr = unsafe { window.data::<u64>(WINDOW_ID_KEY) };
    window_id_ptr
        .map(|ptr| unsafe { ptr.read() })
        .context("window ID is not tracked")
}

#[zbus::proxy(
    interface = "io.github.aecsocket.WordbaseIntegration",
    default_service = "io.github.aecsocket.WordbaseIntegration",
    default_path = "/io/github/aecsocket/WordbaseIntegration"
)]
trait Integration {
    async fn get_focused_window_id(&self) -> zbus::Result<u64>;

    async fn get_app_window_id(&self, title: &str) -> zbus::Result<u64>;

    async fn affix_to_window(&self, parent_id: u64, child_id: u64) -> zbus::Result<()>;

    async fn move_to_window(
        &self,
        target_id: u64,
        to_id: u64,
        to_pid: u32,
        to_title: &str,
        to_wm_class: &str,
        offset_x: i32,
        offset_y: i32,
    ) -> zbus::Result<()>;
}
