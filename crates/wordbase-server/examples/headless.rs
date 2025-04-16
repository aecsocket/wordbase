use anyhow::Result;
use wordbase_engine::Engine;

#[tokio::main]
async fn main() -> Result<()> {
    let engine = Engine::new(wordbase_engine::data_dir()?).await?;
    println!("Running server at http://127.0.0.1:9518/docs");
    wordbase_server::run(engine, "0.0.0.0:9518").await?;
    Ok(())
}
