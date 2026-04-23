//! Repo-level metadata scanners — icon detection, external link extraction, solution matching.
//!
//! Each scanner is a pure function that takes a repo path and returns structured results.
//! Called from process_repo after file discovery, before task enqueuing.

mod icons;
mod external_links;
mod solutions;
mod summary;

// Re-export all public types and functions so callers using
// `crate::tasks::processors::metadata::X` continue to work.
pub use icons::{IconResult, scan_icons};
pub use external_links::{ExternalLinksResult, ExternalLink, scan_external_links};
pub use solutions::{SolutionMatch, suggest_solutions};
pub use summary::{ProjectSummary, extract_summary};
