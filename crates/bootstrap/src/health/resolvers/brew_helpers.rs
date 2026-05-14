//! brew_helpers — shared shell-out + stderr parsing for the brew-based
//! install resolvers (postgres_install, ollama_install, sensei_install).
//!
//! `BrewError` captures the four failure modes we surface to the user via
//! distinct `Remedy` builders in each resolver. `parse_brew_error` is a
//! pure function over the stderr text — extracted for exhaustive unit
//! testing without invoking brew.

use std::path::{Path, PathBuf};
use std::process::Command;
use crate::health::resolver::ResolveOutcome;
use crate::health::types::Remedy;
use crate::util::which_binary;

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
    BrewError::Other(truncate_to_tail(stderr, 500))
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

fn truncate_to_tail(stderr: &str, max_bytes: usize) -> String {
    let mut start = stderr.len().saturating_sub(max_bytes);
    while start < stderr.len() && !stderr.is_char_boundary(start) {
        start += 1;
    }
    stderr[start..].to_string()
}

/// Run `brew install <args>... <formula>` and translate failure modes
/// into typed `BrewError` variants.
///
/// On success, returns `Ok(())`. On any non-zero exit, parses stderr.
/// If `brew` isn't on PATH, returns `BrewError::BrewNotFound` without
/// invoking anything.
pub fn brew_install(formula: &str, args: &[&str]) -> Result<(), BrewError> {
    let brew = match which_binary("brew") {
        Some(p) => p,
        None    => return Err(BrewError::BrewNotFound),
    };
    let output = Command::new(brew)
        .arg("install")
        .args(args)
        .arg(formula)
        .output()
        .map_err(|e| BrewError::Other(format!("spawn failed: {e}")))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(parse_brew_error(&stderr))
}

pub(crate) fn homebrew_install_remedy() -> Remedy {
    Remedy {
        message: "Homebrew isn't installed. Run the script below to install it, then re-check.".to_string(),
        script:  r#"/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)""#.to_string(),
        url:     Some("https://brew.sh".to_string()),
    }
}

pub(crate) fn overwrite_link_remedy(formula: &str, path: &Path) -> Remedy {
    Remedy {
        message: format!(
            "Couldn't link `{formula}` because `{}` already exists. If you installed it elsewhere (e.g. .dmg or Postgres.app) you can keep that and skip this step. To switch to the brew install, run the script below.",
            path.display(),
        ),
        script:  format!("brew link --overwrite {formula}"),
        url:     None,
    }
}

pub(crate) fn tap_missing_remedy(formula: &str) -> Remedy {
    Remedy {
        message: format!("Couldn't find `{formula}`. Run the script below to add the tap, then re-check."),
        script:  format!("brew tap sensei-hq/tap https://github.com/sensei-hq/homebrew-tap && brew install {formula}"),
        url:     None,
    }
}

pub(crate) fn generic_brew_remedy(formula: &str, stderr_tail: &str) -> Remedy {
    Remedy {
        message: format!(
            "Couldn't install `{formula}` automatically. Last brew output was:\n\n```\n{stderr_tail}\n```\n\nRun the script below to retry."
        ),
        script:  format!("brew install {formula}"),
        url:     None,
    }
}

pub(crate) fn brew_install_to_outcome(formula: &str, args: &[&str]) -> ResolveOutcome {
    match brew_install(formula, args) {
        Ok(())                                => ResolveOutcome::Resolved,
        Err(BrewError::BrewNotFound)          => ResolveOutcome::NeedsHumanAction(homebrew_install_remedy()),
        Err(BrewError::LinkConflict { path }) => ResolveOutcome::NeedsHumanAction(overwrite_link_remedy(formula, &path)),
        Err(BrewError::TapMissing)            => ResolveOutcome::NeedsHumanAction(tap_missing_remedy(formula)),
        Err(BrewError::Other(stderr))         => ResolveOutcome::NeedsHumanAction(generic_brew_remedy(formula, &stderr)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn parses_generic_failure_handles_multibyte_utf8_at_boundary() {
        // Build a >500-byte string where the byte at position len-500 is
        // INSIDE a multi-byte UTF-8 character. '€' is 3 bytes (U+20AC).
        // 250 '€' = 750 bytes. tail_start = 250 lands inside '€' #84
        // (which spans bytes 249..=251), so byte 250 is NOT a char boundary.
        let mut input = String::new();
        for _ in 0..250 {
            input.push('€');
        }
        assert_eq!(input.len(), 750);
        // Naive `&input[250..]` would panic. Our parser must not panic.
        let r = parse_brew_error(&input);
        match r {
            BrewError::Other(s) => {
                // The truncate slid the start forward to the next char boundary.
                assert!(s.len() <= 500);
                assert!(s.starts_with('€'));
            }
            other => panic!("expected Other, got {other:?}"),
        }
    }

    #[test]
    fn homebrew_install_remedy_contains_install_script_and_url() {
        let r = homebrew_install_remedy();
        assert!(r.script.contains("brew.sh") || r.script.contains("Homebrew/install"));
        assert_eq!(r.url.as_deref(), Some("https://brew.sh"));
    }

    #[test]
    fn overwrite_link_remedy_mentions_formula_and_path() {
        let r = overwrite_link_remedy("postgresql@17", &PathBuf::from("/opt/homebrew/bin/psql"));
        assert!(r.message.contains("postgresql@17"));
        assert!(r.message.contains("/opt/homebrew/bin/psql"));
        assert_eq!(r.script, "brew link --overwrite postgresql@17");
    }

    #[test]
    fn tap_missing_remedy_runs_tap_then_install() {
        let r = tap_missing_remedy("postgresql@17");
        assert!(r.script.contains("brew tap"));
        assert!(r.script.contains("brew install postgresql@17"));
    }

    #[test]
    fn generic_brew_remedy_includes_stderr_tail() {
        let r = generic_brew_remedy("postgresql@17", "some failure detail");
        assert!(r.message.contains("some failure detail"));
        assert_eq!(r.script, "brew install postgresql@17");
    }
}
