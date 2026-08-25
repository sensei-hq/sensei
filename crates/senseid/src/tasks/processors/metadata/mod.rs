//! Repo-level metadata scanners — icon detection, external link extraction, solution matching.
//!
//! Each scanner is a pure function that takes a repo path and returns structured results.
//! Called from process_git_folder after file discovery, before task enqueuing.

pub(crate) mod external_links;
mod frontmatter;
mod icons;
mod solutions;
mod summary;

// Re-export all public types and functions so callers using
// `crate::tasks::processors::metadata::X` continue to work.
pub use external_links::scan_external_links;
pub use frontmatter::{
    Frontmatter, folder_role_from_frontmatter, icon_is_url, merge_frontmatter, read_frontmatter,
    slugify,
};
pub use icons::scan_icons;
pub use summary::extract_summary;
