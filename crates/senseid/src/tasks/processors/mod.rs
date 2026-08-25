//! File processors — modular adapters for different file types.
//!
//! Each processor implements FileAdapter and handles a specific category of files.
//! The router selects the right processor based on file extension/type.

pub mod code;
pub mod config;
pub mod doc;
pub mod metadata;
pub mod router;
pub mod types;

#[cfg(test)]
mod tests;

pub use router::process_file;
#[cfg(test)]
pub use types::FileProcessResult;
