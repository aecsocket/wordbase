use anyhow::Result;

pub trait Client {
    fn stick_to_focused_window(&self, overlay: &adw::ApplicationWindow) -> Result<()>;
}
