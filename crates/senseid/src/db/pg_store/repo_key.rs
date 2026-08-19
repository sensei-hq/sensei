//! Repository-key normalization (spec 2026-08-18, D10/I11).
//!
//! A repository's canonical identity is its NORMALIZED REMOTE, not a local path.
//! [`normalize_repo_key`] collapses every URL form of one remote — SSH
//! (`git@host:Org/Repo.git`), HTTPS (`https://host/Org/Repo`), `ssh://`, `git://`,
//! credentialed/ported URLs — to a single `host/org/repo` key, so two clones, a
//! rename, or a re-checkout all resolve to the SAME `sensei.repositories` row. A
//! remote-less input yields `None` (a local-only repo has no key and is never
//! federated) — NEVER an abs-path fallback (the old D10 leak).

/// Normalize a git remote URL to a stable, host-qualified repository key
/// (`host/org/repo`, lowercased) with scheme, credentials, port and a trailing
/// `.git` stripped. `None` for an empty / unparseable / path-less remote.
pub(crate) fn normalize_repo_key(remote: &str) -> Option<String> {
    let s = remote.trim();
    if s.is_empty() {
        return None;
    }

    // Split into (authority, path). A scheme URL (`scheme://…`) is checked first so
    // its `:` isn't mistaken for the SCP-like separator; otherwise fall back to the
    // SCP-like `[user@]host:path` form (the `:` before the path).
    let (authority, path) = if let Some((_, after_scheme)) = s.split_once("://") {
        after_scheme.split_once('/')? // scheme://[userinfo@]host[:port]/path
    } else {
        s.split_once(':')? // [user@]host:path
    };

    // host: drop any `userinfo@`, then any `:port`.
    let host_port = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    let host = host_port.split(':').next().unwrap_or(host_port);
    if host.is_empty() {
        return None;
    }

    // path: trim surrounding slashes and a trailing `.git`.
    let path = path.trim_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path).trim_matches('/');
    if path.is_empty() {
        return None;
    }

    Some(format!("{host}/{path}").to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssh_and_https_of_one_remote_collapse_to_one_key() {
        let key = Some("github.com/org/repo".to_string());
        assert_eq!(normalize_repo_key("git@github.com:Org/Repo.git"), key, "SCP-like SSH");
        assert_eq!(normalize_repo_key("https://github.com/Org/Repo"), key, "HTTPS, no .git");
        assert_eq!(normalize_repo_key("https://github.com/Org/Repo.git"), key, "HTTPS, .git");
        assert_eq!(normalize_repo_key("ssh://git@github.com/Org/Repo.git"), key, "ssh:// scheme");
        assert_eq!(normalize_repo_key("git://github.com/Org/Repo.git"), key, "git:// scheme");
    }

    #[test]
    fn strips_credentials_and_port() {
        assert_eq!(
            normalize_repo_key("https://user:pass@github.com:443/Org/Repo.git"),
            Some("github.com/org/repo".to_string()),
            "userinfo + port stripped, host lowercased",
        );
    }

    #[test]
    fn keeps_nested_group_path() {
        // GitLab-style nested groups: the whole path after the host is the key.
        assert_eq!(
            normalize_repo_key("git@gitlab.com:Group/Sub/Repo.git"),
            Some("gitlab.com/group/sub/repo".to_string()),
        );
    }

    #[test]
    fn remote_less_or_unparseable_is_none() {
        // A local-only repo (no remote) has no key — never an abs-path fallback.
        assert_eq!(normalize_repo_key(""), None);
        assert_eq!(normalize_repo_key("   "), None);
        assert_eq!(normalize_repo_key("/Users/jerry/Developer/repo"), None, "an abs path is not a remote");
        assert_eq!(normalize_repo_key("https://github.com"), None, "no repo path → None");
        assert_eq!(normalize_repo_key("git@github.com:"), None, "empty path → None");
    }

    #[test]
    fn trims_whitespace() {
        assert_eq!(
            normalize_repo_key("  git@github.com:Org/Repo.git\n"),
            Some("github.com/org/repo".to_string()),
        );
    }
}
