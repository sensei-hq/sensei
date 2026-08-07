//! Task handlers — one function per TaskKind.
//!
//! Split by pipeline phase: scan, process, resolve, libraries, helpers.

pub(crate) mod scan;
pub(crate) mod scan_logic;
mod process;
mod resolve;
mod embed;
mod libraries;
mod community;
mod verdicts;
mod verdicts_classify;
mod analyze;
mod session_retro;
mod doc_drift;
mod corrections;
mod generate;
mod consolidate;
mod consolidate_governance;
mod publish_segments;
mod publish_run;
pub(crate) mod advance_run;
mod warm_insight_copy;
mod rank;
mod model_insight;
mod corrections_llm;
mod prompt_classify;
pub(crate) mod tool_insights;
mod learn_playbooks;
pub(crate) mod helpers;

pub use scan::{scan_root, branch_switch};
pub use process::{process_git_folder, process_folder, process_file, delete_file, delete_folder, reconcile_identity};
pub use resolve::build_connections;
pub use embed::embed_nodes;
pub use libraries::{resolve_libs, import_lib, index_library, index_library_page, extract_deps};
pub use community::detect_communities;
pub use verdicts::measure_verdicts;
pub use verdicts_classify::classify_pending_verdicts;
pub use analyze::analyze_project;
pub use doc_drift::scan_doc_drift;
pub use corrections::aggregate_corrections;
pub use tool_insights::aggregate_tool_insights;
pub use consolidate_governance::consolidate_governance;
pub use publish_segments::publish_relay_segments;
pub use publish_run::publish_run;
// Shared with the run-nudge API handler (get_pending_nudges): membership
// resolution is identical to the federation task's, so it lives in one place.
pub(crate) use publish_run::resolve_run_memberships;
pub use advance_run::advance_run;
pub use warm_insight_copy::warm_insight_copy;
pub use learn_playbooks::learn_playbooks;
