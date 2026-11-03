//! The architecture metrics the code map's lenses already show (#156) — counted
//! per repository from the same analysis, never re-derived: the Cycles lens's
//! cycles (`layering::analyse`), the Zones lens's zone of pain, Hidden coupling's
//! pairs (at the code map's own threshold), and Complexity's outliers (the tier
//! `analysis::codemap::Complexity` carries).
//!
//! Per repository, because metrics are stored per repository and a count pools by
//! summing: a workspace-wide number written to each repository would be counted
//! once per repository. A cycle or a coupled pair that spans two repositories is
//! counted in neither — each repository's numbers are its own.
//!
//! A repository with no module indexed reads NOTHING, not zeros: zero cycles is a
//! finding, while "not indexed" is the absence of one.

use crate::analysis::codemap::Complexity;
use crate::analysis::layering::Dep;

/// What the shape is read from, for one workspace.
#[allow(dead_code)] // read by `shape_by_repository` in #156 phase B
pub struct ShapeInput<'a> {
    /// Every module, with the repository it belongs to.
    pub modules: &'a [(String, uuid::Uuid)],
    /// Module dependencies, as the code map reads them.
    pub deps: &'a [Dep],
    /// The modules the Zones lens places in the zone of pain.
    pub pain: &'a [String],
    /// Hidden coupling's pairs, already at the code map's threshold.
    pub hidden: &'a [(String, String)],
    /// Each measured module's complexity.
    pub complexity: &'a [(String, Complexity)],
}

/// One repository's readings.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeReading {
    pub repository_id: uuid::Uuid,
    pub cycles: i64,
    /// Modules in the largest cycle; 0 when there is none.
    pub largest_cycle: i64,
    pub pain_modules: i64,
    pub hidden_pairs: i64,
    pub complexity_outliers: i64,
}

/// Each repository that has modules, read. Pending (#156 phase B).
#[allow(dead_code)]
pub fn shape_by_repository(_input: &ShapeInput<'_>) -> Vec<ShapeReading> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(n: u8) -> uuid::Uuid {
        uuid::Uuid::from_bytes([n; 16])
    }
    fn s(v: &str) -> String {
        v.to_string()
    }
    fn reading(input: &ShapeInput<'_>, repo: uuid::Uuid) -> Option<ShapeReading> {
        shape_by_repository(input).into_iter().find(|x| x.repository_id == repo)
    }

    /// The Cycles lens's cycles, counted where they live: r1 holds a 3-cycle and a
    /// 2-cycle; r2 a 2-cycle. A cycle through two repositories counts in neither.
    #[test]
    #[ignore = "pending: #156"]
    fn cycles_and_the_largest_are_counted_within_each_repository() {
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
        let deps = [
            Dep::new("a/x", "a/y", 3),
            Dep::new("a/y", "a/z", 2),
            Dep::new("a/z", "a/x", 1),
            Dep::new("a/p", "a/q", 1),
            Dep::new("a/q", "a/p", 1),
            Dep::new("b/m", "b/n", 1),
            Dep::new("b/n", "b/m", 1),
            // A loop across the two repositories is no one repository's cycle.
            Dep::new("b/o", "a/p", 1),
            Dep::new("a/p", "b/o", 1),
        ];
        let input =
            ShapeInput { modules: &modules, deps: &deps, pain: &[], hidden: &[], complexity: &[] };
        let one = reading(&input, r(1)).expect("r1 has modules");
        let two = reading(&input, r(2)).expect("r2 has modules");
        assert_eq!((one.cycles, one.largest_cycle), (2, 3));
        assert_eq!((two.cycles, two.largest_cycle), (1, 2));
    }

    /// Pain, hidden coupling and complexity outliers, each counted in its
    /// repository; a coupled pair across repositories counts in neither.
    #[test]
    #[ignore = "pending: #156"]
    fn pain_hidden_pairs_and_outliers_are_counted_per_repository() {
        let modules = [(s("a/x"), r(1)), (s("a/y"), r(1)), (s("b/m"), r(2))];
        let pain = [s("a/x"), s("b/m")];
        let hidden = [(s("a/x"), s("a/y")), (s("a/y"), s("b/m"))];
        let complexity = [
            (s("a/x"), Complexity { functions: 10, complex: 3 }), // 30%: an outlier
            (s("a/y"), Complexity { functions: 10, complex: 2 }), // 20%: a warning, not an outlier
            (s("b/m"), Complexity { functions: 4, complex: 1 }),  // 25%: an outlier
        ];
        let input = ShapeInput {
            modules: &modules,
            deps: &[],
            pain: &pain,
            hidden: &hidden,
            complexity: &complexity,
        };
        let one = reading(&input, r(1)).expect("r1");
        let two = reading(&input, r(2)).expect("r2");
        assert_eq!((one.pain_modules, one.hidden_pairs, one.complexity_outliers), (1, 1, 1));
        assert_eq!((two.pain_modules, two.hidden_pairs, two.complexity_outliers), (1, 0, 1));
        assert_eq!((one.cycles, one.largest_cycle), (0, 0), "no cycle is a reading of zero");
    }

    /// A repository with no module indexed reads nothing — not a row of zeros.
    #[test]
    #[ignore = "pending: #156"]
    fn a_repository_with_no_modules_reads_nothing() {
        let modules = [(s("a/x"), r(1))];
        let input =
            ShapeInput { modules: &modules, deps: &[], pain: &[], hidden: &[], complexity: &[] };
        let all = shape_by_repository(&input);
        assert_eq!(all.iter().map(|x| x.repository_id).collect::<Vec<_>>(), vec![r(1)]);
    }
}
