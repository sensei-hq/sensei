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

use std::collections::HashSet;

use crate::analysis::codemap::{Complexity, ComplexityTier};
use crate::analysis::layering::{Conformance, Dep, analyse};

/// What the shape is read from, for one workspace.
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

/// One stored metric row of a reading (review r2 H1): what the database keeps is
/// the contract — TG4 reads base findings from these props. Repository-wide
/// readings: scope `repo`, no identity, no commit.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeRow {
    /// The repository the reading is of — the only thing that tells two
    /// repositories' rows apart (no identity, no commit).
    pub repository_id: uuid::Uuid,
    pub key: &'static str,
    pub value: f64,
    pub props: serde_json::Value,
    pub scope: &'static str,
    pub identity: Option<String>,
    pub commit_sha: Option<String>,
    pub source: &'static str,
    pub grain: &'static str,
}

/// A reading's rows, one per catalogued key — all five always, a clean
/// repository's at 0, so a target can be met on it.
pub fn shape_rows(reading: &ShapeReading) -> Vec<ShapeRow> {
    let f = &reading.findings;
    let row = |key: &'static str, value: i64, props: serde_json::Value| ShapeRow {
        repository_id: reading.repository_id,
        key,
        value: value as f64,
        props,
        scope: "repo",
        identity: None,
        commit_sha: None,
        source: "measured",
        grain: "daily",
    };
    vec![
        row(
            "dependency_cycles",
            reading.cycles,
            serde_json::json!({
                "cycles": f.cycles,
                "largest": reading.largest_cycle,
                "cross_repository_deps": reading.cross_repository_deps,
            }),
        ),
        row(
            "layer_up_edges",
            reading.up_edges,
            serde_json::json!({ "up_edges": f.up_edges, "cross_repository_deps": reading.cross_repository_deps }),
        ),
        row("zone_of_pain_modules", reading.pain_modules, serde_json::json!({ "modules": f.pain })),
        row("hidden_coupling_pairs", reading.hidden_pairs, serde_json::json!({})),
        row("complexity_outlier_modules", reading.complexity_outliers, serde_json::json!({})),
    ]
}

/// Each complete repository that has modules, read on its own subgraph.
pub fn shape_by_repository(input: &ShapeInput<'_>) -> Vec<ShapeReading> {
    let home: std::collections::HashMap<&str, uuid::Uuid> =
        input.modules.iter().map(|(m, r)| (m.as_str(), *r)).collect();
    let mut readings = Vec::new();
    for (repo, complete) in input.repositories {
        if !complete {
            continue; // an incomplete index is not measured
        }
        let mut units: Vec<String> =
            input.modules.iter().filter(|(_, r)| r == repo).map(|(m, _)| m.clone()).collect();
        if units.is_empty() {
            continue; // nothing indexed here to read
        }
        units.sort();
        let mine: HashSet<&str> = units.iter().map(String::as_str).collect();
        // Its own subgraph: a dependency on another repository's module is not
        // followed, and counted; one on no known module is left to `analyse`.
        let mut cross = 0i64;
        let mut deps: Vec<Dep> = Vec::new();
        for d in input.deps.iter().filter(|d| mine.contains(d.source.as_str())) {
            match home.get(d.target.as_str()) {
                Some(r) if r != repo => cross += 1,
                _ => deps.push(d.clone()),
            }
        }
        let layering = analyse(&units, &deps);
        let cycles: Vec<Vec<String>> = layering.cycles.iter().map(|c| c.members.clone()).collect();
        let mut up: Vec<String> = layering
            .deps
            .iter()
            .filter(|d| d.conformance == Conformance::Up)
            .map(|d| format!("{}→{}", d.source, d.target))
            .collect();
        up.sort();
        let mut pain: Vec<String> =
            input.pain.iter().filter(|m| mine.contains(m.as_str())).cloned().collect();
        pain.sort();
        let hidden = input
            .hidden
            .iter()
            .filter(|(a, b)| mine.contains(a.as_str()) && mine.contains(b.as_str()))
            .count() as i64;
        let outliers = input
            .complexity
            .iter()
            .filter(|(m, c)| mine.contains(m.as_str()) && c.tier() == Some(ComplexityTier::Danger))
            .count() as i64;
        readings.push(ShapeReading {
            repository_id: *repo,
            cycles: cycles.len() as i64,
            largest_cycle: cycles.first().map_or(0, |c| c.len() as i64),
            up_edges: up.len() as i64,
            pain_modules: pain.len() as i64,
            hidden_pairs: hidden,
            complexity_outliers: outliers,
            cross_repository_deps: cross,
            findings: ShapeFindings { cycles, up_edges: up, pain },
        });
    }
    readings
}

/// Each module's one repository, deduplicated; a module in two is an error (G9).
pub fn module_homes(rows: Vec<(String, uuid::Uuid)>) -> Result<Vec<(String, uuid::Uuid)>, String> {
    let mut home: std::collections::BTreeMap<String, uuid::Uuid> =
        std::collections::BTreeMap::new();
    for (module, repo) in rows {
        match home.get(&module) {
            Some(r) if *r != repo => {
                return Err(format!("module {module} is in two repositories ({r} and {repo})"));
            }
            _ => {
                home.insert(module, repo);
            }
        }
    }
    Ok(home.into_iter().collect())
}

/// A stored row as the store takes it — the one conversion (review r3 R3-2).
fn metric_row<'a>(
    row: &'a ShapeRow,
    metric_id: &'a uuid::Uuid,
    day: chrono::NaiveDate,
) -> crate::db::pg_store::MetricRow<'a> {
    crate::db::pg_store::MetricRow {
        metric_id,
        repository_id: &row.repository_id,
        scope: row.scope,
        identity: row.identity.as_deref(),
        commit_sha: row.commit_sha.as_deref(),
        computed_on: day,
        grain: row.grain,
        value: row.value,
        props: &row.props,
        source: row.source,
    }
}

/// Read the workspace's shape and write its rows (#156): the same reads the
/// lenses make — the module graph, the Zones lens's pain, hidden coupling at its
/// threshold (every pair, not the lens's 40), the module complexity — and each
/// repository's index readiness. Any failed read fails the task (D83).
pub async fn write_shape(
    pg: &crate::db::pg_store::PgStore,
    workspace_id: &uuid::Uuid,
    day: chrono::NaiveDate,
    ids: &std::collections::HashMap<String, uuid::Uuid>,
) -> Result<u32, String> {
    use crate::analysis::codemap::{hidden_pairs, module_dependencies};
    const KEYS: [&str; 5] = [
        "dependency_cycles",
        "layer_up_edges",
        "zone_of_pain_modules",
        "hidden_coupling_pairs",
        "complexity_outlier_modules",
    ];
    if !KEYS.iter().any(|k| ids.contains_key(*k)) {
        return Ok(0); // none of them active: nothing to read
    }
    let (homes, deps, zones, (hidden, _, _), complexity, ready) = tokio::try_join!(
        pg.architecture_module_repositories(workspace_id),
        module_dependencies(pg, workspace_id),
        pg.component_zones(workspace_id),
        hidden_pairs(pg, workspace_id, usize::MAX),
        pg.codemap_module_complexity(workspace_id),
        pg.repository_index_complete(workspace_id),
    )?;
    let modules = module_homes(homes)?;
    let pain: Vec<String> = zones
        .into_iter()
        .filter(|z| z.zone.as_deref() == Some("pain"))
        .map(|z| z.component)
        .collect();
    let hidden: Vec<(String, String)> = hidden.into_iter().map(|c| (c.a, c.b)).collect();
    let complexity: Vec<(String, Complexity)> = complexity
        .into_iter()
        .map(|(m, (functions, complex))| (m, Complexity { functions, complex }))
        .collect();
    let input = ShapeInput {
        repositories: &ready,
        modules: &modules,
        deps: &deps,
        pain: &pain,
        hidden: &hidden,
        complexity: &complexity,
    };
    let mut written = 0u32;
    for reading in shape_by_repository(&input) {
        for row in shape_rows(&reading) {
            if let Some(mid) = ids.get(row.key) {
                pg.upsert_workspace_metric_repo(&metric_row(&row, mid, day)).await?;
                written += 1;
            }
        }
    }
    Ok(written)
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
    fn an_incomplete_index_or_no_module_reads_nothing() {
        let modules = [(s("a/x"), r(1)), (s("c/x"), r(3)), (s("c/y"), r(3))];
        let deps = [Dep::new("c/x", "c/y", 1), Dep::new("c/y", "c/x", 1)];
        let repos = [(r(1), true), (r(2), true), (r(3), false)];
        let input = empty(&repos, &modules, &deps);
        let all = shape_by_repository(&input);
        assert_eq!(all.iter().map(|x| x.repository_id).collect::<Vec<_>>(), vec![r(1)]);
    }

    /// Review r2 H1: what is stored is the contract. Each key's value, its props
    /// (TG4 reads base findings from them: cycles by member set, the Up-edge count,
    /// the pain modules), and a repository-wide row — scope `repo`, no identity, no
    /// commit. "Largest" is a prop of `dependency_cycles`, never a summed count (H2).
    #[test]
    fn the_rows_store_each_key_its_value_and_findings() {
        let reading = ShapeReading {
            repository_id: r(2),
            cycles: 2,
            largest_cycle: 3,
            up_edges: 3,
            pain_modules: 1,
            hidden_pairs: 4,
            complexity_outliers: 2,
            cross_repository_deps: 1,
            findings: ShapeFindings {
                cycles: vec![vec![s("a/x"), s("a/y"), s("a/z")], vec![s("a/p"), s("a/q")]],
                up_edges: vec![s("a/q→a/p"), s("a/y→a/x"), s("a/z→a/y")],
                pain: vec![s("a/x")],
            },
        };
        let rows = shape_rows(&reading);
        let got: Vec<(&str, f64, serde_json::Value)> =
            rows.iter().map(|r| (r.key, r.value, r.props.clone())).collect();
        assert_eq!(
            got,
            vec![
                (
                    "dependency_cycles",
                    2.0,
                    serde_json::json!({"cycles": [["a/x", "a/y", "a/z"], ["a/p", "a/q"]], "largest": 3, "cross_repository_deps": 1})
                ),
                (
                    "layer_up_edges",
                    3.0,
                    serde_json::json!({"up_edges": ["a/q→a/p", "a/y→a/x", "a/z→a/y"], "cross_repository_deps": 1})
                ),
                ("zone_of_pain_modules", 1.0, serde_json::json!({"modules": ["a/x"]})),
                ("hidden_coupling_pairs", 4.0, serde_json::json!({})),
                ("complexity_outlier_modules", 2.0, serde_json::json!({})),
            ]
        );
        assert!(
            rows.iter()
                .all(|r| r.scope == "repo" && r.identity.is_none() && r.commit_sha.is_none())
        );
        // Review r3 R3-2: each row is its repository's — and a measured daily reading.
        assert!(rows.iter().all(|row| row.repository_id == r(2)
            && row.source == "measured"
            && row.grain == "daily"));
    }

    /// Review r3 R3-1: a complete, clean repository is a reading of zero, not a
    /// missing one — all five rows are stored, each 0, with empty findings, so a
    /// target can be met (a missing row reads "not measured" forever).
    #[test]
    fn a_clean_reading_still_stores_all_five_rows_at_zero() {
        let reading = ShapeReading {
            repository_id: r(3),
            cycles: 0,
            largest_cycle: 0,
            up_edges: 0,
            pain_modules: 0,
            hidden_pairs: 0,
            complexity_outliers: 0,
            cross_repository_deps: 0,
            findings: ShapeFindings::default(),
        };
        let got: Vec<(&str, f64, serde_json::Value)> =
            shape_rows(&reading).iter().map(|r| (r.key, r.value, r.props.clone())).collect();
        assert_eq!(
            got,
            vec![
                (
                    "dependency_cycles",
                    0.0,
                    serde_json::json!({"cycles": [], "largest": 0, "cross_repository_deps": 0})
                ),
                (
                    "layer_up_edges",
                    0.0,
                    serde_json::json!({"up_edges": [], "cross_repository_deps": 0})
                ),
                ("zone_of_pain_modules", 0.0, serde_json::json!({"modules": []})),
                ("hidden_coupling_pairs", 0.0, serde_json::json!({})),
                ("complexity_outlier_modules", 0.0, serde_json::json!({})),
            ]
        );
    }

    /// Review r2 H3: a component that needs two feedback arcs has more Up edges
    /// than cycles. `a/x↔a/y` plus `a/y↔a/z` is one cycle of three with two Up
    /// edges (worked through `feedback_order`: a/x, a/y, a/z, so the back edges are
    /// a/y→a/x and a/z→a/y) — absolute numbers, beside the lens's own.
    #[test]
    fn two_feedback_arcs_in_one_cycle_are_two_up_edges() {
        let names = ["a/x", "a/y", "a/z"];
        let modules: Vec<(String, uuid::Uuid)> = names.iter().map(|m| (s(m), r(1))).collect();
        let deps = [
            Dep::new("a/x", "a/y", 1),
            Dep::new("a/y", "a/x", 1),
            Dep::new("a/y", "a/z", 1),
            Dep::new("a/z", "a/y", 1),
        ];
        let repos = [(r(1), true)];
        let got = reading(&empty(&repos, &modules, &deps), r(1)).expect("r1 reads");
        assert_eq!((got.cycles, got.largest_cycle, got.up_edges), (1, 3, 2));
        assert_eq!(got.findings.up_edges, vec![s("a/y→a/x"), s("a/z→a/y")]);
        let (cycles, largest, _, up) = lens(&names, &deps);
        assert_eq!(
            (got.cycles, got.largest_cycle, got.findings.up_edges.clone()),
            (cycles, largest, up)
        );
    }

    /// Review G9: a module the read places in two repositories is an error, not
    /// whichever came first — its numbers would be counted in one of them by chance.
    #[test]
    fn a_module_in_two_repositories_is_refused() {
        let ok = module_homes(vec![(s("a/x"), r(1)), (s("b/y"), r(2)), (s("a/x"), r(1))]).unwrap();
        assert_eq!(ok, vec![(s("a/x"), r(1)), (s("b/y"), r(2))]);
        let err = module_homes(vec![(s("a/x"), r(1)), (s("a/x"), r(2))]).unwrap_err();
        assert!(err.contains("a/x"), "{err}");
    }

    /// Review L4: readiness is the repository's root folders' `subtree_complete`.
    /// A folder whose expected file count is unknown is not complete — "0 cycles"
    /// there would be a reading of nothing — and it is complete once every expected
    /// file is decided.
    #[tokio::test]
    async fn readiness_is_incomplete_until_the_index_has_decided_every_file() {
        let pg = crate::db::pg_store::PgStore::test_store().await;
        let pid = pg
            .create_workspace(&format!("_test:ready:{}", uuid::Uuid::new_v4()), None, None)
            .await
            .unwrap();
        let rid =
            crate::tasks::test_support::seed_bare_repository(&pg, &pid, &uuid::Uuid::new_v4())
                .await;
        let read = |pg: &crate::db::pg_store::PgStore| {
            let pg = pg.clone();
            async move { pg.repository_index_complete(&pid).await.unwrap() }
        };
        assert_eq!(read(&pg).await, vec![(rid, false)], "no expected file count: not complete");
        sqlx_core::query::query(
            "UPDATE sensei.folders SET props = props || '{\"expected_files\": 0}'::jsonb WHERE repository_id = $1",
        )
        .bind(rid)
        .execute(pg.pool())
        .await
        .unwrap();
        assert_eq!(read(&pg).await, vec![(rid, true)], "every expected file decided: complete");
        sqlx_core::query::query("DELETE FROM sensei.workspaces WHERE id = $1")
            .bind(pid)
            .execute(pg.pool())
            .await
            .unwrap();
    }

    /// End to end (#156): a fully indexed repository whose two modules depend on
    /// each other stores all five rows — the cycle with its members, the one Up
    /// edge, and zeros for the rest — for that repository, scope repo, read
    /// through the same reads the lenses make.
    #[tokio::test]
    async fn the_shape_of_an_indexed_repository_is_stored_as_its_five_rows() {
        use crate::db::pg_store::graph_seed::SeedGraph;
        let pg = crate::db::pg_store::PgStore::test_store().await;
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) =
            crate::tasks::test_support::seed_metrics_workspace_folder(&pg, &uniq).await;
        let pkg = format!("sh{}", uniq.simple());
        let def = |file: &'static str| crate::db::pg_store::FqnDef {
            file_path: file,
            signature: None,
            line_start: Some(1),
            line_end: Some(2),
            is_exported: true,
            parent_id: None,
        };
        let a = pg
            .seed_node_by_fqn(
                &fid,
                &format!("rust·{pkg}·alpha·a"),
                "function",
                "a",
                Some("rust"),
                Some(def("alpha/a.rs")),
            )
            .await
            .unwrap();
        let b = pg
            .seed_node_by_fqn(
                &fid,
                &format!("rust·{pkg}·beta·b"),
                "function",
                "b",
                Some("rust"),
                Some(def("beta/b.rs")),
            )
            .await
            .unwrap();
        for (from, to) in [(&a, &b), (&b, &a)] {
            pg.insert_edge(&fid, from, crate::db::pg_store::EdgeTarget::Node(to), "calls")
                .await
                .unwrap();
        }
        // Fully indexed: every expected file decided.
        let run = |sql: &'static str| sqlx_core::query::query(sql);
        run("UPDATE sensei.files SET parsed_at = now() WHERE folder_id = $1")
            .bind(fid)
            .execute(pg.pool())
            .await
            .unwrap();
        run("UPDATE sensei.folders SET props = props || jsonb_build_object('expected_files', (SELECT count(*) FROM sensei.files WHERE folder_id = $1)) WHERE id = $1")
            .bind(fid)
            .execute(pg.pool())
            .await
            .unwrap();
        let (rid,): (uuid::Uuid,) =
            sqlx_core::query_as::query_as("SELECT repository_id FROM sensei.folders WHERE id = $1")
                .bind(fid)
                .fetch_one(pg.pool())
                .await
                .unwrap();
        let ids = pg.active_metric_ids("architecture").await.unwrap();
        let day = chrono::NaiveDate::from_ymd_opt(2020, 7, 1).unwrap();
        write_shape(&pg, &pid, day, &ids).await.unwrap();

        let rows: Vec<(String, f64, serde_json::Value, String)> = sqlx_core::query_as::query_as(
            "SELECT m.key, rm.value::float8, rm.props, rm.scope::text
               FROM sensei.repository_metrics rm JOIN sensei.metrics m ON m.id = rm.metric_id
              WHERE rm.repository_id = $1 AND rm.computed_on = $2 AND m.task_name = 'architecture'
                AND m.key IN ('dependency_cycles','layer_up_edges','zone_of_pain_modules','hidden_coupling_pairs','complexity_outlier_modules')
              ORDER BY m.key",
        )
        .bind(rid)
        .bind(day)
        .fetch_all(pg.pool())
        .await
        .unwrap();
        run("DELETE FROM sensei.workspaces WHERE id = $1")
            .bind(pid)
            .execute(pg.pool())
            .await
            .unwrap();

        let (alpha, beta) = (format!("{pkg}/alpha"), format!("{pkg}/beta"));
        let get = |k: &str| {
            rows.iter().find(|r| r.0 == k).unwrap_or_else(|| panic!("no {k} row: {rows:?}")).clone()
        };
        assert_eq!(rows.len(), 5, "{rows:?}");
        assert!(rows.iter().all(|r| r.3 == "repo"), "{rows:?}");
        let cycles = get("dependency_cycles");
        assert_eq!(
            (cycles.1, cycles.2["cycles"].clone(), cycles.2["largest"].clone()),
            (1.0, serde_json::json!([[alpha, beta]]), serde_json::json!(2))
        );
        assert_eq!(get("layer_up_edges").1, 1.0);
        assert_eq!(
            (get("hidden_coupling_pairs").1, get("complexity_outlier_modules").1),
            (0.0, 0.0)
        );
    }
}
