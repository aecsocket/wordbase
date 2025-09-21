//! [`wordbase_storage_api::backend::Backend`] implementations.

pub use wordbase_storage_api::backend::*;

#[cfg(feature = "backend-heed")]
pub mod heed;
#[cfg(feature = "backend-heed")]
pub use heed::Heed;

#[cfg(feature = "backend-libsql")]
pub mod libsql;
#[cfg(feature = "backend-libsql")]
pub use libsql::Libsql;

#[cfg(feature = "backend-redb")]
pub mod redb;
#[cfg(feature = "backend-redb")]
pub use redb::Redb;

#[cfg(feature = "backend-rocksdb")]
pub mod rocksdb;
#[cfg(feature = "backend-rocksdb")]
pub use rocksdb::Rocksdb;

// #[cfg(feature = "backend-turso")]
// pub mod turso;
// #[cfg(feature = "backend-turso")]
// pub use turso::Turso;
