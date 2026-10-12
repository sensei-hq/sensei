//! Nesting every indexed declaration by what contains it (#219).
//!
//! The World diagram is containment as area: a circle per project, repository
//! and test-or-code split, sized by how many declarations it holds. `@rokkit/graph`'s
//! `world` layout draws it from a flat list of nodes, each carrying its OWN full
//! path — outermost first and including itself — so a container is an ordinary
//! node that other nodes' paths extend. Nothing here mints a parent id, and a
//! path cannot dangle the way one would.
//!
//! ## Three groupings, one set of cells
//!
//! The screen offers "group first by" project, repository, or code-vs-tests.
//! They are three ORDERINGS of the same three dimensions, not three queries:
//! the chosen dimension leads and the rest follow in a fixed order. One read,
//! one statement the SQL planner can check, and the grouping is a pure function
//! with tests.
//!
//! ## WHY THERE IS NO "DOCS" GROUPING
//!
//! The mockup offers "Code · tests · docs". Only the first two exist as facts:
//! `nodes.is_test` is a column, and a documentation FILE is not a declaration —
//! `sensei.nodes` holds no markdown, and the 31,736 files skipped as
//! `unsupported_format` are never walked. Offering an always-empty "docs" ring
//! would be a lie the picture tells confidently, so the grouping is
//! `CodeOrTests` and documentation appears where it IS a fact: as the
//! `documented` SHARE, which is per-declaration and real.
//!
//! ## The unresolved share is undefined at the leaf, and says so
//!
//! Edges belong to a FOLDER; declarations belong to a (folder, is_test) cell.
//! There is no way to say how many of a repository's unresolved edges came from
//! its tests, so the share is `None` below the repository ring rather than a
//! number split on a guess. `None` renders as unshaded; a fabricated half would
//! render as a finding.

use std::collections::BTreeMap;

/// Which dimension the reader asked to see first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum GroupFirst {
    Project,
    Repository,
    /// Code against tests. NOT "code · tests · docs" — see the module docs.
    CodeOrTests,
}

impl GroupFirst {
    /// The wire spelling, and the one the screen's control sends back.
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "project" => Some(Self::Project),
            "repository" => Some(Self::Repository),
            "kind" => Some(Self::CodeOrTests),
            _ => None,
        }
    }

    pub fn as_label(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Repository => "repository",
            Self::CodeOrTests => "kind",
        }
    }
}

/// One (project, repository, test-or-not) cell, as the database holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub project: String,
    pub repository: String,
    /// Which folder the repository is, so a cell can find its edge placement.
    /// A repository NAME is not unique — two projects can hold repositories of
    /// one name, and one repository can belong to two projects.
    pub folder: uuid::Uuid,
    pub is_test: bool,
    pub declarations: i64,
    pub documented: i64,
}

/// One repository's edges, keyed by folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    pub folder: uuid::Uuid,
    pub edges: i64,
    pub missed: i64,
}

/// One circle.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Unit {
    /// The path joined — stable, and unique because the path is.
    pub id: String,
    /// The last segment: what this circle is called in its parent.
    pub label: String,
    /// Outermost first, INCLUDING itself, which is what the layout reads.
    pub path: Vec<String>,
    /// Declarations held, directly or through what it contains.
    pub weight: i64,
    /// What the shade control can colour by. Independent measures with
    /// different ranges, which is why they are a bag rather than fields.
    pub measures: Measures,
}

/// Shares in `0.0..=1.0`, or `None` where the share is not a fact.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Measures {
    /// Declarations carrying a docstring.
    pub documented_share: f64,
    /// Declarations the walk marked as test code.
    pub test_share: f64,
    /// Edges that named a target and did not reach one.
    ///
    /// `None` below the repository ring, where the question has no answer: see
    /// the module docs. Never 0.0 as a stand-in — an unshaded circle and a
    /// perfectly-resolved one must not look alike.
    pub unresolved_share: Option<f64>,
}

/// What the whole picture adds up to, for the panel beside it.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Totals {
    pub declarations: i64,
    pub documented: i64,
    pub tests: i64,
    pub repositories: i64,
    pub projects: i64,
}

/// The circles to draw, and what they add up to.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct World {
    pub units: Vec<Unit>,
    pub totals: Totals,
}

/// The bucket a cell's declarations sit in.
fn bucket(is_test: bool) -> &'static str {
    match is_test {
        true => "tests",
        false => "code",
    }
}

/// The path a cell occupies under one grouping, outermost first.
///
/// The three orderings are PERMUTATIONS of one set, with the chosen dimension
/// leading. Written as one function so a fourth grouping is one arm rather than
/// a fourth tree builder.
fn path_of(cell: &Cell, group: GroupFirst) -> Vec<String> {
    let project = cell.project.clone();
    let repository = cell.repository.clone();
    let kind = bucket(cell.is_test).to_string();
    match group {
        GroupFirst::Project => vec![project, repository, kind],
        GroupFirst::Repository => vec![repository, project, kind],
        GroupFirst::CodeOrTests => vec![kind, project, repository],
    }
}

/// How deep the repository sits under a grouping — the last ring at which an
/// edge share is a fact.
///
/// Below it a circle is part of one repository and the edges cannot be split;
/// at or above it, a circle is a whole number of repositories and their edges
/// add up. Derived from the ordering rather than written per arm, so the two
/// cannot disagree.
fn repository_depth(group: GroupFirst) -> usize {
    match group {
        GroupFirst::Project => 2,
        GroupFirst::Repository => 1,
        GroupFirst::CodeOrTests => 3,
    }
}

#[derive(Default)]
struct Tally {
    declarations: i64,
    documented: i64,
    tests: i64,
    folders: std::collections::BTreeSet<uuid::Uuid>,
}

/// Nest the cells (#219).
///
/// Every prefix of every cell's path becomes a circle, so a container exists
/// because something is inside it and never because a parent id said so.
pub fn nest(cells: &[Cell], placements: &[Placement], group: GroupFirst) -> World {
    let edges_of: BTreeMap<uuid::Uuid, Placement> =
        placements.iter().map(|p| (p.folder, *p)).collect();

    // Keyed by the path, which IS the identity. A `BTreeMap` so the output is
    // ordered and a caller comparing two runs compares like with like.
    let mut tallies: BTreeMap<Vec<String>, Tally> = BTreeMap::new();
    for cell in cells {
        let path = path_of(cell, group);
        for depth in 1..=path.len() {
            let entry = tallies.entry(path[..depth].to_vec()).or_default();
            entry.declarations += cell.declarations;
            entry.documented += cell.documented;
            if cell.is_test {
                entry.tests += cell.declarations;
            }
            // The FOLDER SET, not a count. One repository contributes two cells
            // — code and tests — and adding its edges once per cell would
            // report them twice.
            entry.folders.insert(cell.folder);
        }
    }

    let depth_of_repository = repository_depth(group);
    let units: Vec<Unit> = tallies
        .iter()
        .map(|(path, tally)| {
            let share = |n: i64| match tally.declarations {
                0 => 0.0,
                total => n as f64 / total as f64,
            };
            // Only at or above the repository ring, and only when the folders in
            // it carry edges at all: a repository nothing links is unshaded
            // rather than perfectly resolved.
            let unresolved_share = (path.len() <= depth_of_repository)
                .then(|| {
                    let (edges, missed) = tally
                        .folders
                        .iter()
                        .filter_map(|f| edges_of.get(f))
                        .fold((0i64, 0i64), |(e, m), p| (e + p.edges, m + p.missed));
                    (edges > 0).then(|| missed as f64 / edges as f64)
                })
                .flatten();
            Unit {
                id: path.join(" / "),
                label: path.last().cloned().unwrap_or_default(),
                path: path.clone(),
                weight: tally.declarations,
                measures: Measures {
                    documented_share: share(tally.documented),
                    test_share: share(tally.tests),
                    unresolved_share,
                },
            }
        })
        .collect();

    let totals = Totals {
        declarations: cells.iter().map(|c| c.declarations).sum(),
        documented: cells.iter().map(|c| c.documented).sum(),
        tests: cells.iter().filter(|c| c.is_test).map(|c| c.declarations).sum(),
        repositories: cells
            .iter()
            .map(|c| c.folder)
            .collect::<std::collections::BTreeSet<_>>()
            .len() as i64,
        projects: cells
            .iter()
            .map(|c| c.project.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .len() as i64,
    };

    World { units, totals }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(n: u8) -> uuid::Uuid {
        uuid::Uuid::from_bytes([n; 16])
    }

    /// Two projects, one shared repository, each with a code and a test cell.
    fn corpus() -> (Vec<Cell>, Vec<Placement>) {
        let cells = vec![
            Cell {
                project: "alpha".into(),
                repository: "core".into(),
                folder: folder(1),
                is_test: false,
                declarations: 80,
                documented: 8,
            },
            Cell {
                project: "alpha".into(),
                repository: "core".into(),
                folder: folder(1),
                is_test: true,
                declarations: 20,
                documented: 0,
            },
            Cell {
                project: "beta".into(),
                repository: "core".into(),
                folder: folder(1),
                is_test: false,
                declarations: 80,
                documented: 8,
            },
            Cell {
                project: "beta".into(),
                repository: "web".into(),
                folder: folder(2),
                is_test: false,
                declarations: 50,
                documented: 25,
            },
        ];
        let placements = vec![
            Placement { folder: folder(1), edges: 200, missed: 50 },
            Placement { folder: folder(2), edges: 100, missed: 10 },
        ];
        (cells, placements)
    }

    fn unit<'a>(w: &'a World, id: &str) -> &'a Unit {
        w.units.iter().find(|u| u.id == id).unwrap_or_else(|| {
            panic!("no unit {id}: {:?}", w.units.iter().map(|u| &u.id).collect::<Vec<_>>())
        })
    }

    /// A CONTAINER WEIGHS WHAT IT HOLDS, at every ring.
    ///
    /// The property the picture is: area is containment. A parent that did not
    /// sum its children would draw a circle smaller than the circles inside it.
    ///
    /// Mutation that must break this test: accumulate only at the leaf depth.
    #[test]
    fn every_ring_weighs_what_is_inside_it() {
        let (cells, placements) = corpus();
        let w = nest(&cells, &placements, GroupFirst::Project);

        assert_eq!(unit(&w, "alpha").weight, 100, "80 code + 20 tests");
        assert_eq!(unit(&w, "alpha / core").weight, 100);
        assert_eq!(unit(&w, "alpha / core / code").weight, 80);
        assert_eq!(unit(&w, "alpha / core / tests").weight, 20);
        assert_eq!(unit(&w, "beta").weight, 130, "80 in core + 50 in web");
    }

    /// THE PATH INCLUDES THE CIRCLE ITSELF, outermost first.
    ///
    /// What `@rokkit/graph`'s `world` layout reads. A path that stopped at the
    /// parent would nest every circle one ring too shallow.
    #[test]
    fn a_path_names_itself_last() {
        let (cells, placements) = corpus();
        let w = nest(&cells, &placements, GroupFirst::Project);
        let u = unit(&w, "alpha / core / tests");
        assert_eq!(u.path, vec!["alpha", "core", "tests"]);
        assert_eq!(u.label, "tests", "the label is what it is called in its parent");
    }

    /// THE GROUPINGS ARE PERMUTATIONS, and every one covers the same weight.
    ///
    /// Re-ordering the rings cannot change how much code there is. A grouping
    /// that lost or duplicated a cell would show a different total for the same
    /// corpus, which is the failure a reader cannot see.
    ///
    /// Mutation that must break this test: give one arm of `path_of` a
    /// different set of dimensions.
    #[test]
    fn every_grouping_covers_the_same_corpus() {
        let (cells, placements) = corpus();
        for group in [GroupFirst::Project, GroupFirst::Repository, GroupFirst::CodeOrTests] {
            let w = nest(&cells, &placements, group);
            let roots: i64 = w.units.iter().filter(|u| u.path.len() == 1).map(|u| u.weight).sum();
            assert_eq!(roots, 230, "{group:?}: the outermost ring holds the whole corpus");
            assert_eq!(w.totals.declarations, 230, "{group:?}");
        }
    }

    /// A REPOSITORY'S EDGES ARE COUNTED ONCE, not once per cell.
    ///
    /// `core` contributes a code cell and a test cell, and its 200 edges belong
    /// to the FOLDER. Adding them per cell would report 400 and halve the
    /// unresolved share — a number that looks precise and is wrong.
    ///
    /// Mutation that must break this test: sum `edges_of` per cell instead of
    /// over the folder set.
    #[test]
    fn a_repositorys_edges_are_counted_once_however_many_cells_it_has() {
        let (cells, placements) = corpus();
        let w = nest(&cells, &placements, GroupFirst::Project);
        // alpha holds only `core`: 50 missed of 200.
        assert_eq!(unit(&w, "alpha").measures.unresolved_share, Some(0.25));
        assert_eq!(unit(&w, "alpha / core").measures.unresolved_share, Some(0.25));
        // beta holds core AND web: (50 + 10) of (200 + 100).
        assert_eq!(unit(&w, "beta").measures.unresolved_share, Some(0.2));
    }

    /// BELOW THE REPOSITORY, THE SHARE IS `None` AND NEVER ZERO.
    ///
    /// Edges belong to a folder and declarations to a (folder, is_test) cell,
    /// so there is no way to say how many of a repository's unresolved edges
    /// came from its tests. An unshaded circle and a perfectly-resolved one must
    /// not look alike.
    ///
    /// Mutation that must break this test: default the share to `Some(0.0)`, or
    /// drop the depth guard.
    #[test]
    fn the_unresolved_share_is_none_below_the_repository_ring() {
        let (cells, placements) = corpus();
        let w = nest(&cells, &placements, GroupFirst::Project);
        assert_eq!(unit(&w, "alpha / core / code").measures.unresolved_share, None);
        assert_eq!(unit(&w, "alpha / core / tests").measures.unresolved_share, None);

        // ...and the ring it IS defined at moves with the grouping, because the
        // repository sits at a different depth in each ordering.
        let by_repo = nest(&cells, &placements, GroupFirst::Repository);
        assert_eq!(unit(&by_repo, "core").measures.unresolved_share, Some(0.25));
        assert_eq!(unit(&by_repo, "core / alpha").measures.unresolved_share, None);
    }

    /// A repository nothing links is UNSHADED, not perfectly resolved.
    #[test]
    fn a_repository_with_no_edges_has_no_share() {
        let cells = vec![Cell {
            project: "solo".into(),
            repository: "quiet".into(),
            folder: folder(9),
            is_test: false,
            declarations: 5,
            documented: 0,
        }];
        let w = nest(&cells, &[], GroupFirst::Project);
        assert_eq!(unit(&w, "solo").measures.unresolved_share, None);
    }

    /// The documented and test shares are per-declaration and real.
    #[test]
    fn the_declaration_shares_are_weighted_by_declarations() {
        let (cells, placements) = corpus();
        let w = nest(&cells, &placements, GroupFirst::Project);
        // alpha: 8 documented of 100, 20 tests of 100.
        assert_eq!(unit(&w, "alpha").measures.documented_share, 0.08);
        assert_eq!(unit(&w, "alpha").measures.test_share, 0.2);
        // beta has no tests at all, which is a share of zero and not an absence.
        assert_eq!(unit(&w, "beta").measures.test_share, 0.0);
    }

    /// ONE REPOSITORY IN TWO PROJECTS IS COUNTED ONCE in the totals.
    ///
    /// Membership comes through `folder_projects`, so `core` legitimately
    /// appears under both `alpha` and `beta`. The picture draws it twice — that
    /// is what the rings mean — but "how many repositories are there" has one
    /// answer.
    ///
    /// Mutation that must break this test: count rows instead of distinct
    /// folders.
    #[test]
    fn a_shared_repository_is_drawn_twice_and_counted_once() {
        let (cells, placements) = corpus();
        let w = nest(&cells, &placements, GroupFirst::Project);
        assert_eq!(w.totals.repositories, 2, "core and web, however many projects hold them");
        assert_eq!(w.totals.projects, 2);
        assert!(w.units.iter().any(|u| u.id == "alpha / core"));
        assert!(w.units.iter().any(|u| u.id == "beta / core"));
    }

    /// The wire vocabulary round-trips, so the control and the handler cannot
    /// drift apart.
    #[test]
    fn every_grouping_survives_its_label() {
        for group in [GroupFirst::Project, GroupFirst::Repository, GroupFirst::CodeOrTests] {
            assert_eq!(GroupFirst::from_label(group.as_label()), Some(group));
        }
        assert_eq!(GroupFirst::from_label("docs"), None, "there is no docs grouping");
    }
}
