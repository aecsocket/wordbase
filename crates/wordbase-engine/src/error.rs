use {
    derive_more::{Display, Error},
    eyre::WrapErr,
    std::{
        convert::Infallible,
        fmt::{Debug, Display},
    },
};

#[derive(Debug, Display, Error)]
pub enum Error {
    #[display("internal error")]
    Internal(eyre::Report),
    #[display("bad request")]
    Request(eyre::Report),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

pub trait Context<T, E>: Sized {
    fn wrap_err_with<D>(self, f: impl FnOnce() -> D) -> eyre::Result<T>
    where
        D: Debug + Display + Send + Sync + 'static;

    fn wrap_err<D>(self, msg: D) -> eyre::Result<T>
    where
        D: Debug + Display + Send + Sync + 'static,
    {
        self.wrap_err_with(|| msg)
    }

    fn wrap_internal_err_with<D>(self, f: impl FnOnce() -> D) -> Result<T>
    where
        D: Debug + Display + Send + Sync + 'static,
    {
        self.wrap_err_with(f).map_err(Error::Internal)
    }

    fn wrap_internal_err<D>(self, msg: D) -> Result<T>
    where
        D: Debug + Display + Send + Sync + 'static,
    {
        self.wrap_internal_err_with(|| msg)
    }

    fn wrap_request_err_with<D>(self, f: impl FnOnce() -> D) -> Result<T>
    where
        D: Debug + Display + Send + Sync + 'static,
    {
        self.wrap_err_with(f).map_err(Error::Request)
    }

    fn wrap_request_err<D>(self, msg: D) -> Result<T>
    where
        D: Debug + Display + Send + Sync + 'static,
    {
        self.wrap_request_err_with(|| msg)
    }
}

impl<T, E> Context<T, E> for Result<T, E>
where
    Self: WrapErr<T, E> + Sized,
{
    fn wrap_err_with<D>(self, f: impl FnOnce() -> D) -> eyre::Result<T>
    where
        D: Debug + Display + Send + Sync + 'static,
    {
        WrapErr::wrap_err_with(self, f)
    }
}

impl<T> Context<T, Infallible> for Option<T> {
    fn wrap_err_with<D>(self, f: impl FnOnce() -> D) -> eyre::Result<T>
    where
        D: Debug + Display + Send + Sync + 'static,
    {
        self.ok_or_else(|| eyre::Report::msg(f()))
    }
}
