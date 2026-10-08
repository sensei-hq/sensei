//! One symbol and what reaches it, walked a ring at a time (#220).
//!
//! The Neighbourhood diagram centres one symbol, its callers to the left and its
//! callees to the right, and at `depth: 2` the callers' callers and the callees'
//! callees. `@rokkit/graph`'s `Neighborhood` assigns the rings itself from the
//! edges it is handed; what it cannot do is fetch them. This module decides WHICH
//! edges to fetch, one ring at a time, so the daemon reads a bounded subgraph
//! rather than the project's call graph.
//!
//! ## Two directed walks, not one undirected one
//!
//! A caller's caller is further left and a callee's callee further right; the
//! callers of a callee are not in this picture at all. Each side walks in its own
//! direction from the focus with its OWN claimed set — a mutual neighbour (it
//! calls the focus and the focus calls it) is reached by both walks, and the
//! component, not this module, decides which column it lands in by its dominant
//! direction. Sharing one claimed set would starve whichever side it was not
//! claimed on of that node's next ring.
//!
//! ## A hub is cut, and the cut is counted
//!
//! One ring can be thousands wide — a logging helper is called from everywhere.
//! A ring keeps at most `cap` far ends, the most-called first, and says how many
//! it dropped. A column of forty cards with "and 1,212 more" beneath it is a
//! true picture; a column of forty with nothing beneath it reads as the whole
//! set.
//!
//! ## A node outside the project is reached but never walked through
//!
//! A call into a library, or into another project's code, is a real edge and
//! its far end is drawn. Expanding through it would pull in every caller of
//! `serde_json::to_string` on the machine, so it is a leaf: it never seeds the
//! next ring.

use std::collections::{BTreeMap, HashSet};

/// Which way a ring walks from its frontier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    /// Toward callers: from a frontier node to whatever calls it.
    In,
    /// Toward callees: from a frontier node to whatever it calls.
    Out,
}

/// One placed `calls` relationship between two symbols, as the hop read returns
/// it — every edge row between the pair folded into one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call {
    pub source: uuid::Uuid,
    pub target: uuid::Uuid,
    /// How many call sites the edge rows between this pair record.
    pub occurrences: i64,
    /// Whether the end AWAY from the frontier may seed the next ring — false for
    /// a library node or a symbol outside the project.
    pub far_walkable: bool,
}

impl Call {
    /// The end of this call away from the frontier, seen from `side`.
    fn far(&self, side: Side) -> uuid::Uuid {
        match side {
            Side::In => self.source,
            Side::Out => self.target,
        }
    }

    fn near(&self, side: Side) -> uuid::Uuid {
        match side {
            Side::In => self.target,
            Side::Out => self.source,
        }
    }
}

/// What one ring step produced.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Ring {
    /// Far ends claimed by this ring, most-called first.
    pub reached: Vec<uuid::Uuid>,
    /// The calls that reach them — and ONLY them, so an edge never points at a
    /// node the payload does not carry.
    pub calls: Vec<Call>,
    /// The subset of `reached` that may seed the next ring.
    pub frontier: Vec<uuid::Uuid>,
    /// Far ends this ring found but did not keep, because the cap was reached.
    pub cut: usize,
}

/// Advance one ring on `side` from `frontier`.
///
/// `calls` is what the hop read returned for this frontier — it may contain
/// calls that do not touch the frontier on the expected end, and those are
/// ignored rather than trusted. `claimed` is this side's running set and is
/// extended with what the ring keeps; a far end already claimed (nearer, or the
/// focus itself) is never re-claimed, which is what keeps one identity to one
/// ring.
pub fn step(
    calls: &[Call],
    frontier: &HashSet<uuid::Uuid>,
    side: Side,
    claimed: &mut HashSet<uuid::Uuid>,
    cap: usize,
) -> Ring {
    // Weight per far end: the call sites reaching it from this frontier, so the
    // cap keeps the neighbours the frontier leans on most. A BTreeMap gives the
    // tie-break (lowest id) a stable order across requests — a cut that moved on
    // every reload would show a different picture of the same graph.
    let mut weight: BTreeMap<uuid::Uuid, (i64, bool)> = BTreeMap::new();
    for c in calls {
        if !frontier.contains(&c.near(side)) {
            continue;
        }
        let far = c.far(side);
        if far == c.near(side) || claimed.contains(&far) {
            continue;
        }
        let e = weight.entry(far).or_insert((0, c.far_walkable));
        e.0 += c.occurrences;
        e.1 &= c.far_walkable;
    }

    let mut ranked: Vec<(uuid::Uuid, i64, bool)> =
        weight.into_iter().map(|(id, (w, walkable))| (id, w, walkable)).collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let cut = ranked.len().saturating_sub(cap);
    ranked.truncate(cap);

    let kept: HashSet<uuid::Uuid> = ranked.iter().map(|r| r.0).collect();
    claimed.extend(kept.iter().copied());
    let ring_calls = calls
        .iter()
        .filter(|c| frontier.contains(&c.near(side)) && kept.contains(&c.far(side)))
        .cloned()
        .collect();

    Ring {
        reached: ranked.iter().map(|r| r.0).collect(),
        frontier: ranked.iter().filter(|r| r.2).map(|r| r.0).collect(),
        calls: ring_calls,
        cut,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u128) -> uuid::Uuid {
        uuid::Uuid::from_u128(n)
    }

    fn call(source: u128, target: u128, occurrences: i64) -> Call {
        Call { source: id(source), target: id(target), occurrences, far_walkable: true }
    }

    fn set(ids: &[u128]) -> HashSet<uuid::Uuid> {
        ids.iter().map(|&n| id(n)).collect()
    }

    /// Mutation that must break this: swap `far`/`near` for `Side::In`.
    #[test]
    fn the_in_side_reaches_callers_and_the_out_side_reaches_callees() {
        // 1 calls the focus (0); the focus calls 2.
        let calls = [call(1, 0, 1), call(0, 2, 1)];
        let focus = set(&[0]);

        let mut claimed = set(&[0]);
        let left = step(&calls, &focus, Side::In, &mut claimed, 10);
        assert_eq!(left.reached, vec![id(1)]);

        let mut claimed = set(&[0]);
        let right = step(&calls, &focus, Side::Out, &mut claimed, 10);
        assert_eq!(right.reached, vec![id(2)]);
    }

    /// Mutation that must break this: drop the `claimed.contains` guard.
    #[test]
    fn a_node_already_claimed_is_never_reclaimed() {
        // 1 calls 0 directly AND via 2, so it is ring 1 — not ring 2 as well.
        let calls = [call(1, 0, 1), call(2, 0, 1), call(1, 2, 1)];
        let mut claimed = set(&[0]);
        let ring1 = step(&calls, &set(&[0]), Side::In, &mut claimed, 10);
        assert_eq!(ring1.reached.len(), 2);

        let frontier: HashSet<_> = ring1.frontier.iter().copied().collect();
        let ring2 = step(&calls, &frontier, Side::In, &mut claimed, 10);
        assert!(ring2.reached.is_empty(), "1 was claimed at ring 1: {:?}", ring2.reached);
    }

    /// Mutation that must break this: drop the self-call guard.
    #[test]
    fn recursion_is_not_a_neighbour() {
        let calls = [call(0, 0, 3)];
        let mut claimed = HashSet::new();
        let ring = step(&calls, &set(&[0]), Side::Out, &mut claimed, 10);
        assert!(ring.reached.is_empty());
    }

    /// Mutations that must break this: sort ascending, drop `cut`, or keep the
    /// calls into the cut nodes.
    #[test]
    fn a_hub_keeps_the_most_called_and_counts_the_rest() {
        let calls = [call(1, 0, 1), call(2, 0, 9), call(3, 0, 5), call(4, 0, 2)];
        let mut claimed = set(&[0]);
        let ring = step(&calls, &set(&[0]), Side::In, &mut claimed, 2);

        assert_eq!(ring.reached, vec![id(2), id(3)], "most call sites first");
        assert_eq!(ring.cut, 2, "two callers were found and not drawn");
        assert!(
            ring.calls.iter().all(|c| c.source == id(2) || c.source == id(3)),
            "no edge points at a node the payload does not carry: {:?}",
            ring.calls
        );
        assert!(!claimed.contains(&id(1)), "a cut node is not claimed, so it cannot shadow");
    }

    /// Weight is the SUM over the frontier, so a node two frontier members both
    /// lean on outranks one that a single member calls slightly more.
    ///
    /// Mutation that must break this: `e.0 = c.occurrences` instead of `+=`.
    #[test]
    fn weight_sums_over_the_whole_frontier() {
        let calls = [call(1, 7, 3), call(2, 7, 3), call(1, 8, 4)];
        let mut claimed = set(&[1, 2]);
        let ring = step(&calls, &set(&[1, 2]), Side::Out, &mut claimed, 1);
        assert_eq!(ring.reached, vec![id(7)]);
    }

    /// Mutation that must break this: reverse the tie-break, or iterate a
    /// HashMap — the cut would move between reloads.
    #[test]
    fn a_tie_is_broken_by_id_so_the_cut_is_stable() {
        let calls = [call(9, 0, 1), call(5, 0, 1), call(7, 0, 1)];
        let mut claimed = set(&[0]);
        let ring = step(&calls, &set(&[0]), Side::In, &mut claimed, 2);
        assert_eq!(ring.reached, vec![id(5), id(7)]);
    }

    /// Mutation that must break this: make `frontier` equal `reached`.
    #[test]
    fn a_node_outside_the_project_is_drawn_but_never_walked_through() {
        let lib = Call { far_walkable: false, ..call(0, 2, 1) };
        let calls = [call(0, 1, 1), lib];
        let mut claimed = set(&[0]);
        let ring = step(&calls, &set(&[0]), Side::Out, &mut claimed, 10);
        assert_eq!(ring.reached.len(), 2, "the library call is a real edge and is drawn");
        assert_eq!(ring.frontier, vec![id(1)], "but it seeds nothing");
    }

    /// The hop read may return calls touching the frontier on the wrong end
    /// (an `In` read that also matched sources). They are ignored, not trusted.
    ///
    /// Mutation that must break this: drop the `frontier.contains(near)` guard.
    #[test]
    fn a_call_not_anchored_on_the_frontier_is_ignored() {
        // 4 calls 5, and 5 is not on the frontier — so 4 is nobody's caller here.
        // Its far end must be UNCLAIMED, or the claimed guard rejects it first and
        // this test passes without the anchor guard ever running.
        let calls = [call(4, 5, 1)];
        let mut claimed = set(&[0]);
        let ring = step(&calls, &set(&[0]), Side::In, &mut claimed, 10);
        assert!(ring.reached.is_empty(), "4 calls 5, not 0: {:?}", ring.reached);
    }
}
