use anyhow::{Context, Result};
use futures::future::{BoxFuture, LocalBoxFuture};
use relm4::adw::{self, prelude::*};
use zbus::zvariant::{DeserializeDict, SerializeDict, Type};

use super::WindowFilter;

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
    fn affix_to_focused_window(&self, window: &adw::Window) -> LocalBoxFuture<Result<()>> {
        let window = window.clone();
        Box::pin(async move {
            let title = window.title().context("window has no title")?;
            self.integration
                .affix_to_focused_window(&title)
                .await
                .context("failed to send request to integration")?;
            Ok(())
        })
    }

    fn move_to_window(
        &self,
        window: &adw::Window,
        to: WindowFilter,
        offset: (i32, i32),
    ) -> LocalBoxFuture<Result<()>> {
        todo!();

        // let window_token = rand::random::<WindowToken>();
        // // SAFETY: we will always read this in the extension as a `WindowToken`
        // unsafe {
        //     window.set_data(WORDBASE_WINDOW_TOKEN, window_token);
        // }

        // Box::pin(async move {
        //     self.integration
        //         .move_to_window(window_token, to.into(), offset)
        //         .await
        //         .context("failed to send request to integration")?;
        //     Ok(())
        // })
    }
}

#[zbus::proxy(
    interface = "io.github.aecsocket.WordbaseIntegration",
    default_service = "io.github.aecsocket.WordbaseIntegration",
    default_path = "/io/github/aecsocket/WordbaseIntegration"
)]
trait Integration {
    async fn affix_to_focused_window(&self, target_title: &str) -> zbus::Result<()>;

    async fn move_to_window(
        &self,
        window_token: u64,
        to: WindowFilterSerial,
        offset: (i32, i32),
    ) -> zbus::Result<()>;
}

#[derive(SerializeDict, DeserializeDict, Type)]
#[zvariant(signature = "dict")]
struct WindowFilterSerial {
    pub id: Option<u64>,
    pub pid: Option<u32>,
    pub title: Option<String>,
    pub wm_class: Option<String>,
}

// impl From<WindowFilter> for WindowFilterSerial {
//     fn from(value: WindowFilter) -> Self {
//         Self {
//             id: value.id,
//             pid: value.pid,
//             title: value.title,
//             wm_class: value.wm_class,
//         }
//     }
// }
