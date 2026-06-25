//! Task handlers — one function per TaskKind.
//!
//! Split by pipeline phase: scan, process, resolve, libraries, helpers.

mod scan;
pub(crate) mod scan_logic;
mod process;
mod resolve;
mod embed;
mod libraries;
mod community;
mod verdicts;
mod analyze;
mod corrections;
mod generate;
mod consolidate;
mod rank;
mod model_insight;
mod corrections_llm;
mod prompt_classify;
pub(crate) mod helpers;

pub use scan::{scan_root, branch_switch};
pub use process::{process_git_folder, process_folder, process_file, delete_file, delete_folder, reconcile_identity};
pub use resolve::{resolve_edges, build_connections, reconcile_connections};
pub use embed::embed_nodes;
pub use libraries::{resolve_libs, import_lib, index_library, index_library_page, extract_deps};
pub use community::detect_communities;
pub use verdicts::measure_verdicts;
pub use analyze::analyze_project;
pub use corrections::aggregate_corrections;
