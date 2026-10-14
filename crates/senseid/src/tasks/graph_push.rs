//! What a graph push sends, decided from what the daemon has and what the dōjō
//! holds (#225; docs/spec/dojo/graph-sync.md §3, §5).
//!
//! Pure: the git output, the local manifest and the dōjō's hashes come in; the
//! files to push, the manifest to send and the batches come out. The I/O around
//! it lives in `dojo_sync`.
//!
//! **Only committed work is shared.** The local graph is read from the working
//! tree, so a `main` checkout with uncommitted edits would otherwise push
//! unmerged work into the team's `main` graph. A path `git status` reports is
//! held back — and kept in the manifest at the hash the dōjō already has, so the
//! dōjō does not prune a file merely because it is being edited here.

use std::collections::{BTreeMap, HashMap, HashSet};

/// `git log -1` with this format prints `<sha>\0<committer ISO 8601>`.
pub const HEAD_FORMAT: &str = "--format=%H%x00%cI";

/// The sha and committer time of `git log -1 --format=%H%x00%cI`.
pub fn parse_head(stdout: &str) -> Result<(String, String), String> {
    let line = stdout.trim();
    let (sha, at) =
        line.split_once('\0').ok_or_else(|| format!("unreadable HEAD line: {line:?}"))?;
    if sha.len() < 7 || !sha.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("unreadable HEAD sha: {sha:?}"));
    }
    chrono::DateTime::parse_from_rfc3339(at)
        .map_err(|e| format!("unreadable HEAD time {at:?}: {e}"))?;
    Ok((sha.to_string(), at.to_string()))
}

/// The paths `git status --porcelain -z` reports: modified, added, deleted,
/// untracked, and both sides of a rename or copy.
pub fn parse_porcelain_z(stdout: &[u8]) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut fields = stdout.split(|b| *b == 0).filter(|f| !f.is_empty());
    while let Some(entry) = fields.next() {
        if entry.len() < 4 {
            continue;
        }
        let status = &entry[..2];
        out.insert(String::from_utf8_lossy(&entry[3..]).into_owned());
        // A rename or copy is followed by its ORIGINAL path as the next field.
        if (status.contains(&b'R') || status.contains(&b'C'))
            && let Some(from) = fields.next()
        {
            out.insert(String::from_utf8_lossy(from).into_owned());
        }
    }
    out
}

/// What to push for one checkout.
#[derive(Debug, PartialEq, Eq)]
pub struct PushPlan {
    /// Committed files whose hash differs from the dōjō's, sorted.
    pub changed: Vec<String>,
    /// Every path the dōjō should keep, with its hash: the committed local files,
    /// plus each held-back path at the hash the dōjō already has.
    pub manifest: BTreeMap<String, String>,
}

pub fn plan_push(
    local: &BTreeMap<String, String>,
    remote: &HashMap<String, String>,
    dirty: &HashSet<String>,
) -> PushPlan {
    let mut changed = Vec::new();
    let mut manifest = BTreeMap::new();
    for (path, hash) in local {
        if dirty.contains(path) {
            continue;
        }
        if remote.get(path) != Some(hash) {
            changed.push(path.clone());
        }
        manifest.insert(path.clone(), hash.clone());
    }
    for path in dirty {
        if let Some(hash) = remote.get(path) {
            manifest.insert(path.clone(), hash.clone());
        }
    }
    PushPlan { changed, manifest }
}

/// Slices grouped into requests under a byte cap.
#[derive(Debug, PartialEq)]
pub struct Batches {
    pub batches: Vec<Vec<serde_json::Value>>,
    /// Paths whose single slice is over the cap — refused here, never split,
    /// and counted in the sync reason.
    pub oversize: Vec<String>,
}

/// Group whole slices, in order, into batches whose serialized slices stay
/// within `cap` bytes. A slice is never split.
pub fn batch(slices: Vec<(String, serde_json::Value)>, cap: usize) -> Batches {
    let mut batches = Vec::new();
    let mut oversize = Vec::new();
    let mut current: Vec<serde_json::Value> = Vec::new();
    let mut size = 0usize;
    for (path, slice) in slices {
        let n = serde_json::to_vec(&slice).map(|v| v.len()).unwrap_or(usize::MAX);
        if n > cap {
            oversize.push(path);
            continue;
        }
        if size + n > cap && !current.is_empty() {
            batches.push(std::mem::take(&mut current));
            size = 0;
        }
        size += n;
        current.push(slice);
    }
    if !current.is_empty() {
        batches.push(current);
    }
    Batches { batches, oversize }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn map(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs.iter().map(|(p, h)| (p.to_string(), h.to_string())).collect()
    }
    fn hmap(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(p, h)| (p.to_string(), h.to_string())).collect()
    }
    fn set(paths: &[&str]) -> HashSet<String> {
        paths.iter().map(|p| p.to_string()).collect()
    }

    #[test]
    fn head_reads_the_sha_and_committer_time() {
        let (sha, at) =
            parse_head("0123456789abcdef0123456789abcdef01234567\u{0}2026-10-01T10:00:00+02:00\n")
                .unwrap();
        assert_eq!(sha, "0123456789abcdef0123456789abcdef01234567");
        assert_eq!(at, "2026-10-01T10:00:00+02:00");
        // An unborn branch prints nothing; a bad time is a failed read, not a value.
        assert!(parse_head("").is_err());
        assert!(parse_head("0123456789abcdef\u{0}yesterday").is_err());
    }

    #[test]
    fn porcelain_names_every_touched_path_including_both_sides_of_a_rename() {
        let out = b" M src/a.ts\0?? new.ts\0D  gone.ts\0R  src/to.ts\0src/from.ts\0";
        assert_eq!(
            parse_porcelain_z(out),
            set(&["src/a.ts", "new.ts", "gone.ts", "src/to.ts", "src/from.ts"])
        );
        assert!(parse_porcelain_z(b"").is_empty());
    }

    #[test]
    fn only_committed_files_that_differ_are_pushed() {
        let local = map(&[("a.ts", "h1"), ("b.ts", "h2"), ("c.ts", "h3")]);
        let remote = hmap(&[("a.ts", "h1"), ("b.ts", "old")]);
        let plan = plan_push(&local, &remote, &set(&[]));
        assert_eq!(plan.changed, ["b.ts", "c.ts"]);
        assert_eq!(plan.manifest, local);
    }

    #[test]
    fn an_edited_file_is_held_back_and_kept_at_the_dojos_hash() {
        // a.ts is mid-edit: its local hash is the working tree's, which is not
        // shared. It must not be pushed, and must not be pruned either.
        let local = map(&[("a.ts", "dirty"), ("b.ts", "h2")]);
        let remote = hmap(&[("a.ts", "h1"), ("b.ts", "h2")]);
        let plan = plan_push(&local, &remote, &set(&["a.ts"]));
        assert!(plan.changed.is_empty());
        assert_eq!(plan.manifest, map(&[("a.ts", "h1"), ("b.ts", "h2")]));
    }

    #[test]
    fn a_file_deleted_but_not_committed_is_still_kept() {
        // Gone from the working tree (so from the local index) but the deletion is
        // not committed: the shared graph keeps it until it is.
        let local = map(&[("b.ts", "h2")]);
        let remote = hmap(&[("a.ts", "h1"), ("b.ts", "h2")]);
        let plan = plan_push(&local, &remote, &set(&["a.ts"]));
        assert_eq!(plan.manifest, map(&[("a.ts", "h1"), ("b.ts", "h2")]));
    }

    #[test]
    fn an_untracked_new_file_is_neither_pushed_nor_listed() {
        let local = map(&[("new.ts", "hn"), ("b.ts", "h2")]);
        let remote = hmap(&[("b.ts", "h2")]);
        let plan = plan_push(&local, &remote, &set(&["new.ts"]));
        assert!(plan.changed.is_empty());
        assert_eq!(plan.manifest, map(&[("b.ts", "h2")]));
    }

    #[test]
    fn batches_keep_whole_slices_under_the_cap_in_order_and_count_an_oversize_one() {
        let s = |p: &str, pad: usize| (p.to_string(), json!({ "path": p, "pad": "x".repeat(pad) }));
        let one = serde_json::to_vec(&s("a", 100).1).unwrap().len();
        let out = batch(vec![s("a", 100), s("b", 100), s("c", 100), s("huge", 10_000)], one * 2);
        assert_eq!(out.oversize, ["huge"]);
        let paths: Vec<Vec<&str>> = out
            .batches
            .iter()
            .map(|b| b.iter().map(|v| v["path"].as_str().unwrap()).collect())
            .collect();
        assert_eq!(paths, [vec!["a", "b"], vec!["c"]]);
        assert!(batch(vec![], 10).batches.is_empty());
    }
}
