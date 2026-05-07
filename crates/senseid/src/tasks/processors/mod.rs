//! File processors — modular adapters for different file types.
//!
//! Each processor implements FileAdapter and handles a specific category of files.
//! The router selects the right processor based on file extension/type.

pub mod types;
pub mod router;
pub mod code;
pub mod doc;
pub mod config;
pub mod metadata;

#[cfg(test)]
mod tests;


#[cfg(test)]
pub use types::FileProcessResult;
pub use router::process_file;
