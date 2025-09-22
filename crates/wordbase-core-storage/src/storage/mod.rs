//! [`wordbase_core::storage::Storage`] implementations.

pub use wordbase_core::storage::*;

#[cfg(feature = "storage-heed")]
pub mod heed;
#[cfg(feature = "storage-heed")]
pub use heed::Heed;

#[cfg(feature = "storage-libsql")]
pub mod libsql;
#[cfg(feature = "storage-libsql")]
pub use libsql::Libsql;

#[cfg(feature = "storage-redb")]
pub mod redb;
#[cfg(feature = "storage-redb")]
pub use redb::Redb;

#[cfg(feature = "storage-rocksdb")]
pub mod rocksdb;
#[cfg(feature = "storage-rocksdb")]
pub use rocksdb::Rocksdb;

// #[cfg(feature = "storage-turso")]
// pub mod turso;
// #[cfg(feature = "storage-turso")]
// pub use turso::Turso;
