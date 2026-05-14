//! Repo-level metadata scanners — icon detection, external link extraction, solution matching.
//!
//! Each scanner is a pure function that takes a repo path and returns structured results.
//! Called from process_git_folder after file discovery, before task enqueuing.

mod icons;
mod external_links;
mod solutions;
mod summary;

// Re-export all public types and functions so callers using
// `crate::tasks::processors::metadata::X` continue to work.
pub use icons::scan_icons;
pub use external_links::scan_external_links;
pub use summary::extract_summary;
