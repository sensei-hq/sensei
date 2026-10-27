//! The Code map's L0: modules as blocks in group bands, edges as one band per
//! module pair (M17, `docs/mockups/Sensei/docs/prd-drafts/M17-code-map.md`).
//!
//! Pure — the endpoint reads the module-level nodes and edges and hands them
//! here, so what the map draws is decided in one place and tested without a
//! database.
//!
//! Three rules keep L0 readable at any size:
//! - **Bands, not lines.** Every edge between two modules, of any kind, is one
//!   band, its weight the count behind it.
//! - **Hub lane.** A module taking more than [`HUB_SHARE`] of all inbound weight
//!   (a logger, an error type) is a hub: its edges are counted on the block,
//!   not drawn.
//! - **Budget.** At most [`BLOCK_BUDGET`] blocks. The smallest modules of a
//!   group fold into one "+N more" block for that group; their bands fold
//!   with them.

use std::collections::{BTreeMap, HashMap};

/// The most blocks L0 draws (G-1).
pub const BLOCK_BUDGET: usize = 60;

/// A module taking more than this share of all inbound weight is a hub...
pub const HUB_SHARE: f64 = 0.05;

/// ...when that weight comes from many modules: at least this many, and at
/// least [`HUB_REACH`] of them. A share alone makes every module of a small
/// workspace a hub, and the map would draw no band at all.
pub const HUB_MIN_CALLERS: usize = 5;
pub const HUB_REACH: f64 = 0.25;

/// One module, as the endpoint reads it.
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub id: String,
    pub label: String,
    /// The band it sits in: a package (Folders), a layer (Layers).
    pub group: String,
    pub files: i64,
    pub symbols: i64,
    /// Functions, and how many exceed cyclomatic 10 — `None` when none of the
    /// module's files was measured.
    pub complexity: Option<Complexity>,
}

/// A module's functions and how many are over cyclomatic 10 (local qlty).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
pub struct Complexity {
    pub functions: i64,
    pub complex: i64,
}

/// One edge between two modules, of one kind, with the count behind it.
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleEdge {
    pub source: String,
    pub target: String,
    pub kind: String,
    pub count: i64,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Block {
    pub id: String,
    pub label: String,
    pub group: String,
    pub files: i64,
    pub symbols: i64,
    /// A hub's inbound weight, counted here rather than drawn.
    pub hub: bool,
    pub weight_in: i64,
    pub weight_out: i64,
    /// For a "+N more" block: the modules folded into it.
    pub folded: Vec<String>,
    /// Summed over what it holds; `None` when nothing in it was measured.
    pub complexity: Option<Complexity>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Band {
    pub source: String,
    pub target: String,
    pub weight: i64,
    /// The weight per edge kind (`calls`, `imports`, …).
    pub kinds: BTreeMap<String, i64>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Budget {
    /// Blocks drawn, and modules in the workspace.
    pub drawn: usize,
    pub modules: usize,
    pub bands: usize,
    /// The total weight of every edge between modules, drawn or not.
    pub edges: i64,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Level0 {
    pub blocks: Vec<Block>,
    pub bands: Vec<Band>,
    pub budget: Budget,
}

/// The fold block's id for a group.
pub fn fold_id(group: &str) -> String {
    format!("fold:{group}")
}

/// L0 from the module-level nodes and edges.
pub fn level0(modules: &[Module], edges: &[ModuleEdge], budget: usize) -> Level0 {
    let between: Vec<&ModuleEdge> = edges.iter().filter(|e| e.source != e.target).collect();
    let total: i64 = between.iter().map(|e| e.count).sum();

    // Hubs: over HUB_SHARE of all inbound weight.
    let mut inbound: HashMap<&str, i64> = HashMap::new();
    let mut callers: HashMap<&str, std::collections::HashSet<&str>> = HashMap::new();
    for e in &between {
        *inbound.entry(e.target.as_str()).or_default() += e.count;
        callers.entry(e.target.as_str()).or_default().insert(e.source.as_str());
    }
    let min_callers = HUB_MIN_CALLERS.max((HUB_REACH * modules.len() as f64).ceil() as usize);
    let is_hub = |id: &str| {
        total > 0
            && inbound.get(id).copied().unwrap_or(0) as f64 > HUB_SHARE * total as f64
            && callers.get(id).map_or(0, |c| c.len()) >= min_callers
    };

    // The largest modules are drawn; within the budget, hubs and one fold block
    // per group that has folded members count too.
    let mut ranked: Vec<&Module> = modules.iter().filter(|m| !is_hub(&m.id)).collect();
    ranked.sort_by(|a, b| b.symbols.cmp(&a.symbols).then_with(|| a.id.cmp(&b.id)));
    let hubs = modules.len() - ranked.len();
    let room = budget.saturating_sub(hubs);
    let mut keep = ranked.len().min(room);
    let folds_for = |k: usize| {
        let mut groups: Vec<&str> = ranked[k..].iter().map(|m| m.group.as_str()).collect();
        groups.sort_unstable();
        groups.dedup();
        groups.len()
    };
    while keep > 0 && keep + folds_for(keep) > room {
        keep -= 1;
    }

    // Where each module is drawn: itself, or its group's fold block.
    let drawn_as: HashMap<&str, String> = modules
        .iter()
        .map(|m| {
            let folded = !is_hub(&m.id) && !ranked[..keep].iter().any(|k| k.id == m.id);
            (m.id.as_str(), if folded { fold_id(&m.group) } else { m.id.clone() })
        })
        .collect();

    let mut blocks: BTreeMap<String, Block> = BTreeMap::new();
    for m in modules {
        let id = drawn_as[m.id.as_str()].clone();
        let block = blocks.entry(id.clone()).or_insert_with(|| Block {
            label: if id == m.id { m.label.clone() } else { String::new() },
            id,
            group: m.group.clone(),
            files: 0,
            symbols: 0,
            hub: is_hub(&m.id),
            weight_in: 0,
            weight_out: 0,
            folded: Vec::new(),
            complexity: None,
        });
        block.files += m.files;
        block.symbols += m.symbols;
        if let Some(c) = m.complexity {
            let sum = block.complexity.get_or_insert_with(Complexity::default);
            sum.functions += c.functions;
            sum.complex += c.complex;
        }
        if block.id != m.id {
            block.folded.push(m.id.clone());
        }
    }

    let mut bands: BTreeMap<(String, String), Band> = BTreeMap::new();
    for e in &between {
        // An edge naming a module the node list lacks is drawn as itself only
        // if it is a block; otherwise it has nowhere to go and is counted in
        // `budget.edges` alone.
        let (Some(s), Some(t)) = (drawn_as.get(e.source.as_str()), drawn_as.get(e.target.as_str()))
        else {
            continue;
        };
        if let Some(b) = blocks.get_mut(s) {
            b.weight_out += e.count;
        }
        if let Some(b) = blocks.get_mut(t) {
            b.weight_in += e.count;
        }
        if s == t || is_hub(&e.source) || is_hub(&e.target) {
            continue;
        }
        let band = bands.entry((s.clone(), t.clone())).or_insert_with(|| Band {
            source: s.clone(),
            target: t.clone(),
            weight: 0,
            kinds: BTreeMap::new(),
        });
        band.weight += e.count;
        *band.kinds.entry(e.kind.clone()).or_default() += e.count;
    }

    let mut blocks: Vec<Block> = blocks.into_values().collect();
    for b in &mut blocks {
        if !b.folded.is_empty() {
            b.folded.sort();
            b.label = format!("+{} more", b.folded.len());
        }
    }
    blocks.sort_by(|a, b| {
        a.group
            .cmp(&b.group)
            .then(b.folded.is_empty().cmp(&a.folded.is_empty()).reverse())
            .then(b.symbols.cmp(&a.symbols))
            .then(a.id.cmp(&b.id))
    });
    let bands: Vec<Band> = bands.into_values().collect();
    Level0 {
        budget: Budget {
            drawn: blocks.len(),
            modules: modules.len(),
            bands: bands.len(),
            edges: total,
        },
        blocks,
        bands,
    }
}

/// What a file is, for the map's columns and its Show filter (M17): code,
/// tests, docs, config, or generated. From the path, the conventions every
/// ecosystem we index uses; generated wins, since a generated test is not
/// the developer's test.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Code,
    Test,
    Docs,
    Config,
    Generated,
}

pub fn file_role(path: &str) -> Role {
    let lower = path.to_ascii_lowercase();
    let segments: Vec<&str> = lower.split('/').collect();
    let (dirs, name) = segments.split_at(segments.len().saturating_sub(1));
    let name = name.first().copied().unwrap_or("");
    let original = path.rsplit('/').next().unwrap_or(path);
    let in_dir = |names: &[&str]| dirs.iter().any(|d| names.contains(d));
    let ext = name.rsplit_once('.').map_or("", |(_, e)| e);

    if in_dir(&["generated", "__generated__", "gen", "dist", "target"])
        || name.contains(".generated.")
        || name.ends_with(".pb.go")
        || name.ends_with("_pb2.py")
        || name.ends_with(".g.dart")
        || name.ends_with(".d.ts")
    {
        return Role::Generated;
    }
    if in_dir(&["tests", "test", "__tests__", "e2e", "spec", "specs"])
        || name.contains(".spec.")
        || name.contains(".test.")
        || name.ends_with("_test.go")
        || name.ends_with("_test.rs")
        || name.ends_with("_tests.rs")
        || (name.starts_with("test_") && ext == "py")
        || original.ends_with("Test.java")
        || original.ends_with("Tests.cs")
        || original.ends_with("Test.kt")
    {
        return Role::Test;
    }
    if matches!(ext, "md" | "mdx" | "rst" | "adoc" | "txt") || in_dir(&["docs", "doc"]) {
        return Role::Docs;
    }
    if matches!(ext, "json" | "toml" | "yaml" | "yml" | "ini" | "cfg" | "conf" | "lock")
        || name.contains(".config.")
        || matches!(name, "makefile" | "dockerfile" | ".env")
    {
        return Role::Config;
    }
    Role::Code
}

/// One file of a module, as the endpoint reads it.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ModuleFile {
    pub id: String,
    pub path: String,
    pub symbols: i64,
}

/// The most files one L1 column draws (M17); the rest fold into "+N more".
pub const COLUMN_BUDGET: usize = 12;

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Column {
    pub role: Role,
    /// The largest files, at most [`COLUMN_BUDGET`].
    pub files: Vec<ModuleFile>,
    /// How many smaller files fold into "+N more", and their symbols.
    pub more: usize,
    pub more_symbols: i64,
}

/// L1: one module's files in columns by role, in the order code, tests, docs,
/// config, generated; an empty role has no column.
pub fn level1(files: &[ModuleFile]) -> Vec<Column> {
    let mut by_role: BTreeMap<Role, Vec<&ModuleFile>> = BTreeMap::new();
    for f in files {
        by_role.entry(file_role(&f.path)).or_default().push(f);
    }
    by_role
        .into_iter()
        .map(|(role, mut fs)| {
            fs.sort_by(|a, b| b.symbols.cmp(&a.symbols).then_with(|| a.path.cmp(&b.path)));
            let rest = fs.split_off(fs.len().min(COLUMN_BUDGET));
            Column {
                role,
                files: fs.into_iter().cloned().collect(),
                more: rest.len(),
                more_symbols: rest.iter().map(|f| f.symbols).sum(),
            }
        })
        .collect()
}

/// Two modules changed in the same commits, with how often each changed.
#[derive(Debug, Clone, PartialEq)]
pub struct CoChange {
    pub a: String,
    pub b: String,
    pub together: i64,
    pub a_changes: i64,
    pub b_changes: i64,
}

/// A pair that changes together and has no static dependency either way (M17
/// Hidden coupling), with `strength` = together ÷ the rarer one's changes: 1.0
/// means the rarer never changes without the other.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Coupling {
    pub a: String,
    pub b: String,
    pub together: i64,
    pub strength: f64,
}

/// The hidden couplings among `pairs`: those with no edge between them in
/// either direction in `edges`, strongest first, at most `limit`.
pub fn hidden_coupling(pairs: &[CoChange], edges: &[ModuleEdge], limit: usize) -> Vec<Coupling> {
    let linked: std::collections::HashSet<(&str, &str)> = edges
        .iter()
        .flat_map(|e| {
            [(e.source.as_str(), e.target.as_str()), (e.target.as_str(), e.source.as_str())]
        })
        .collect();
    let mut out: Vec<Coupling> = pairs
        .iter()
        .filter(|p| !linked.contains(&(p.a.as_str(), p.b.as_str())))
        .map(|p| Coupling {
            a: p.a.clone(),
            b: p.b.clone(),
            together: p.together,
            strength: p.together as f64 / p.a_changes.min(p.b_changes).max(1) as f64,
        })
        .collect();
    out.sort_by(|x, y| {
        y.strength.total_cmp(&x.strength).then(y.together.cmp(&x.together)).then(x.a.cmp(&y.a))
    });
    out.truncate(limit);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(id: &str, group: &str, symbols: i64) -> Module {
        Module {
            id: id.into(),
            label: id.into(),
            group: group.into(),
            files: 1,
            symbols,
            complexity: None,
        }
    }
    fn e(source: &str, target: &str, kind: &str, count: i64) -> ModuleEdge {
        ModuleEdge { source: source.into(), target: target.into(), kind: kind.into(), count }
    }
    fn band<'a>(l0: &'a Level0, s: &str, t: &str) -> Option<&'a Band> {
        l0.bands.iter().find(|b| b.source == s && b.target == t)
    }

    /// Every edge between two modules, whatever its kind, is one band.
    #[test]
    fn edges_between_two_modules_are_one_band() {
        // A ring: every module takes the same share, so none is a hub.
        let mods: Vec<_> = (0..40).map(|i| m(&format!("m{i}"), "g", 10)).collect();
        let mut edges: Vec<_> = (0..40)
            .map(|i| e(&format!("m{}", (i + 1) % 40), &format!("m{i}"), "calls", 3))
            .collect();
        edges.push(e("m1", "m0", "imports", 2));
        edges.push(e("m0", "m0", "calls", 9)); // within a module: not a band
        let l0 = level0(&mods, &edges, BLOCK_BUDGET);
        assert!(l0.blocks.iter().all(|b| !b.hub));
        let b = band(&l0, "m1", "m0").expect("one band m1 → m0");
        assert_eq!(b.weight, 5);
        assert_eq!(b.kinds, BTreeMap::from([("calls".into(), 3), ("imports".into(), 2)]));
        assert_eq!(l0.bands.iter().filter(|b| b.source == "m1" && b.target == "m0").count(), 1);
        assert!(band(&l0, "m0", "m0").is_none(), "a module's own edges are not a band");
        assert_eq!(l0.budget.edges, 40 * 3 + 2, "every edge between modules is counted");
        assert_eq!(l0.budget.bands, 40);
    }

    /// A module taking over 5% of all inbound weight is a hub: its bands are
    /// not drawn, its weight is on the block.
    #[test]
    fn a_hub_draws_no_bands_and_carries_its_weight() {
        let mut mods: Vec<_> = (0..30).map(|i| m(&format!("m{i}"), "g", 10)).collect();
        mods.push(m("log", "infra", 5));
        let mut edges: Vec<_> = (0..30).map(|i| e(&format!("m{i}"), "log", "calls", 2)).collect();
        edges.extend((1..30).map(|i| e(&format!("m{i}"), &format!("m{}", i - 1), "calls", 1)));
        let l0 = level0(&mods, &edges, BLOCK_BUDGET);
        let log = l0.blocks.iter().find(|b| b.id == "log").unwrap();
        assert!(log.hub);
        assert_eq!(log.weight_in, 60);
        assert!(
            l0.bands.iter().all(|b| b.target != "log" && b.source != "log"),
            "no band to a hub"
        );
        assert!(l0.blocks.iter().filter(|b| b.id != "log").all(|b| !b.hub));
        assert_eq!(l0.budget.edges, 60 + 29);
    }

    /// Over budget, the smallest modules of each group fold into one "+N more"
    /// block per group; their bands fold with them; nothing is dropped.
    #[test]
    fn over_budget_the_smallest_fold_per_group_and_their_bands_follow() {
        let mut mods = Vec::new();
        for g in ["a", "b"] {
            for i in 0..40 {
                mods.push(m(&format!("{g}{i}"), g, 1000 - i));
            }
        }
        let edges = vec![
            e("a0", "b0", "calls", 3),   // kept → kept
            e("a39", "b39", "calls", 5), // folded → folded
            e("a39", "a38", "calls", 7), // within one fold: not a band
            e("a1", "b39", "calls", 2),  // kept → folded
        ];
        let l0 = level0(&mods, &edges, BLOCK_BUDGET);
        assert!(l0.blocks.len() <= BLOCK_BUDGET, "{} blocks", l0.blocks.len());
        assert_eq!(l0.budget.drawn, l0.blocks.len());
        assert_eq!(l0.budget.modules, 80);
        let (fa, fb) = (fold_id("a"), fold_id("b"));
        let folded: usize = l0.blocks.iter().map(|b| b.folded.len()).sum();
        let kept = l0.blocks.iter().filter(|b| b.folded.is_empty()).count();
        assert_eq!(kept + folded, 80, "every module is drawn or folded");
        assert!(l0.blocks.iter().any(|b| b.id == "a0") && l0.blocks.iter().any(|b| b.id == fa));
        assert!(!l0.blocks.iter().any(|b| b.id == "a39"), "the smallest are folded");
        assert_eq!(band(&l0, "a0", "b0").unwrap().weight, 3);
        assert_eq!(band(&l0, &fa, &fb).unwrap().weight, 5);
        assert_eq!(band(&l0, "a1", &fb).unwrap().weight, 2);
        assert!(band(&l0, &fa, &fa).is_none());
        let fold_a = l0.blocks.iter().find(|b| b.id == fa).unwrap();
        assert_eq!(
            fold_a.symbols,
            fold_a.folded.len() as i64 * 1000
                - (fold_a.folded.iter().map(|id| id[1..].parse::<i64>().unwrap()).sum::<i64>())
        );
    }

    /// In a small workspace no module is a hub: a large share from one or two
    /// callers is an ordinary dependency, drawn as a band.
    #[test]
    fn a_small_workspace_has_no_hub() {
        let mods = vec![m("a", "g", 1), m("b", "g", 1), m("c", "g", 1)];
        let edges = vec![e("a", "c", "calls", 5), e("b", "c", "calls", 5)];
        let l0 = level0(&mods, &edges, BLOCK_BUDGET);
        assert!(l0.blocks.iter().all(|b| !b.hub));
        assert_eq!(l0.bands.len(), 2);
    }

    /// Roles from the path, across the ecosystems we index.
    #[test]
    fn a_files_role_comes_from_its_path() {
        for (path, role) in [
            ("src/lib.rs", Role::Code),
            ("app/src/lib/api.ts", Role::Code),
            ("crates/x/tests/routes.rs", Role::Test),
            ("src/db/pg_store/tests/graph_tests.rs", Role::Test),
            ("src/lib/api.spec.ts", Role::Test),
            ("src/routes/page.spec.svelte.ts", Role::Test),
            ("pkg/foo_test.go", Role::Test),
            ("tests/test_models.py", Role::Test),
            ("src/test/java/a/FooTest.java", Role::Test),
            ("e2e/tests/login.spec.ts", Role::Test),
            ("docs/guide/intro.md", Role::Docs),
            ("README.md", Role::Docs),
            ("package.json", Role::Config),
            ("svelte.config.js", Role::Config),
            ("Cargo.toml", Role::Config),
            ("src/gen/schema.pb.go", Role::Generated),
            ("src/__generated__/graphql.ts", Role::Generated),
            ("src/api_pb2.py", Role::Generated),
            ("src/__generated__/thing.test.ts", Role::Generated),
        ] {
            assert_eq!(file_role(path), role, "{path}");
        }
    }

    /// Files sit in role columns, largest first; past the column budget the
    /// rest fold into "+N more" — counted, not dropped.
    #[test]
    fn a_modules_files_sit_in_role_columns_with_the_rest_folded() {
        let f =
            |path: &str, symbols: i64| ModuleFile { id: path.into(), path: path.into(), symbols };
        let mut files: Vec<_> = (0..15).map(|i| f(&format!("src/f{i}.rs"), 100 - i)).collect();
        files.push(f("tests/a.rs", 5));
        files.push(f("README.md", 1));
        let cols = level1(&files);
        assert_eq!(
            cols.iter().map(|c| c.role).collect::<Vec<_>>(),
            vec![Role::Code, Role::Test, Role::Docs]
        );
        let code = &cols[0];
        assert_eq!(code.files.len(), COLUMN_BUDGET);
        assert_eq!(code.files[0].path, "src/f0.rs", "largest first");
        assert_eq!((code.more, code.more_symbols), (3, 88 + 87 + 86));
        assert_eq!(cols.iter().map(|c| c.files.len() + c.more).sum::<usize>(), files.len());
    }

    /// Co-change with a static edge either way is ordinary; without one it is
    /// hidden coupling, strongest first.
    #[test]
    fn hidden_coupling_is_co_change_with_no_static_edge() {
        let c = |a: &str, b: &str, together, a_changes, b_changes| CoChange {
            a: a.into(),
            b: b.into(),
            together,
            a_changes,
            b_changes,
        };
        let pairs = vec![
            c("p/a", "p/b", 9, 10, 40), // a → b statically: ordinary
            c("p/c", "p/d", 6, 12, 6),  // no edge: the rarer (d) never changes alone
            c("p/e", "p/f", 5, 20, 25), // no edge: 5 of e's 20
            c("p/g", "p/h", 8, 8, 8),   // h → g statically: ordinary, the other way
        ];
        let edges = vec![e("p/a", "p/b", "calls", 1), e("p/h", "p/g", "imports", 1)];
        let hidden = hidden_coupling(&pairs, &edges, 10);
        assert_eq!(
            hidden.iter().map(|h| (h.a.as_str(), h.b.as_str(), h.strength)).collect::<Vec<_>>(),
            vec![("p/c", "p/d", 1.0), ("p/e", "p/f", 0.25)]
        );
        assert_eq!(hidden_coupling(&pairs, &edges, 1).len(), 1);
    }

    /// Under budget, nothing folds.
    #[test]
    fn under_budget_every_module_is_a_block() {
        let mods: Vec<_> = (0..10).map(|i| m(&format!("m{i}"), "g", 1)).collect();
        let l0 = level0(&mods, &[], BLOCK_BUDGET);
        assert_eq!(l0.blocks.len(), 10);
        assert!(l0.blocks.iter().all(|b| b.folded.is_empty()));
    }
}
