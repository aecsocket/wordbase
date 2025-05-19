use {
    super::OverlayGuard,
    crate::CHANNEL_BUF_CAP,
    anyhow::{Context, Result, bail},
    futures::{StreamExt, future::LocalBoxFuture},
    relm4::gtk::{self, prelude::*},
    tokio::sync::broadcast,
    tokio_util::task::AbortOnDropHandle,
    wordbase::WindowFilter,
};

/*
implementation notes:
- `Meta.Window`'s `get_id()` and `get_stable_sequence()` are not for us,
  since we show/hide the `gtk::Window`, which creates/destroys `Meta.Window`s
*/

#[derive(Debug)]
pub struct Platform {
    integration: IntegrationProxy<'static>,
    _overlay_closed_task: AbortOnDropHandle<()>,
    recv_overlay_closed: broadcast::Receiver<u64>,
}

impl Platform {
    pub async fn new() -> Result<Self> {
        let dbus = zbus::Connection::session()
            .await
            .context("failed to establish session bus connection")?;
        let integration = IntegrationProxy::new(&dbus)
            .await
            .context("failed to create integration dbus proxy")?;

        let (send_overlay_closed, recv_overlay_closed) = broadcast::channel(CHANNEL_BUF_CAP);
        let mut overlay_closed_stream = integration
            .receive_close_overlay()
            .await
            .context("failed to start receiving close overlay requests")?;
        let overlay_closed_task = AbortOnDropHandle::new(tokio::spawn(async move {
            while let Some(signal) = overlay_closed_stream.next().await {
                if let Ok(args) = signal.args() {
                    _ = send_overlay_closed.send(args.overlay_id);
                }
            }
        }));

        Ok(Self {
            integration,
            _overlay_closed_task: overlay_closed_task,
            recv_overlay_closed,
        })
    }
}

impl super::Platform for Platform {
    fn init_overlay(&self, overlay: &gtk::Window) -> LocalBoxFuture<Result<OverlayGuard>> {
        struct GuardImpl {
            close_on_request_task: glib::JoinHandle<()>,
        }

        impl Drop for GuardImpl {
            fn drop(&mut self) {
                self.close_on_request_task.abort();
            }
        }

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
                .overlay_on_window(focused_window, window_id)
                .await
                .context("failed to overlay on focused window")?;

            let overlay = overlay.clone();
            let mut recv_overlay_closed = self.recv_overlay_closed.resubscribe();
            let close_on_request_task = glib::spawn_future_local(async move {
                while let Ok(overlay_id) = recv_overlay_closed.recv().await {
                    if overlay_id == window_id {
                        overlay.close();
                    }
                }
            });
            Ok(Box::new(GuardImpl {
                close_on_request_task,
            }) as OverlayGuard)
        })
    }

    fn init_popup(&self, _popup: &gtk::Window) -> LocalBoxFuture<Result<()>> {
        Box::pin(async move { Ok(()) })
    }

    fn move_popup_to_window(
        &self,
        popup: &gtk::Window,
        to: WindowFilter,
        offset_nw: (i32, i32),
        offset_se: (i32, i32),
    ) -> LocalBoxFuture<Result<()>> {
        let popup = popup.clone();
        Box::pin(async move {
            let popup_window_id = get_window_id(&self.integration, &popup).await?;
            self.integration
                .move_popup_to_window(
                    popup_window_id,
                    to.id.unwrap_or_default(),
                    to.title.as_deref().unwrap_or_default(),
                    to.wm_class.as_deref().unwrap_or_default(),
                    offset_nw.0,
                    offset_nw.1,
                    offset_se.0,
                    offset_se.1,
                )
                .await
                .context("failed to request to move popup window")?;
            Ok(())
        })
    }
}

async fn get_window_id(integration: &IntegrationProxy<'_>, window: &gtk::Window) -> Result<u64> {
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

    async fn overlay_on_window(&self, parent_id: u64, overlay_id: u64) -> zbus::Result<()>;

    async fn move_popup_to_window(
        &self,
        moved_id: u64,
        to_id: u64,
        to_title: &str,
        to_wm_class: &str,
        offset_nw_x: i32,
        offset_nw_y: i32,
        offset_se_x: i32,
        offset_se_y: i32,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    fn close_overlay(&self, overlay_id: u64) -> zbus::Result<()>;
}
