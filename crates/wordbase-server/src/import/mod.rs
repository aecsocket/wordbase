mod yomitan;

use {
    crate::Config,
    derive_more::{Display, Error},
    sqlx::{Pool, Sqlite},
    std::sync::Arc,
    tokio::sync::{Mutex, Semaphore, mpsc},
    wordbase::DictionaryMeta,
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

/// Failed to import a dictionary.
#[derive(Debug, Clone, Display, Error)]
pub enum ImportError {
    /// Dictionary with this name already exists.
    #[display("already exists")]
    AlreadyExists,
    /// Dictionary was parsed, but it had no records to insert into the
    /// database.
    #[display("no records to insert")]
    NoRecords,
}

/// Tracks the state of a dictionary import operation.
#[derive(Debug)]
pub struct ImportTracker {
    /// Parsed dictionary meta.
    pub meta: DictionaryMeta,
    /// Channel receiver for the progress of the import operation.
    ///
    /// The progress value is between 0.0 and 1.0, and is entirely opaque to
    /// users. Implementations are free to display import progress in whatever
    /// way they want.
    pub recv_frac_done: mpsc::Receiver<f64>,
}
