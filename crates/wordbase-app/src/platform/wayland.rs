use std::pin::Pin;

use anyhow::{Result, bail};

pub struct Client;

pub fn new() -> Client {
    Client
}

impl super::Platform for Client {
    fn affix_to_focused_window(
        &self,
        _overlay: &adw::ApplicationWindow,
    ) -> Pin<Box<dyn Future<Output = Result<()>>>> {
        Box::pin(async move {
            bail!("unsupported");
        })
    }
}
