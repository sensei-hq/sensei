//! Stage 3 — structure write: the barrier.
//!
//! Spec: `docs/spec/indexer/03-structure-write.md`.
//!
//! Every folder row and every file row is written BEFORE any parse task is
//! enqueued. The gap between "all structure exists" and "work begins" is the
//! barrier, and three problems do not arise because of it (R14):
//!
//! 1. No race. Parse tasks run concurrently; if each did get-or-create on its
//!    file row, two tasks touching one file would race to insert it. Creating
//!    the rows upfront, single-threaded, removes the race rather than locking
//!    around it.
//! 2. The denominator is free — at the barrier the complete post-filter file
//!    set is known, which IS `folders.props.expected_files`.
//! 3. A stalled parse is visible: a file row with no outcome is a task that
//!    never ran, which today is indistinguishable from a file that does not
//!    exist.
//!
//! [`plan_structure`] is PURE. It is also the SAME classification stage 9's
//! incremental path needs (09 S4) — one implementation, so a full scan and an
//! incremental update cannot disagree about what changed.

use std::collections::BTreeMap;

/// What the walk observed about one file. `hash` is the content fingerprint
/// that decides re-parsing (R14/S6) — `mtime` alone is not enough, because a
/// touched-but-identical file must NOT be re-parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFacts {
    pub mtime: i64,
    pub hash: String,
}

/// How one file changed between the previous scan and this one.
///
/// Derived from OLD **plus** NEW — the pairing is what makes a change
/// classifiable rather than guessable (R14).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    /// New file. Create the row, parse it.
    Added,
    /// Content changed. Touch mtime + hash, parse it.
    ContentChanged,
    /// Seen before, byte-identical, only the timestamp moved. Refresh mtime
    /// and do NOT parse — re-parsing on mtime alone is how a `touch` or a
    /// checkout turns into a full re-index.
    TouchedOnly,
    /// Unchanged in every respect.
    Unchanged,
    /// Gone from disk. Handed to reconcile (R10.8), NEVER a prefix DELETE.
    Removed,
}

impl ChangeKind {
    /// Whether this change requires re-parsing the file. Content decides —
    /// never the timestamp.
    pub fn needs_parse(self) -> bool {
        matches!(self, ChangeKind::Added | ChangeKind::ContentChanged)
    }
}

/// The classification of one scan against the previous state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StructurePlan {
    /// Every observed file with its verdict, ordered by path so a plan is
    /// comparable and a test can assert on it directly.
    pub files: Vec<(String, ChangeKind)>,
    /// Paths present before and absent now. These go to reconcile.
    pub removed: Vec<String>,
}

impl StructurePlan {
    /// The barrier's denominator (S3): every file this scan will account for.
    ///
    /// Counted at the barrier from the plan itself, never recomputed later by
    /// a second query — a progress bar whose denominator is re-derived can
    /// disagree with the work actually queued.
    pub fn expected_files(&self) -> usize {
        self.files.len()
    }

    /// The subset that needs a parse task. Everything else already has a
    /// current row.
    pub fn to_parse(&self) -> Vec<&str> {
        self.files.iter().filter(|(_, k)| k.needs_parse()).map(|(p, _)| p.as_str()).collect()
    }

    pub fn count(&self, kind: ChangeKind) -> usize {
        self.files.iter().filter(|(_, k)| *k == kind).count()
    }
}

/// Classify this scan against the previous one (S4). PURE.
///
/// `current` is what the walk just observed; `previous` is what the `files`
/// table holds. Both are keyed by folder-relative path.
pub fn plan_structure(
    current: &BTreeMap<String, FileFacts>,
    previous: &BTreeMap<String, FileFacts>,
) -> StructurePlan {
    let mut plan = StructurePlan::default();

    for (path, now) in current {
        let kind = match previous.get(path) {
            None => ChangeKind::Added,
            Some(before) if before.hash != now.hash => ChangeKind::ContentChanged,
            Some(before) if before.mtime != now.mtime => ChangeKind::TouchedOnly,
            Some(_) => ChangeKind::Unchanged,
        };
        plan.files.push((path.clone(), kind));
    }

    // A path in the previous set and not the current one is REMOVED. Absence
    // here is observed, not inferred — the walk enumerated the directory — so
    // it is safe to act on, unlike the "parsed zero symbols" case R10.3 brakes.
    for path in previous.keys() {
        if !current.contains_key(path) {
            plan.removed.push(path.clone());
        }
    }

    plan
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(mtime: i64, hash: &str) -> FileFacts {
        FileFacts { mtime, hash: hash.to_string() }
    }

    fn map(v: &[(&str, i64, &str)]) -> BTreeMap<String, FileFacts> {
        v.iter().map(|(p, m, h)| ((*p).to_string(), facts(*m, h))).collect()
    }

    #[test]
    fn a_new_file_is_added_and_parses() {
        let plan = plan_structure(&map(&[("a.rs", 1, "h1")]), &map(&[]));
        assert_eq!(plan.files, vec![("a.rs".into(), ChangeKind::Added)]);
        assert_eq!(plan.to_parse(), vec!["a.rs"]);
    }

    #[test]
    fn changed_content_parses_but_a_bare_touch_does_not() {
        // The distinction the whole `content_hash` column exists for. A
        // checkout or a `touch` moves every mtime; re-parsing on that turns a
        // no-op into a full re-index.
        let previous = map(&[("same.rs", 100, "h1"), ("edited.rs", 100, "h1")]);
        let current = map(&[("same.rs", 999, "h1"), ("edited.rs", 999, "h2")]);

        let plan = plan_structure(&current, &previous);

        assert_eq!(plan.count(ChangeKind::TouchedOnly), 1);
        assert_eq!(plan.count(ChangeKind::ContentChanged), 1);
        assert_eq!(plan.to_parse(), vec!["edited.rs"], "only the edited file re-parses");
    }

    #[test]
    fn an_identical_file_is_unchanged_and_does_not_parse() {
        let m = map(&[("a.rs", 1, "h1")]);
        let plan = plan_structure(&m, &m);
        assert_eq!(plan.files, vec![("a.rs".into(), ChangeKind::Unchanged)]);
        assert!(plan.to_parse().is_empty());
    }

    #[test]
    fn a_vanished_file_is_removed_and_is_not_in_the_file_set() {
        // Removed files go to reconcile (R10.8), so they must NOT appear in
        // `files` — a plan that both keeps and removes a path is incoherent.
        let plan = plan_structure(
            &map(&[("kept.rs", 1, "h1")]),
            &map(&[("kept.rs", 1, "h1"), ("gone.rs", 1, "h2")]),
        );

        assert_eq!(plan.removed, vec!["gone.rs"]);
        assert!(!plan.files.iter().any(|(p, _)| p == "gone.rs"));
        assert!(plan.to_parse().is_empty());
    }

    #[test]
    fn the_denominator_counts_every_observed_file_not_just_the_parsed_ones() {
        // S3. expected_files is the BARRIER's count — what the scan will
        // account for — not the work queue. A progress bar keyed on the
        // parse list reads 100% while unchanged files are unaccounted for.
        let previous = map(&[("a.rs", 1, "h1"), ("b.rs", 1, "h2")]);
        let current = map(&[("a.rs", 1, "h1"), ("b.rs", 2, "CHANGED"), ("c.rs", 1, "h3")]);

        let plan = plan_structure(&current, &previous);

        assert_eq!(plan.expected_files(), 3, "every observed file");
        assert_eq!(plan.to_parse().len(), 2, "only two need work");
    }

    #[test]
    fn a_rename_is_an_add_plus_a_remove_with_no_special_case() {
        // R10.5: a rename needs no case of its own. In Rust and TS the module
        // path IS the file path, so every fqn in the file changes — it is not
        // "the same symbol at a new path".
        let plan = plan_structure(&map(&[("b.rs", 1, "same")]), &map(&[("a.rs", 1, "same")]));

        assert_eq!(plan.files, vec![("b.rs".into(), ChangeKind::Added)]);
        assert_eq!(plan.removed, vec!["a.rs"]);
    }

    #[test]
    fn an_empty_scan_of_a_populated_folder_removes_everything_it_observed_gone() {
        // A repo whose files were all deleted. Observed, not inferred — the
        // walk enumerated the directory and found nothing, which is different
        // from a parse returning zero symbols (R10.3's brake).
        let plan = plan_structure(&map(&[]), &map(&[("a.rs", 1, "h1"), ("b.rs", 1, "h2")]));

        assert_eq!(plan.removed, vec!["a.rs", "b.rs"]);
        assert_eq!(plan.expected_files(), 0);
    }

    #[test]
    fn the_plan_is_order_independent() {
        // R6/A6: the same inputs must produce the same plan regardless of the
        // order the walk happened to yield them in. BTreeMap makes this true
        // by construction; the test is what stops someone swapping in a HashMap.
        let a = map(&[("z.rs", 1, "h"), ("a.rs", 1, "h"), ("m.rs", 1, "h")]);
        let b = map(&[("a.rs", 1, "h"), ("m.rs", 1, "h"), ("z.rs", 1, "h")]);
        assert_eq!(plan_structure(&a, &map(&[])), plan_structure(&b, &map(&[])));
    }
}
