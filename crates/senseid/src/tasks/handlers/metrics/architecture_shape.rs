//! The architecture metrics the code map's lenses already show (#156) — counted
//! per repository from the same analysis, never re-derived: the Cycles lens's
//! cycles and the Structure lens's upward dependencies (`layering::analyse`), the
//! Zones lens's zone of pain, Hidden coupling's pairs (at the code map's own
//! threshold, never the lens's display cap), and Complexity's outliers (the tier
//! `analysis::codemap::Complexity` carries).
//!
//! **Each repository is analysed on its own subgraph** (L0 ruling, 2026-11-03):
//! its modules and the dependencies between them. A dependency on another
//! repository's module is not followed — so a loop through two repositories is
//! no repository's cycle — and the number left out is carried in the reading
//! (`cross_repository_deps`), so a screen can say "N links to other repositories
//! are not counted". Per repository because metrics are stored per repository
//! and a count pools by summing.
//!
//! **No reading on an incomplete index** (L0 ruling): a repository whose root
//! folder's index is not complete writes nothing, so "0 cycles" can never mean
//! "not indexed yet". A complete repository with no module also reads nothing.
//!
//! **Findings, not only counts:** targets 3 and 4 are delta-scoped — a verdict
//! compares what is found at HEAD with what was found at base, by identity — so
//! each reading carries its cycles' member sets, its Up edges and its pain modules.

use crate::analysis::codemap::Complexity;
use crate::analysis::layering::Dep;

/// What the shape is read from, for one workspace.
#[allow(dead_code)] // read by `shape_by_repository` in #156 phase B
pub struct ShapeInput<'a> {
    /// Every repository of the workspace, and whether its index is complete
    /// (`folder_completeness.subtree_complete` of its root folder).
    pub repositories: &'a [(uuid::Uuid, bool)],
    /// Every module, with the repository it belongs to.
    pub modules: &'a [(String, uuid::Uuid)],
    /// Module dependencies, as the code map reads them.
    pub deps: &'a [Dep],
    /// The modules the Zones lens places in the zone of pain.
    pub pain: &'a [String],
    /// Hidden coupling's pairs at the code map's threshold — all of them, not the lens's first 40.
    pub hidden: &'a [(String, String)],
    /// Each measured module's complexity.
    pub complexity: &'a [(String, Complexity)],
}

/// What a reading found, by identity — what a delta verdict diffs.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ShapeFindings {
    /// Each cycle's members, sorted (as `layering::Cycle.members`).
    pub cycles: Vec<Vec<String>>,
    /// Each upward dependency, `source→target`, sorted.
    pub up_edges: Vec<String>,
    /// The modules in the zone of pain, sorted.
    pub pain: Vec<String>,
}

/// One repository's readings.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeReading {
    pub repository_id: uuid::Uuid,
    pub cycles: i64,
    /// Modules in the largest cycle; 0 when there is none.
    pub largest_cycle: i64,
    /// Dependencies the layering marks `Up` — each closes a loop (target 3).
    pub up_edges: i64,
    pub pain_modules: i64,
    pub hidden_pairs: i64,
    pub complexity_outliers: i64,
    /// Dependencies on another repository's modules, not followed.
    pub cross_repository_deps: i64,
    pub findings: ShapeFindings,
}

/// Each complete repository that has modules, read. Pending (#156 phase B).
#[allow(dead_code)]
pub fn shape_by_repository(_input: &ShapeInput<'_>) -> Vec<ShapeReading> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::layering::{Conformance, analyse};

    fn r(n: u8) -> uuid::Uuid {
        uuid::Uuid::from_bytes([n; 16])
    }
    fn s(v: &str) -> String {
        v.to_string()
    }
    fn reading(input: &ShapeInput<'_>, repo: uuid::Uuid) -> Option<ShapeReading> {
        shape_by_repository(input).into_iter().find(|x| x.repository_id == repo)
    }
    fn empty<'a>(
        repositories: &'a [(uuid::Uuid, bool)],
        modules: &'a [(String, uuid::Uuid)],
        deps: &'a [Dep],
    ) -> ShapeInput<'a> {
        ShapeInput { repositories, modules, deps, pain: &[], hidden: &[], complexity: &[] }
    }
    /// What `layering::analyse` says of one repository's own subgraph — the
    /// numbers the lens draws, which the reading must equal.
    fn lens(units: &[&str], deps: &[Dep]) -> (i64, i64, Vec<Vec<String>>, Vec<String>) {
        let units: Vec<String> = units.iter().map(|u| s(u)).collect();
        let l = analyse(&units, deps);
        let mut up: Vec<String> = l
            .deps
            .iter()
            .filter(|d| d.conformance == Conformance::Up)
            .map(|d| format!("{}→{}", d.source, d.target))
            .collect();
        up.sort();
        let cycles: Vec<Vec<String>> = l.cycles.iter().map(|c| c.members.clone()).collect();
        (l.cycles.len() as i64, l.cycles.first().map_or(0, |c| c.members.len() as i64), cycles, up)
    }

    /// Conservation (review G4): for one repository, the reading IS the lens's
    /// analysis — cycles, the largest, the Up edges and their identities. A
    /// self-dependency is no cycle and a dependency on an unknown module is
    /// dropped, so a hand-rolled count that sees either fails.
    #[test]
    #[ignore = "pending: #156"]
    fn a_single_repository_reads_exactly_what_the_lens_analyses() {
        let names = ["a/x", "a/y", "a/z", "a/p", "a/q", "a/w"];
        let modules: Vec<(String, uuid::Uuid)> = names.iter().map(|m| (s(m), r(1))).collect();
        let deps = [
            Dep::new("a/x", "a/y", 3),
            Dep::new("a/y", "a/z", 2),
            Dep::new("a/z", "a/x", 1),
            Dep::new("a/p", "a/q", 1),
            Dep::new("a/q", "a/p", 1),
            Dep::new("a/w", "a/x", 4),
            Dep::new("a/w", "a/w", 9),       // a self-dependency: no cycle
            Dep::new("a/q", "nowhere/m", 2), // names no module: dropped
        ];
        let (cycles, largest, members, up) = lens(&names, &deps);
        let repos = [(r(1), true)];
        let got = reading(&empty(&repos, &modules, &deps), r(1)).expect("r1 reads");
        assert_eq!(
            (got.cycles, got.largest_cycle, got.up_edges),
            (cycles, largest, up.len() as i64)
        );
        assert_eq!((got.findings.cycles, got.findings.up_edges), (members, up));
        assert_eq!(got.cross_repository_deps, 0, "nowhere/m is no repository's module");
    }

    /// Each repository on its own subgraph: r1 holds a 3-cycle and a 2-cycle, r2 a
    /// 2-cycle. A loop through r1 and r2 is neither's, and both count the link
    /// they did not follow (G1, G4).
    #[test]
    #[ignore = "pending: #156"]
    fn each_repository_reads_its_own_subgraph_and_counts_what_it_left_out() {
        let modules = [
            (s("a/x"), r(1)),
            (s("a/y"), r(1)),
            (s("a/z"), r(1)),
            (s("a/p"), r(1)),
            (s("a/q"), r(1)),
            (s("b/m"), r(2)),
            (s("b/n"), r(2)),
            (s("b/o"), r(2)),
        ];
        let r1_deps = [
            Dep::new("a/x", "a/y", 3),
            Dep::new("a/y", "a/z", 2),
            Dep::new("a/z", "a/x", 1),
            Dep::new("a/p", "a/q", 1),
            Dep::new("a/q", "a/p", 1),
        ];
        let r2_deps = [Dep::new("b/m", "b/n", 1), Dep::new("b/n", "b/m", 1)];
        let across = [Dep::new("b/o", "a/p", 1), Dep::new("a/p", "b/o", 1)];
        let deps: Vec<Dep> = r1_deps.iter().chain(&r2_deps).chain(&across).cloned().collect();
        let repos = [(r(1), true), (r(2), true)];
        let input = empty(&repos, &modules, &deps);
        let one = reading(&input, r(1)).expect("r1 reads");
        let two = reading(&input, r(2)).expect("r2 reads");
        let (c1, l1, m1, u1) = lens(&["a/x", "a/y", "a/z", "a/p", "a/q"], &r1_deps);
        let (c2, l2, m2, u2) = lens(&["b/m", "b/n", "b/o"], &r2_deps);
        assert_eq!((one.cycles, one.largest_cycle, one.up_edges), (c1, l1, u1.len() as i64));
        assert_eq!((two.cycles, two.largest_cycle, two.up_edges), (c2, l2, u2.len() as i64));
        assert_eq!(
            (c1, l1, c2, l2),
            (2, 3, 1, 2),
            "the fixture's shape, as the lens draws it per repository"
        );
        assert_eq!((one.findings.cycles, two.findings.cycles), (m1, m2));
        assert_eq!((one.findings.up_edges, two.findings.up_edges), (u1, u2));
        assert_eq!(
            (one.cross_repository_deps, two.cross_repository_deps),
            (1, 1),
            "a/p→b/o for r1, b/o→a/p for r2"
        );
    }

    /// Pain, hidden coupling and complexity outliers, each counted in its
    /// repository with the pain modules named; a coupled pair across
    /// repositories counts in neither; every hidden pair counts — not the lens's 40.
    #[test]
    #[ignore = "pending: #156"]
    fn pain_hidden_pairs_and_outliers_are_counted_per_repository() {
        let mut modules = vec![(s("a/x"), r(1)), (s("a/y"), r(1)), (s("b/m"), r(2))];
        // 41 more modules in r1, each hidden-coupled with a/x: more than the lens's 40.
        let extra: Vec<String> = (0..41).map(|i| format!("a/k{i:02}")).collect();
        modules.extend(extra.iter().map(|m| (m.clone(), r(1))));
        let pain = [s("a/x"), s("b/m")];
        let mut hidden = vec![(s("a/x"), s("a/y")), (s("a/y"), s("b/m"))];
        hidden.extend(extra.iter().map(|m| (s("a/x"), m.clone())));
        let complexity = [
            (s("a/x"), Complexity { functions: 10, complex: 3 }), // 30%: an outlier
            (s("a/y"), Complexity { functions: 10, complex: 2 }), // 20%: a warning, not an outlier
            (s("b/m"), Complexity { functions: 4, complex: 1 }),  // 25%: an outlier
        ];
        let input = ShapeInput {
            repositories: &[(r(1), true), (r(2), true)],
            modules: &modules,
            deps: &[],
            pain: &pain,
            hidden: &hidden,
            complexity: &complexity,
        };
        let one = reading(&input, r(1)).expect("r1");
        let two = reading(&input, r(2)).expect("r2");
        assert_eq!((one.pain_modules, one.hidden_pairs, one.complexity_outliers), (1, 42, 1));
        assert_eq!((two.pain_modules, two.hidden_pairs, two.complexity_outliers), (1, 0, 1));
        assert_eq!(
            (one.findings.pain.clone(), two.findings.pain.clone()),
            (vec![s("a/x")], vec![s("b/m")])
        );
        assert_eq!(
            (one.cycles, one.largest_cycle, one.up_edges),
            (0, 0, 0),
            "no cycle is a reading of zero"
        );
    }

    /// No reading where there is nothing to read: r2 is complete with no module;
    /// r3 has modules but its index is incomplete (G3: "0 cycles" there would read
    /// as a pass). Only r1 reads.
    #[test]
    #[ignore = "pending: #156"]
    fn an_incomplete_index_or_no_module_reads_nothing() {
        let modules = [(s("a/x"), r(1)), (s("c/x"), r(3)), (s("c/y"), r(3))];
        let deps = [Dep::new("c/x", "c/y", 1), Dep::new("c/y", "c/x", 1)];
        let repos = [(r(1), true), (r(2), true), (r(3), false)];
        let input = empty(&repos, &modules, &deps);
        let all = shape_by_repository(&input);
        assert_eq!(all.iter().map(|x| x.repository_id).collect::<Vec<_>>(), vec![r(1)]);
    }
}
