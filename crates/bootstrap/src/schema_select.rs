// Shared by build.rs and schema.rs (via `include!`), so the files the build
// embeds and the files the test expects come from one rule.

/// Whether a git-tracked path under `database/` (given relative to it) ships
/// in the binary. Tests, the DBML render and backups are development material.
#[allow(dead_code)]
fn is_shipped(rel: &str) -> bool {
    !(rel.starts_with("tests/")
        || rel.starts_with("backup/")
        || rel == "design.dbml"
        || rel == ".gitignore")
}
