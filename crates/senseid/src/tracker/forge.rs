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

// ── JSON readers shared by the forge adapters (GitLab, GitHub) ──────────────

/// A string field, or `None`.
pub(crate) fn text<'a>(v: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(|x| x.as_str())
}

/// A `YYYY-MM-DD` field: `None` when absent or null, `Decode` when malformed.
pub(crate) fn date(
    v: &serde_json::Value,
    key: &str,
) -> Result<Option<chrono::NaiveDate>, crate::tracker::TrackerError> {
    use crate::tracker::TrackerError;
    match v.get(key) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(s)) => chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map(Some)
            .map_err(|_| TrackerError::Decode),
        Some(_) => Err(TrackerError::Decode),
    }
}

/// 00:00 UTC of a date.
pub(crate) fn midnight(d: chrono::NaiveDate) -> chrono::DateTime<chrono::Utc> {
    d.and_hms_opt(0, 0, 0).expect("midnight exists").and_utc()
}

/// An RFC 3339 timestamp field: `None` when absent or null, `Decode` when malformed.
pub(crate) fn timestamp(
    v: &serde_json::Value,
    key: &str,
) -> Result<Option<chrono::DateTime<chrono::Utc>>, crate::tracker::TrackerError> {
    use crate::tracker::TrackerError;
    match v.get(key) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(s)) => chrono::DateTime::parse_from_rfc3339(s)
            .map(|t| Some(t.with_timezone(&chrono::Utc)))
            .map_err(|_| TrackerError::Decode),
        Some(_) => Err(TrackerError::Decode),
    }
}

/// Pure: the GitLab tracker among `trackers` that answers to the forge `key`: the one
/// whose current `full_path` is the key's path or its longest prefix on `/`
/// boundaries; on a tie, the first in `trackers`' order (level, then creation).
///
/// `host` is the key's host when it is known (a bare `#n` read in a folder's
/// repository, spec tracker-github §H5): then only trackers on that host answer.
/// A typed `path#n` carries no host and matches by path alone.
pub fn tracker_for_key<'a>(
    trackers: &'a [crate::db::pg_store::trackers::TrackerRow],
    key: &str,
    host: Option<&str>,
) -> Option<&'a crate::db::pg_store::trackers::TrackerRow> {
    use crate::tracker::TrackerKind;
    let (path, _) = parse_key(key)?;
    let on_host = |t: &crate::db::pg_store::trackers::TrackerRow| {
        host.is_none_or(|h| {
            reqwest::Url::parse(&t.base_url)
                .ok()
                .and_then(|u| u.host_str().map(|th| th.eq_ignore_ascii_case(h)))
                .unwrap_or(false)
        })
    };
    let mut best: Option<(&crate::db::pg_store::trackers::TrackerRow, usize)> = None;
    for t in trackers
        .iter()
        .filter(|t| TrackerKind::parse(&t.kind) == Some(TrackerKind::Gitlab) && on_host(t))
    {
        let Some(full_path) = t.params["full_path"].as_str().map(str::to_ascii_lowercase) else {
            continue;
        };
        let matches = path == full_path || path.starts_with(&format!("{full_path}/"));
        if matches && best.is_none_or(|(_, len)| full_path.len() > len) {
            best = Some((t, full_path.len()));
        }
    }
    best.map(|(t, _)| t)
}

/// The trackers that may answer a forge `key`, in the order to try them
/// (tracker-github §H5). `holding` are the trackers whose mirror holds a task of the
/// key's repository; they are tried first among GitHub's.
pub fn candidates_for_key<'a>(
    trackers: &'a [crate::db::pg_store::trackers::TrackerRow],
    key: &str,
    host: Option<&str>,
    holding: &[uuid::Uuid],
) -> Vec<&'a crate::db::pg_store::trackers::TrackerRow> {
    use crate::tracker::TrackerKind;
    // A project spans repositories, so the path cannot pick a GitHub board: those
    // holding the repository first, then the rest; containment decides (§H5.3).
    let github = || {
        let (first, rest): (Vec<_>, Vec<_>) = trackers
            .iter()
            .filter(|t| TrackerKind::parse(&t.kind) == Some(TrackerKind::Github))
            .partition(|t| holding.contains(&t.id));
        first.into_iter().chain(rest).collect()
    };
    match host {
        Some(h) if h.eq_ignore_ascii_case("github.com") => github(),
        Some(_) => tracker_for_key(trackers, key, host).into_iter().collect(),
        None => match tracker_for_key(trackers, key, None) {
            Some(gitlab) => vec![gitlab],
            None => github(),
        },
    }
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

    #[test]
    fn the_longest_full_path_answers_a_key() {
        let row = |kind: &str, full_path: &str| crate::db::pg_store::trackers::TrackerRow {
            kind: kind.into(),
            base_url: "https://gitlab.example.com".into(),
            params: serde_json::json!({"full_path": full_path}),
            ..crate::db::pg_store::trackers::TrackerRow::test_default()
        };
        let trackers = [
            row("gitlab", "acme"),
            row("gitlab", "acme/backend"),
            row("gitlab", "acme/back"),
            row("jira", "acme/backend/x"),
        ];
        let pick = |k: &str| {
            tracker_for_key(&trackers, k, None)
                .map(|t| t.params["full_path"].as_str().unwrap().to_string())
        };
        assert_eq!(pick("acme/backend#4").as_deref(), Some("acme/backend"));
        assert_eq!(pick("acme/backend/sub#4").as_deref(), Some("acme/backend"), "on / boundaries");
        assert_eq!(pick("acme/frontend#4").as_deref(), Some("acme"));
        assert_eq!(
            pick("acme/backendx#4").as_deref(),
            Some("acme"),
            "acme/back is not a prefix of it"
        );
        assert_eq!(pick("other#4"), None);
    }

    #[test]
    fn a_key_read_on_one_host_never_reaches_a_tracker_on_another() {
        // tracker-github §H5: a bare `#12` keeps its folder's host, so a GitLab board
        // whose path happens to match cannot answer for another forge's ticket.
        let row = |base: &str, full_path: &str| crate::db::pg_store::trackers::TrackerRow {
            kind: "gitlab".into(),
            base_url: base.into(),
            params: serde_json::json!({"full_path": full_path}),
            ..crate::db::pg_store::trackers::TrackerRow::test_default()
        };
        let trackers =
            [row("https://gitlab.example.com", "acme"), row("https://git.other.org", "acme")];
        let host_of =
            |k: &str, h: Option<&str>| tracker_for_key(&trackers, k, h).map(|t| t.base_url.clone());
        assert_eq!(host_of("acme/platform#12", Some("github.com")), None);
        assert_eq!(
            host_of("acme/platform#12", Some("git.other.org")).as_deref(),
            Some("https://git.other.org")
        );
        assert_eq!(
            host_of("acme/platform#12", Some("GitLab.Example.com")).as_deref(),
            Some("https://gitlab.example.com"),
            "hosts compare case-insensitively"
        );
        // A typed key carries no host: the path alone, first in list order.
        assert_eq!(
            host_of("acme/platform#12", None).as_deref(),
            Some("https://gitlab.example.com")
        );
    }

    #[test]
    fn a_key_is_tried_on_its_own_forge_and_github_boards_holding_its_repo_first() {
        let row = |kind: &str, base: &str, full_path: Option<&str>, n: u128| {
            crate::db::pg_store::trackers::TrackerRow {
                id: uuid::Uuid::from_u128(n),
                kind: kind.into(),
                base_url: base.into(),
                params: serde_json::json!({"full_path": full_path}),
                ..crate::db::pg_store::trackers::TrackerRow::test_default()
            }
        };
        let trackers = [
            row("gitlab", "https://gitlab.example.com", Some("acme"), 1),
            row("github", "https://api.github.com", None, 2),
            row("github", "https://api.github.com", None, 3),
            row("jira", "https://acme.atlassian.net", None, 4),
        ];
        let ids = |host: Option<&str>, key: &str, holding: &[u128]| {
            let holding: Vec<uuid::Uuid> =
                holding.iter().map(|n| uuid::Uuid::from_u128(*n)).collect();
            candidates_for_key(&trackers, key, host, &holding)
                .iter()
                .map(|t| t.id.as_u128())
                .collect::<Vec<_>>()
        };
        // Read on github.com: only GitHub's boards, those holding the repo first.
        assert_eq!(ids(Some("github.com"), "acme/platform#2", &[3]), [3, 2]);
        assert_eq!(
            ids(Some("github.com"), "acme/platform#2", &[]),
            [2, 3],
            "an unmirrored repo is still tried"
        );
        // Read on a GitLab host: that host's GitLab board only.
        assert_eq!(ids(Some("gitlab.example.com"), "acme/platform#2", &[2, 3]), [1]);
        assert!(ids(Some("git.other.org"), "acme/platform#2", &[2]).is_empty());
        // Typed, no host: GitLab by path first, else GitHub's.
        assert_eq!(ids(None, "acme/platform#2", &[2]), [1]);
        assert_eq!(ids(None, "other/repo#2", &[3]), [3, 2]);
        let no_github = &trackers[..1];
        assert!(candidates_for_key(no_github, "other/repo#2", None, &[]).is_empty());
    }
}
