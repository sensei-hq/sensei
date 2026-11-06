//! Forge keys and links (GitLab now, GitHub next; spec §G6). Pure: no endpoint, no
//! auth. A forge key is `<path>#<n>`: `acme/platform#12`, its path lowercased.
//!
//! - A **full reference** `<path>#<n>` matches anywhere, in any session.
//! - A **bare** `#<n>` in a prompt, or a branch starting `<n>-`, resolves only to a
//!   task whose scope is the session's own `repo_key`; a NULL `repo_key` resolves
//!   nothing.
//! - A path with an empty, `.` or `..` segment is not a key.

use crate::tracker::{KnownTask, Link, LinkSource, SessionText};

/// A path character: `[A-Za-z0-9_.\-/]`.
fn is_path_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-' | b'/')
}

/// Whether `path` is a usable key path: path characters only, and no empty, `.` or
/// `..` segment.
pub fn is_key_path(path: &str) -> bool {
    !path.is_empty()
        && path.bytes().all(is_path_byte)
        && path.split('/').all(|seg| !seg.is_empty() && seg != "." && seg != "..")
}

/// `(lowercased path, n)` of a full forge key, or `None` when `key` is not one.
pub fn parse_key(key: &str) -> Option<(String, u64)> {
    let (path, digits) = key.rsplit_once('#')?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) || !is_key_path(path) {
        return None;
    }
    Some((path.to_ascii_lowercase(), digits.parse().ok()?))
}

/// One `#` reference found in text: its path (empty for a bare `#n`), its number,
/// and the text as found.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Found {
    path: String,
    n: u64,
    text: String,
}

/// Every `#<n>` in `text`: a path is the run of path characters right before `#`
/// (so `acme/platform#12` is one reference, never a bare `#12` too); the digits
/// must not be followed by a digit.
fn references(text: &str) -> Vec<Found> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    for (i, _) in b.iter().enumerate().filter(|(_, c)| **c == b'#') {
        let end =
            b[i + 1..].iter().position(|c| !c.is_ascii_digit()).map_or(b.len(), |p| i + 1 + p);
        let Ok(n) = text[i + 1..end].parse::<u64>() else {
            continue; // no digits, or too many to be an iid
        };
        let start = b[..i].iter().rposition(|c| !is_path_byte(*c)).map_or(0, |p| p + 1);
        out.push(Found { path: text[start..i].to_string(), n, text: text[start..end].to_string() });
    }
    out
}

/// The `<n>` of a branch named `<n>-…`.
fn branch_number(branch: &str) -> Option<u64> {
    let (digits, _) = branch.split_once('-')?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// The task's iid when its key is a forge key.
fn iid(key: &str) -> Option<u64> {
    parse_key(key).map(|(_, n)| n)
}

/// The forge `match_links` (§G6).
pub fn match_links(session: &SessionText, known: &[KnownTask]) -> Vec<Link> {
    use std::collections::HashMap;
    let mut by_key: HashMap<&str, Vec<&KnownTask>> = HashMap::new();
    for k in known {
        by_key.entry(k.key.as_str()).or_default().push(k);
    }
    // A bare key resolves only within the session's own repo (§G6, C12).
    let mut in_repo: HashMap<u64, Vec<&KnownTask>> = HashMap::new();
    if let Some(repo) = session.repo_key.as_deref() {
        for k in known.iter().filter(|k| k.scope.as_deref() == Some(repo)) {
            if let Some(n) = iid(&k.key) {
                in_repo.entry(n).or_default().push(k);
            }
        }
    }
    let tasks_in = |text: &str, source: LinkSource| -> Vec<(&str, String)> {
        let mut found: Vec<(&str, String)> = Vec::new();
        for r in references(text) {
            let tasks = if r.path.is_empty() {
                // A bare `#n` counts in prompts; a branch names its number as `<n>-`.
                match source {
                    LinkSource::Prompt => in_repo.get(&r.n),
                    _ => None,
                }
            } else if is_key_path(&r.path) {
                by_key.get(format!("{}#{}", r.path.to_ascii_lowercase(), r.n).as_str())
            } else {
                None
            };
            found.extend(
                tasks.into_iter().flatten().map(|t| (t.external_id.as_str(), r.text.clone())),
            );
        }
        if source == LinkSource::Branch
            && let Some(n) = branch_number(text)
        {
            found.extend(
                in_repo
                    .get(&n)
                    .into_iter()
                    .flatten()
                    .map(|t| (t.external_id.as_str(), format!("#{n}"))),
            );
        }
        found
    };
    crate::tracker::link::tally_links(session, known, tasks_in)
}

/// The forge `unknown_keys` (§G6, R11): full references only, lowercased, whose path is
/// already a path on the board, that match no known task. Sorted, distinct.
pub fn unknown_keys(session: &SessionText, known: &[KnownTask]) -> Vec<String> {
    use std::collections::{BTreeSet, HashSet};
    let known_keys: HashSet<&str> = known.iter().map(|k| k.key.as_str()).collect();
    let board_paths: HashSet<String> =
        known.iter().filter_map(|k| parse_key(&k.key)).map(|(p, _)| p).collect();
    let mut out = BTreeSet::new();
    for text in session.branches.iter().chain(&session.prompts) {
        for r in references(text) {
            if r.path.is_empty() || !is_key_path(&r.path) {
                continue;
            }
            let path = r.path.to_ascii_lowercase();
            let key = format!("{path}#{}", r.n);
            if board_paths.contains(&path) && !known_keys.contains(key.as_str()) {
                out.insert(key);
            }
        }
    }
    out.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(id: &str, key: &str, scope: &str) -> KnownTask {
        KnownTask {
            external_id: id.into(),
            key: key.into(),
            scope: Some(scope.into()),
            current: true,
        }
    }

    const REPO_A: &str = "gitlab.com/acme/platform";
    const REPO_B: &str = "gitlab.com/acme/core";

    fn two_repos() -> Vec<KnownTask> {
        vec![
            task("1", "acme/platform#12", REPO_A),
            task("2", "acme/core#12", REPO_B),
            task("3", "acme/platform#123", REPO_A),
        ]
    }

    fn session(repo: Option<&str>, branches: &[&str], prompts: &[&str]) -> SessionText {
        SessionText {
            session_id: uuid::Uuid::nil(),
            branches: branches.iter().map(|b| b.to_string()).collect(),
            prompts: prompts.iter().map(|p| p.to_string()).collect(),
            repo_key: repo.map(String::from),
        }
    }

    fn linked(links: &[Link]) -> Vec<(&str, LinkSource, &str, u32)> {
        let mut v: Vec<_> = links
            .iter()
            .map(|l| (l.task_external_id.as_str(), l.source, l.matched_key.as_str(), l.mentions))
            .collect();
        v.sort();
        v
    }

    #[test]
    fn a_bare_key_links_only_the_sessions_own_repo() {
        let links = match_links(&session(Some(REPO_A), &[], &["look at #12"]), &two_repos());
        assert_eq!(linked(&links), [("1", LinkSource::Prompt, "#12", 1)]);
    }

    #[test]
    fn without_a_repo_a_bare_key_links_nothing() {
        assert!(match_links(&session(None, &["12-fix"], &["#12"]), &two_repos()).is_empty());
    }

    #[test]
    fn a_full_reference_links_in_any_session_and_counts_once() {
        let links = match_links(&session(None, &[], &["fix Acme/Platform#12 now"]), &two_repos());
        assert_eq!(linked(&links), [("1", LinkSource::Prompt, "Acme/Platform#12", 1)]);
        let links =
            match_links(&session(Some(REPO_A), &[], &["fix acme/platform#12."]), &two_repos());
        assert_eq!(linked(&links), [("1", LinkSource::Prompt, "acme/platform#12", 1)]);
    }

    #[test]
    fn a_longer_number_never_matches_a_shorter_one() {
        let links = match_links(&session(Some(REPO_A), &[], &["#123"]), &two_repos());
        assert_eq!(linked(&links), [("3", LinkSource::Prompt, "#123", 1)]);
        let links = match_links(&session(None, &[], &["acme/platform#1234"]), &two_repos());
        assert!(links.is_empty());
    }

    #[test]
    fn a_branch_starting_with_the_number_links_in_its_repo() {
        let links = match_links(&session(Some(REPO_A), &["12-fix-login"], &[]), &two_repos());
        assert_eq!(linked(&links), [("1", LinkSource::Branch, "#12", 1)]);
        let links = match_links(&session(Some(REPO_A), &["fix-12", "123x-y"], &[]), &two_repos());
        assert!(links.is_empty(), "only a leading `<n>-` counts");
    }

    #[test]
    fn a_full_reference_in_a_branch_links_too() {
        let links = match_links(&session(Some(REPO_B), &["acme/platform#12"], &[]), &two_repos());
        assert_eq!(linked(&links), [("1", LinkSource::Branch, "acme/platform#12", 1)]);
        // The path is every path character before `#` (§G6), so a prefix changes it.
        let links =
            match_links(&session(Some(REPO_B), &["fix/acme/platform#12"], &[]), &two_repos());
        assert!(links.is_empty());
    }

    #[test]
    fn mentions_count_every_occurrence_in_prompts() {
        let links = match_links(
            &session(Some(REPO_A), &[], &["#12 and #12", "again acme/platform#12"]),
            &two_repos(),
        );
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].mentions, 3);
    }

    #[test]
    fn a_path_with_an_empty_dot_or_dotdot_segment_is_not_a_key() {
        for bad in ["acme//platform#12", "acme/./platform#12", "acme/../platform#12", "/acme#1"] {
            assert_eq!(parse_key(bad), None, "{bad}");
            assert!(match_links(&session(None, &[], &[bad]), &two_repos()).is_empty(), "{bad}");
        }
        assert_eq!(parse_key("Acme/Platform#12"), Some(("acme/platform".into(), 12)));
        assert_eq!(parse_key("acme/platform#"), None);
        assert_eq!(parse_key("acme/platform#1x"), None);
        assert_eq!(parse_key("#12"), None, "a bare key is not a full key");
    }

    #[test]
    fn unknown_keys_are_full_references_on_a_board_path_only() {
        let s = session(
            Some(REPO_A),
            &["99-new"],
            &["#99, Acme/Platform#99, acme/platform#12, other/repo#5, acme/core#7, acme/./core#8"],
        );
        assert_eq!(unknown_keys(&s, &two_repos()), ["acme/core#7", "acme/platform#99"]);
    }
}
