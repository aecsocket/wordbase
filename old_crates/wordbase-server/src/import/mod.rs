mod yomitan;

use {
    crate::Config,
    derive_more::{Display, Error},
    sqlx::{Pool, Sqlite},
    std::sync::Arc,
    tokio::sync::{Mutex, Semaphore, mpsc},
    wordbase::DictionaryState,
};

#[derive(Debug, Clone)]
pub struct Imports {
    db: Pool<Sqlite>,
    concurrency: Arc<Semaphore>,
    insert_lock: Arc<Mutex<()>>,
}

impl Imports {
    pub fn new(db: Pool<Sqlite>, config: &Config) -> Self {
        Self {
            db,
            concurrency: Arc::new(Semaphore::new(
                usize::try_from(config.max_concurrent_imports.get()).unwrap_or(usize::MAX),
            )),
            insert_lock: Arc::default(),
        }
    }
}
