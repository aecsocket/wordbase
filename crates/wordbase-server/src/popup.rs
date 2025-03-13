use anyhow::Result;
use futures::future::BoxFuture;
use wordbase::protocol::{ShowPopupError, ShowPopupRequest};

pub trait Popups {
    fn show(&self, request: ShowPopupRequest) -> BoxFuture<Result<Result<(), ShowPopupError>>>;

    fn hide(&self) -> BoxFuture<Result<()>>;
}
