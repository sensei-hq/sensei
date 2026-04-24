pub mod pg_store;

// Legacy SQLite store — kept temporarily for indexer modules that haven't been migrated yet.
// TODO: Remove once lib_indexer, cross_repo, llms_indexer are migrated to PgStore.
#[allow(dead_code)]
pub mod store;
#[allow(dead_code)]
mod entities;
#[allow(dead_code)]
pub use store::Store;
