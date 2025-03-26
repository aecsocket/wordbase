#![doc = include_str!("../README.md")]

use std::net::SocketAddr;

use anyhow::Result;
use futures::never::Never;
use wordbase_engine::Engine;

#[derive(Debug, Clone)]
pub struct Config {
    pub listen_addr: SocketAddr,
}

pub async fn run(engine: Engine, config: &Config) -> Result<Never> {
    loop {}
}
