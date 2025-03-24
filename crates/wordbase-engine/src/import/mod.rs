mod yomitan;

use {
    derive_more::{Display, Error, From},
    sqlx::{Pool, Sqlite},
    std::sync::Arc,
    tokio::sync::{Mutex, Semaphore, mpsc},
    wordbase::Dictionary,
};

#[derive(Debug, Clone)]
pub struct Imports {
    db: Pool<Sqlite>,
    concurrency: Arc<Semaphore>,
    insert_lock: Arc<Mutex<()>>,
}

impl Imports {
    pub(super) fn new(db: Pool<Sqlite>, max_concurrent_imports: usize) -> Self {
        Self {
            db,
            concurrency: Arc::new(Semaphore::new(max_concurrent_imports)),
            insert_lock: Arc::new(Mutex::new(())),
        }
    }
}

/// Failed to import a dictionary.
#[derive(Debug, Display, Error, From)]
pub enum ImportError {
    /// Dictionary with this name already exists.
    #[display("already exists")]
    AlreadyExists,
    /// Dictionary was parsed, but it had no records to insert into the
    /// database.
    #[display("no records to insert")]
    NoRecords,
    /// Implementation-specific error.
    Other(#[from] anyhow::Error),
}

/// Tracks the state of a dictionary import operation.
#[derive(Debug)]
pub struct ImportTracker {
    /// Parsed dictionary meta.
    pub meta: Dictionary,
    recv_progress: mpsc::Receiver<Result<f64, ImportError>>,
}
