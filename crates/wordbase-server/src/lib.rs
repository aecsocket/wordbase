#![doc = include_str!("../README.md")]

use {anyhow::Result, futures::never::Never, std::net::SocketAddr, wordbase_engine::Engine};

#[derive(Debug, Clone)]
pub struct Config {
    pub listen_addr: SocketAddr,
}

pub async fn run(engine: Engine, config: &Config) -> Result<Never> {
    loop {}
}
