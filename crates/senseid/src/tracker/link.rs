//! Provider-neutral linking inputs (spec R5 step 3): which keys can link, and to
//! which task. Matching text against them is the adapter's `match_links`.

use crate::tracker::{KnownTask, TaskRow};

/// One stored key alias of a task, with what the tie-break needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredAlias {
    pub external_id: String,
    pub key: String,
    pub scope: Option<String>,
    /// The alias is the task's stored current key.
    pub current: bool,
    pub removed: bool,
    pub last_seen_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Every key that can link this sync: all stored aliases plus the snapshot's
/// current keys. One task per `(key, scope)`: a task in the snapshot wins, then a
/// non-removed one, then the later `last_seen_at`. Sorted by `(key, scope, external_id)`.
pub fn known_tasks(stored: &[StoredAlias], snapshot: &[TaskRow]) -> Vec<KnownTask> {
    use std::collections::{BTreeMap, HashMap};
    type Slot = (String, Option<String>);
    let in_snapshot: HashMap<&str, &TaskRow> =
        snapshot.iter().map(|t| (t.external_id.as_str(), t)).collect();
    // Per stored task: (removed, last_seen_at).
    let state: HashMap<&str, (bool, Option<chrono::DateTime<chrono::Utc>>)> =
        stored.iter().map(|a| (a.external_id.as_str(), (a.removed, a.last_seen_at))).collect();
    // Every claim on a (key, scope): (task, the key is this task's current key).
    let mut claims: BTreeMap<Slot, Vec<(&str, bool)>> = BTreeMap::new();
    for a in stored {
        let current = match in_snapshot.get(a.external_id.as_str()) {
            Some(t) => t.key == a.key && t.scope == a.scope,
            None => a.current,
        };
        claims.entry((a.key.clone(), a.scope.clone())).or_default().push((&a.external_id, current));
    }
    for t in snapshot {
        claims.entry((t.key.clone(), t.scope.clone())).or_default().push((&t.external_id, true));
    }
    claims
        .into_iter()
        .filter_map(|((key, scope), tasks)| {
            // Rank: the snapshot's current owner of the key, then live, then latest seen.
            let rank = |(id, current): &(&str, bool)| {
                let owns = in_snapshot.get(id).is_some_and(|t| t.key == key && t.scope == scope);
                let (removed, seen) = match in_snapshot.get(id) {
                    Some(_) => (false, None),
                    None => state.get(id).copied().unwrap_or((false, None)),
                };
                (owns, !removed, seen, *current, std::cmp::Reverse(id.to_string()))
            };
            let best = tasks.iter().max_by_key(|c| rank(c))?;
            let current = tasks.iter().filter(|(id, _)| *id == best.0).any(|(_, c)| *c);
            Some(KnownTask { external_id: best.0.to_string(), key, scope, current })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn alias(id: &str, key: &str, current: bool, removed: bool, seen_day: u32) -> StoredAlias {
        StoredAlias {
            external_id: id.into(),
            key: key.into(),
            scope: None,
            current,
            removed,
            last_seen_at: Some(Utc.with_ymd_and_hms(2026, 10, seen_day, 0, 0, 0).unwrap()),
        }
    }

    fn task(id: &str, key: &str) -> TaskRow {
        TaskRow {
            external_id: id.into(),
            key: key.into(),
            scope: None,
            summary: None,
            issue_type: None,
            status: None,
        }
    }

    /// (key, external_id, current)
    fn view(k: Vec<KnownTask>) -> Vec<(String, String, bool)> {
        k.into_iter().map(|t| (t.key, t.external_id, t.current)).collect()
    }

    #[test]
    fn stored_aliases_and_new_snapshot_keys_are_all_known() {
        let stored = [alias("1", "AUTH-1", true, false, 1), alias("9", "AUTH-9", true, true, 1)];
        let snap = [task("1", "AUTH-1"), task("2", "AUTH-2")];
        assert_eq!(
            view(known_tasks(&stored, &snap)),
            vec![
                ("AUTH-1".into(), "1".into(), true),
                ("AUTH-2".into(), "2".into(), true),
                ("AUTH-9".into(), "9".into(), true),
            ],
            "a new ticket and a removed one both link"
        );
    }

    #[test]
    fn a_rename_in_the_snapshot_moves_current_and_keeps_the_old_key() {
        let stored = [alias("1", "AUTH-420", true, false, 1)];
        assert_eq!(
            view(known_tasks(&stored, &[task("1", "IAM-5")])),
            vec![("AUTH-420".into(), "1".into(), false), ("IAM-5".into(), "1".into(), true)]
        );
    }

    #[test]
    fn a_reused_key_goes_to_the_task_in_the_snapshot() {
        // Task 1 was AUTH-7 and moved away; task 2 is AUTH-7 now.
        let stored = [alias("1", "AUTH-7", false, false, 5), alias("1", "IAM-1", true, false, 5)];
        let snap = [task("1", "IAM-1"), task("2", "AUTH-7")];
        assert_eq!(
            view(known_tasks(&stored, &snap)),
            vec![("AUTH-7".into(), "2".into(), true), ("IAM-1".into(), "1".into(), true)]
        );
    }

    #[test]
    fn the_snapshot_owner_beats_a_stored_claim_on_the_key() {
        // Task 1 held AUTH-7 (still stored as its current key) and is not in this
        // snapshot; task 2 is AUTH-7 now.
        let stored = [alias("1", "AUTH-7", true, false, 5)];
        assert_eq!(
            view(known_tasks(&stored, &[task("2", "AUTH-7")])),
            vec![("AUTH-7".into(), "2".into(), true)]
        );
    }

    #[test]
    fn among_stored_tasks_live_beats_removed_then_later_seen_wins() {
        let live_vs_removed =
            [alias("1", "AUTH-7", true, true, 9), alias("2", "AUTH-7", false, false, 1)];
        assert_eq!(view(known_tasks(&live_vs_removed, &[]))[0].1, "2", "the live one wins");
        let both_removed =
            [alias("1", "AUTH-7", true, true, 1), alias("2", "AUTH-7", true, true, 9)];
        assert_eq!(view(known_tasks(&both_removed, &[]))[0].1, "2", "the later seen wins");
        assert_eq!(known_tasks(&both_removed, &[]).len(), 1, "one task per key");
    }

    #[test]
    fn the_same_key_in_two_scopes_is_two_tasks() {
        let mut a = alias("1", "#12", true, false, 1);
        a.scope = Some("gitlab.com/g/a".into());
        let mut b = alias("2", "#12", true, false, 1);
        b.scope = Some("gitlab.com/g/b".into());
        assert_eq!(known_tasks(&[a, b], &[]).len(), 2);
    }
}
