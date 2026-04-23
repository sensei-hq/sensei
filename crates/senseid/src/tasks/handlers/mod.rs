//! Task handlers — one function per TaskKind.
//!
//! Split by pipeline phase: scan, process, resolve, libraries, helpers.

mod scan;
mod process;
mod resolve;
mod libraries;
pub(crate) mod helpers;

pub use scan::{scan_root, branch_switch};
pub use process::{process_repo, process_folder, process_file, delete_file, delete_folder};
pub use resolve::{resolve_edges, build_connections, reconcile_connections};
pub use libraries::{resolve_libs, import_lib};
