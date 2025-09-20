#[cfg(feature = "backend-heed")]
pub mod heed;
#[cfg(feature = "backend-heed")]
pub type Heed = heed::Backend;

#[cfg(feature = "backend-libsql")]
pub mod libsql;
#[cfg(feature = "backend-libsql")]
pub type Libsql = libsql::Backend;

#[cfg(feature = "backend-redb")]
pub mod redb;
#[cfg(feature = "backend-redb")]
pub type Redb = redb::Backend;

#[cfg(feature = "backend-rocksdb")]
pub mod rocksdb;
#[cfg(feature = "backend-rocksdb")]
pub type Rocksdb = rocksdb::Backend;

#[cfg(feature = "backend-turso")]
pub mod turso;
#[cfg(feature = "backend-turso")]
pub type Turso = turso::Backend;
