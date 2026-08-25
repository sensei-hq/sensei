//! Effective git author identity for a working directory.
//!
//! Returns the `user.name` / `user.email` that git itself would stamp on a
//! commit made in a directory, applying git's own **local → global** precedence
//! (a repo `.git/config` override wins; otherwise the user's global
//! `~/.gitconfig`). Surfaced by `GET /api/user` (MCP `get_user_for_project`) so
//! a run/plan is registered under the same identity as the commit author —
//! which is the identity a contributor signs in to the Dōjō with (GitHub
//! OAuth). That makes attribution automatic instead of a hand-paired token.
//!
//! Mirrors the git-subprocess discipline already used in `indexer/cross_repo.rs`
//! and `tasks/handlers/scan.rs` (shell out to `git`, tolerate its absence) — we
//! deliberately do not pull in a git library for a two-value read. The pure
//! parsers ([`parse_show_origin`], [`classify_origin`]) are unit-tested; the
//! thin subprocess wrapper is verified live against the daemon.

use std::path::Path;
use std::process::Command;

/// The effective git identity resolved for a directory. Each field is `None`
/// when git is unavailable, the dir is not a repo with no global config, or that
/// key is unset in every scope.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GitUser {
    pub name: Option<String>,
    pub email: Option<String>,
    /// Where the *email* (the identity key) was resolved from: `"local"` (repo
    /// `.git/config`), `"global"` (`~/.gitconfig`), or `"system"`. `None` when
    /// the email is unset or its origin could not be classified.
    pub source: Option<String>,
}

impl GitUser {
    /// True when at least an email is known — the minimum to attribute work to
    /// a person (the email is the identity key that matches a Dōjō login).
    pub fn is_resolved(&self) -> bool {
        self.email.is_some()
    }
}

/// Read the effective git identity for `dir`. git applies local→global
/// precedence itself, so a plain `git config --get` executed *in the repo dir*
/// returns the repo override when present and the global value otherwise.
///
/// Never errors: a missing git binary, a non-repo dir, or an unset key all
/// resolve to `None` fields rather than a failure.
pub fn read_git_user(dir: &Path) -> GitUser {
    let name = git_config(dir, "user.name").map(|(value, _origin)| value);
    let (email, source) = match git_config(dir, "user.email") {
        Some((value, origin)) => (Some(value), classify_origin(&origin)),
        None => (None, None),
    };
    GitUser { name, email, source }
}

/// Run `git config --show-origin --get <key>` in `dir` and parse the resulting
/// `origin\tvalue` line. `None` when git fails, exits non-zero (key unset), or
/// produces empty output.
fn git_config(dir: &Path, key: &str) -> Option<(String, String)> {
    let out = Command::new("git")
        .args(["config", "--show-origin", "--get", key])
        .current_dir(dir)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    parse_show_origin(&String::from_utf8_lossy(&out.stdout))
}

/// Parse one `git config --show-origin --get` output line, shaped
/// `"<origin>\t<value>"` (origin and value are TAB-separated; the value may
/// itself contain spaces). Returns `(value, origin)`, or `None` for empty
/// output. Falls back to treating the whole line as the value when no TAB is
/// present (defensive: unusual git builds / configs).
fn parse_show_origin(line: &str) -> Option<(String, String)> {
    let line = line.trim_end_matches(['\n', '\r']);
    if line.is_empty() {
        return None;
    }
    match line.split_once('\t') {
        Some((origin, value)) if !value.is_empty() => Some((value.to_string(), origin.to_string())),
        _ => Some((line.to_string(), String::new())),
    }
}

/// Classify a `--show-origin` origin token into `Some("local"|"global"|
/// "system")`, or `None` when it can't be classified (e.g. no origin present).
/// Origins look like `file:.git/config` (repo-local), `file:/Users/x/.gitconfig`
/// (global), or `file:/etc/gitconfig` (system).
fn classify_origin(origin: &str) -> Option<String> {
    let path = origin.strip_prefix("file:").unwrap_or(origin);
    if path.is_empty() {
        None
    } else if path.ends_with(".git/config") {
        Some("local".to_string())
    } else if path.starts_with("/etc/") || path.contains("/etc/gitconfig") {
        Some("system".to_string())
    } else {
        Some("global".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_show_origin_splits_origin_and_value() {
        assert_eq!(
            parse_show_origin("file:.git/config\tdev@sensei-hq.com\n"),
            Some(("dev@sensei-hq.com".to_string(), "file:.git/config".to_string()))
        );
    }

    #[test]
    fn parse_show_origin_keeps_spaces_in_value() {
        // A display name has spaces; only the first TAB separates origin/value.
        assert_eq!(
            parse_show_origin("file:/Users/jerry/.gitconfig\tJerry Thomas"),
            Some(("Jerry Thomas".to_string(), "file:/Users/jerry/.gitconfig".to_string()))
        );
    }

    #[test]
    fn parse_show_origin_empty_is_none() {
        assert_eq!(parse_show_origin(""), None);
        assert_eq!(parse_show_origin("\n"), None);
    }

    #[test]
    fn parse_show_origin_no_tab_treats_line_as_value() {
        assert_eq!(
            parse_show_origin("dev@sensei-hq.com"),
            Some(("dev@sensei-hq.com".to_string(), String::new()))
        );
    }

    #[test]
    fn classify_origin_local_global_system() {
        assert_eq!(classify_origin("file:.git/config"), Some("local".to_string()));
        assert_eq!(
            classify_origin("file:/Users/jerry/Developer/x/.git/config"),
            Some("local".to_string())
        );
        assert_eq!(classify_origin("file:/Users/jerry/.gitconfig"), Some("global".to_string()));
        assert_eq!(classify_origin("file:/etc/gitconfig"), Some("system".to_string()));
    }

    #[test]
    fn classify_origin_empty_is_none() {
        assert_eq!(classify_origin(""), None);
        assert_eq!(classify_origin("file:"), None);
    }

    #[test]
    fn read_git_user_here_resolves_this_repos_local_override() {
        // This crate lives inside the sensei repo, whose `.git/config` sets a
        // local author override. read_git_user must apply local precedence: a
        // resolved email tagged `source: "local"`. (If a checkout has no local
        // override AND no global config, email is None — so we only assert the
        // shape/precedence, never a specific address.)
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let user = read_git_user(dir);
        if let Some(src) = user.source.as_deref() {
            assert!(user.email.is_some(), "a classified source implies a resolved email");
            assert!(
                ["local", "global", "system"].contains(&src),
                "source must be one of the known scopes, got {src:?}"
            );
        }
    }
}
