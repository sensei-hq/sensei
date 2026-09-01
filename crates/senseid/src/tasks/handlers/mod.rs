//! Task handlers — one function per TaskKind.
//!
//! Split by pipeline phase: scan, process, resolve, libraries, helpers.

pub(crate) mod advance_run;
mod analyze;
mod community;
mod consolidate;
mod consolidate_governance;
mod corrections;
mod corrections_llm;
mod doc_drift;
mod embed;
mod generate;
pub(crate) mod helpers;
mod learn_playbooks;
mod libraries;
pub(crate) mod metrics;
mod model_insight;
mod process;
mod prompt_classify;
mod publish_run;
mod publish_segments;
mod rank;
pub(crate) mod scan;
pub(crate) mod scan_logic;
pub(crate) mod session_process;
mod session_retro;
pub(crate) mod tool_insights;
mod verdicts;
mod verdicts_classify;
mod warm_narration_cache;

pub use analyze::analyze_project;
pub use community::detect_communities;
pub use consolidate_governance::consolidate_governance;
pub use corrections::aggregate_corrections;
pub use doc_drift::scan_doc_drift;
pub use embed::embed_nodes;
pub use libraries::{extract_deps, import_lib, index_library, index_library_page, resolve_libs};
pub use process::{
    delete_file, delete_folder, process_file, process_folder, process_git_folder,
    reconcile_repo_metadata,
};
pub use publish_run::publish_run;
pub use publish_segments::publish_relay_segments;
pub use scan::{branch_switch, scan_root};
pub use tool_insights::aggregate_tool_insights;
pub use verdicts::measure_verdicts;
pub use verdicts_classify::classify_pending_verdicts;
// Shared with the run-nudge API handler (get_pending_nudges): membership
// resolution is identical to the federation task's, so it lives in one place.
pub use advance_run::advance_run;
pub use learn_playbooks::learn_playbooks;
pub(crate) use publish_run::resolve_run_memberships;
pub use session_process::analyze_session_process;
pub use warm_narration_cache::warm_narration_cache;
