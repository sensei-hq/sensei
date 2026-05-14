//! brew_helpers — shared shell-out + stderr parsing for the brew-based
//! install resolvers (postgres_install, ollama_install, sensei_install).
//!
//! `BrewError` captures the four failure modes we surface to the user via
//! distinct `Remedy` builders in each resolver. `parse_brew_error` is a
//! pure function over the stderr text — extracted for exhaustive unit
//! testing without invoking brew.

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrewError {
    /// `brew` not on PATH.
    BrewNotFound,
    /// Symlink conflict at `path` (something already exists where brew
    /// wants to link). Destructive remedy — user decides.
    LinkConflict { path: PathBuf },
    /// Formula not found — typically the tap isn't added.
    TapMissing,
    /// Any other brew failure. Carries the last ~500 chars of stderr for
    /// display in the remedy.
    Other(String),
}

pub(crate) fn parse_brew_error(stderr: &str) -> BrewError {
    if stderr.contains("already exists. You may want to remove it") {
        if let Some(path) = extract_target_path(stderr) {
            return BrewError::LinkConflict { path };
        }
    }
    if stderr.contains("No available formula with the name") {
        return BrewError::TapMissing;
    }
    let tail_start = stderr.len().saturating_sub(500);
    BrewError::Other(stderr[tail_start..].to_string())
}

fn extract_target_path(stderr: &str) -> Option<PathBuf> {
    // Look for a line `Target <path>` that brew emits before the
    // "already exists" marker on link conflicts.
    for line in stderr.lines() {
        if let Some(rest) = line.strip_prefix("Target ") {
            return Some(PathBuf::from(rest.trim()));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const OLLAMA_LINK_CONFLICT: &str = r#"Error: Could not symlink bin/ollama
Target /opt/homebrew/bin/ollama
already exists. You may want to remove it:
  rm '/opt/homebrew/bin/ollama'

To force the link and overwrite all conflicting files:
  brew link --overwrite ollama
"#;

    const POSTGRES_LINK_CONFLICT: &str = r#"Error: Could not symlink bin/psql
Target /opt/homebrew/bin/psql
already exists. You may want to remove it:
  rm '/opt/homebrew/bin/psql'
"#;

    const TAP_MISSING: &str = r#"Error: No available formula with the name "sensei-hq/tap/foo".
"#;

    const GENERIC: &str = r#"Error: Cannot install in Homebrew on ARM processor in Intel default prefix (/usr/local)!
Please create a new installation in /opt/homebrew using one of the
"Alternative Installs" from:
  https://docs.brew.sh/Installation
"#;

    #[test]
    fn parses_ollama_link_conflict() {
        match parse_brew_error(OLLAMA_LINK_CONFLICT) {
            BrewError::LinkConflict { path } => {
                assert_eq!(path, PathBuf::from("/opt/homebrew/bin/ollama"));
            }
            other => panic!("expected LinkConflict, got {other:?}"),
        }
    }

    #[test]
    fn parses_postgres_link_conflict() {
        match parse_brew_error(POSTGRES_LINK_CONFLICT) {
            BrewError::LinkConflict { path } => {
                assert_eq!(path, PathBuf::from("/opt/homebrew/bin/psql"));
            }
            other => panic!("expected LinkConflict, got {other:?}"),
        }
    }

    #[test]
    fn parses_tap_missing() {
        assert_eq!(parse_brew_error(TAP_MISSING), BrewError::TapMissing);
    }

    #[test]
    fn parses_generic_failure_truncates_to_500_chars() {
        let long = "x".repeat(2000);
        match parse_brew_error(&long) {
            BrewError::Other(s) => assert_eq!(s.len(), 500),
            other => panic!("expected Other, got {other:?}"),
        }
    }

    #[test]
    fn parses_generic_failure_preserves_short_messages() {
        match parse_brew_error(GENERIC) {
            BrewError::Other(s) => {
                assert!(s.contains("ARM processor"), "kept body: {s}");
                assert!(s.len() <= 500);
            }
            other => panic!("expected Other, got {other:?}"),
        }
    }
}
