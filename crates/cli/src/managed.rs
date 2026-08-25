//! Managed governance directive (workstream A).
//!
//! The persistent "load governance first" instruction `sensei init` writes into a
//! project's `CLAUDE.md` / `AGENTS.md`. It is a *pull-first* directive — a durable
//! fallback to the SessionStart/PreCompact hooks (which only push for Claude Code),
//! and the only governance signal non-Claude assistants (Cursor/Codex/Copilot) get.
//!
//! Idempotent + marker-delimited: re-running `init` replaces only the block between
//! the markers and never touches content outside them.

use std::fs;
use std::path::Path;

pub const MANAGED_START: &str = "<!-- sensei:managed:start -->";
pub const MANAGED_END: &str = "<!-- sensei:managed:end -->";

/// The full managed block (markers included) that sensei owns.
pub fn directive_block() -> String {
    format!(
        "{MANAGED_START}\n\
## Sensei — load governance first (managed; edits between the markers are overwritten by `sensei init`)\n\
\n\
On session start AND after any context compaction, BEFORE other work:\n\
\n\
1. Call `get_rules()` (this repo's governance) and `get_layered_context()` (blended memory).\n\
2. Treat **mandatory** rules as non-negotiable — a narrower scope cannot override them.\n\
3. If rules conflict, list the conflict and ask which one wins before proceeding.\n\
4. Then proceed with the task.\n\
\n\
(The sensei plugin's SessionStart / PreCompact hooks also push these for Claude Code; this\n\
directive is the durable fallback and covers non-Claude assistants.)\n\
{MANAGED_END}"
    )
}

/// Upsert the managed `block` into a file's existing text (`None` = new file),
/// returning the new full contents. Replaces an in-place marked block, appends to a
/// file with no block, or creates one when absent. Content OUTSIDE the markers is
/// never altered. A malformed pair (markers missing or out of order) is treated as
/// "no block" and the block is appended.
pub fn upsert(existing: Option<&str>, block: &str) -> String {
    let Some(text) = existing else {
        return format!("{block}\n");
    };
    if text.trim().is_empty() {
        return format!("{block}\n");
    }
    match (text.find(MANAGED_START), text.find(MANAGED_END)) {
        (Some(s), Some(e)) if e > s => {
            let end = e + MANAGED_END.len();
            format!("{}{}{}", &text[..s], block, &text[end..])
        }
        _ => {
            let sep = if text.ends_with('\n') { "\n" } else { "\n\n" };
            format!("{text}{sep}{block}\n")
        }
    }
}

/// What `write_directive` did to a file.
#[derive(Debug, PartialEq, Eq)]
pub enum Change {
    Created,
    Updated,
    Unchanged,
}

/// Write/refresh the managed directive in `path`. No-op (Unchanged) when the current
/// block is already identical, so re-running `init` is cheap and stable.
pub fn write_directive(path: &Path) -> std::io::Result<Change> {
    // `path` is program-derived (init: `current_dir().join("CLAUDE.md")`), never untrusted
    // or HTTP input — the Actix web path-traversal rule is a false positive in this CLI crate.
    let existing = fs::read_to_string(path).ok(); // nosemgrep
    let block = directive_block();
    if let Some(text) = existing.as_deref()
        && text.contains(&block)
    {
        return Ok(Change::Unchanged);
    }
    let new = upsert(existing.as_deref(), &block);
    fs::write(path, new)?; // nosemgrep: same program-derived path — see note above
    Ok(if existing.is_some() { Change::Updated } else { Change::Created })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_block_for_a_new_file() {
        let out = upsert(None, &directive_block());
        assert!(out.starts_with(MANAGED_START));
        assert!(out.trim_end().ends_with(MANAGED_END));
        assert!(out.contains("get_rules()") && out.contains("get_layered_context()"));
    }

    #[test]
    fn appends_to_a_file_with_no_block_preserving_existing_content() {
        let existing = "# My Project\n\nSome notes.\n";
        let out = upsert(Some(existing), &directive_block());
        assert!(out.starts_with("# My Project"), "existing content is preserved at the top");
        assert!(out.contains(MANAGED_START) && out.contains(MANAGED_END));
    }

    #[test]
    fn replaces_the_block_in_place_and_keeps_surrounding_text() {
        let existing = format!(
            "# Head\n\nbefore text\n\n{MANAGED_START}\nOLD BODY\n{MANAGED_END}\n\nafter text\n"
        );
        let out = upsert(Some(&existing), &directive_block());
        assert!(
            out.contains("# Head") && out.contains("before text") && out.contains("after text"),
            "surrounding text on both sides is preserved"
        );
        assert!(!out.contains("OLD BODY"), "the old managed body is replaced");
        assert!(out.contains("get_rules()"), "the new directive is present");
        assert_eq!(out.matches(MANAGED_START).count(), 1, "exactly one block — no duplication");
    }

    #[test]
    fn is_idempotent_running_twice_is_stable() {
        let once = upsert(Some("# Doc\n"), &directive_block());
        let twice = upsert(Some(&once), &directive_block());
        assert_eq!(once, twice, "re-applying the block yields identical content");
        assert_eq!(twice.matches(MANAGED_START).count(), 1);
    }

    #[test]
    fn write_directive_creates_then_noops_then_preserves_surrounding() {
        let path =
            std::env::temp_dir().join(format!("sensei_managed_test_{}.md", std::process::id()));
        let _ = std::fs::remove_file(&path);
        assert_eq!(write_directive(&path).unwrap(), Change::Created);
        assert_eq!(write_directive(&path).unwrap(), Change::Unchanged, "a re-run is a no-op");
        // Wrap the file with surrounding content; the block is still present + identical.
        let body = std::fs::read_to_string(&path).unwrap();
        std::fs::write(&path, format!("# Preamble\n\n{body}\n## Tail\n")).unwrap();
        assert_eq!(write_directive(&path).unwrap(), Change::Unchanged);
        let after = std::fs::read_to_string(&path).unwrap();
        assert!(
            after.contains("# Preamble") && after.contains("## Tail"),
            "surrounding content kept"
        );
        assert_eq!(after.matches(MANAGED_START).count(), 1, "no duplicate block");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn malformed_markers_out_of_order_append_rather_than_corrupt() {
        // END before START (corrupted) → treated as no-block, appended.
        let existing = format!("body {MANAGED_END} ... {MANAGED_START} tail\n");
        let out = upsert(Some(&existing), &directive_block());
        assert!(out.contains("body"), "original text kept");
        assert!(out.trim_end().ends_with(MANAGED_END), "a fresh block is appended at the end");
    }
}
