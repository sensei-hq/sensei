//! Task handlers — one function per TaskKind.
//!
//! Split by pipeline phase: scan, process, resolve, libraries, helpers.

mod scan;
pub(crate) mod scan_logic;
mod process;
mod resolve;
mod libraries;
mod community;
pub(crate) mod helpers;

pub use scan::{scan_root, branch_switch};
pub use process::{process_git_folder, process_folder, process_file, delete_file, delete_folder};
pub use resolve::{resolve_edges, build_connections, reconcile_connections};
pub use libraries::{resolve_libs, import_lib, index_library, index_library_page};
pub use community::detect_communities;
