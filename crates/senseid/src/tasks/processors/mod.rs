//! File processors — modular adapters for different file types.
//!
//! Each processor implements FileAdapter and handles a specific category of files.
//! The router selects the right processor based on file extension/type.

pub mod types;
pub mod router;
pub mod code;
pub mod doc;
pub mod config;
pub mod graph_writer;

#[cfg(test)]
mod tests;


#[cfg(test)]
pub use types::{FileProcessResult, SymbolResult, UnresolvedCall, ParentRef};
pub use router::process_file;
pub use graph_writer::write_to_graph;
