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
    /// The package (repository member) the module belongs to.
    pub package: String,
    pub files: i64,
    pub symbols: i64,
    /// Its files by role, so tests and docs never read as code size.
    pub roles: RoleCounts,
    /// Functions, and how many exceed cyclomatic 10 — `None` when none of the
    /// module's files was measured.
    pub complexity: Option<Complexity>,
}

/// Files by role (see [`file_role`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
pub struct RoleCounts {
    pub code: i64,
    pub test: i64,
    pub docs: i64,
    pub config: i64,
    pub generated: i64,
}

impl RoleCounts {
    fn count(&mut self, role: Role) {
        match role {
            Role::Code => self.code += 1,
            Role::Test => self.test += 1,
            Role::Docs => self.docs += 1,
            Role::Config => self.config += 1,
            Role::Generated => self.generated += 1,
        }
    }

    fn add(&mut self, other: RoleCounts) {
        self.code += other.code;
        self.test += other.test;
        self.docs += other.docs;
        self.config += other.config;
        self.generated += other.generated;
    }
}

/// Each module's files by role, from `(module, path)` pairs.
pub fn role_counts(paths: &[(String, String)]) -> HashMap<String, RoleCounts> {
    let mut out: HashMap<String, RoleCounts> = HashMap::new();
    for (module, path) in paths {
        out.entry(module.clone()).or_default().count(file_role(path));
    }
    out
}

/// The workspace's referenced libraries as L0 draws them — one block, never
/// members (M17): how many, and the `top` most used by repositories. `rows` are
/// [`crate::db::pg_store::PgStore::list_libraries_with_usage`]'s.
pub fn libraries_summary(rows: &[serde_json::Value], top: usize) -> serde_json::Value {
    let mut ranked: Vec<(&str, i64)> = rows
        .iter()
        .filter_map(|r| Some((r.get("name")?.as_str()?, r.get("repo_count")?.as_i64()?)))
        .collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    let names: Vec<&str> = ranked.iter().take(top).map(|(n, _)| *n).collect();
    serde_json::json!({ "count": rows.len(), "top": names })
}

/// A module's functions and how many are over cyclomatic 10 (local qlty).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
pub struct Complexity {
    pub functions: i64,
    pub complex: i64,
}

/// How a module's complexity reads — the one rule the Complexity lens, its
/// takeaways and the architecture metrics (#156) all use.
#[allow(dead_code)] // constructed by `tier` in #156 phase B
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ComplexityTier {
    /// More than a fifth of its functions over cyclomatic 10: an outlier.
    Danger,
    /// More than a tenth.
    Warning,
    Neutral,
}

impl Complexity {
    /// Its tier; none when no function was measured. Pending (#156 phase B).
    #[allow(dead_code)]
    pub fn tier(&self) -> Option<ComplexityTier> {
        None
    }
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
    /// The package its modules share; `None` for a fold spanning several.
    pub package: Option<String>,
    pub files: i64,
    pub symbols: i64,
    pub roles: RoleCounts,
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

/// The map narrowed to a focus (user, 2026-11-03): the focused modules, the
/// modules one dependency away in either direction, and only the edges that
/// touch a focused module.
#[derive(Debug, Clone, PartialEq)]
pub struct Focused {
    pub modules: Vec<Module>,
    pub edges: Vec<ModuleEdge>,
    /// The kept modules that are not focused, sorted.
    pub neighbours: Vec<String>,
}

/// Narrow `modules` and `edges` to `members` and their direct neighbours. Run
/// before [`level0`], so nothing in the neighbourhood is folded away. A focus
/// that names no known module keeps nothing.
pub fn focus_scope(
    modules: &[Module],
    edges: &[ModuleEdge],
    members: &std::collections::HashSet<String>,
) -> Focused {
    let known: std::collections::HashSet<&str> = modules.iter().map(|m| m.id.as_str()).collect();
    let touches = |e: &&ModuleEdge| {
        (members.contains(&e.source) || members.contains(&e.target))
            && known.contains(e.source.as_str())
            && known.contains(e.target.as_str())
    };
    let kept_edges: Vec<ModuleEdge> = edges.iter().filter(touches).cloned().collect();
    let mut neighbours: Vec<String> = kept_edges
        .iter()
        .flat_map(|e| [&e.source, &e.target])
        .filter(|id| !members.contains(*id))
        .cloned()
        .collect();
    neighbours.sort_unstable();
    neighbours.dedup();
    let kept_modules: Vec<Module> = modules
        .iter()
        .filter(|m| members.contains(&m.id) || neighbours.binary_search(&m.id).is_ok())
        .cloned()
        .collect();
    Focused { modules: kept_modules, edges: kept_edges, neighbours }
}

/// The modules a Group-by segment holds: every module whose group is `segment`.
pub fn segment_members(modules: &[Module], segment: &str) -> std::collections::HashSet<String> {
    modules.iter().filter(|m| m.group == segment).map(|m| m.id.clone()).collect()
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
            package: Some(m.package.clone()),
            files: 0,
            symbols: 0,
            roles: RoleCounts::default(),
            hub: is_hub(&m.id),
            weight_in: 0,
            weight_out: 0,
            folded: Vec::new(),
            complexity: None,
        });
        block.files += m.files;
        block.symbols += m.symbols;
        block.roles.add(m.roles);
        if block.package.as_deref() != Some(m.package.as_str()) {
            block.package = None;
        }
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

/// One measured file, with the module it belongs to (local qlty).
#[derive(Debug, Clone, PartialEq)]
pub struct MeasuredFile {
    pub module: String,
    /// Repository-relative path, as qlty reports it.
    pub path: String,
    pub functions: i64,
    pub complex: i64,
    pub max_cyclo: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeKind {
    Module,
    Folder,
    File,
}

/// One row of the Complexity tree: a module, a folder in it, or a file.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComplexityNode {
    pub id: String,
    pub label: String,
    pub kind: NodeKind,
    pub functions: i64,
    /// Functions over cyclomatic 10.
    pub complex: i64,
    /// The highest cyclomatic complexity of any function under this row.
    pub max_cyclo: i64,
    /// Files measured under this row.
    pub files: i64,
    pub children: Vec<ComplexityNode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComplexityTotal {
    pub functions: i64,
    pub complex: i64,
    pub max_cyclo: i64,
    pub files: i64,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ComplexityTree {
    pub total: ComplexityTotal,
    pub modules: Vec<ComplexityNode>,
}

/// The Complexity lens (user, 2026-11-02): every measured file under its module
/// and folders, each level carrying what is under it, most complex first. A
/// module's own directory is its root; a folder holding only one folder reads as
/// one row (`src/lib`).
pub fn complexity_tree(files: &[MeasuredFile]) -> ComplexityTree {
    let mut by_module: std::collections::BTreeMap<&str, Vec<&MeasuredFile>> = Default::default();
    for f in files {
        by_module.entry(f.module.as_str()).or_default().push(f);
    }
    let mut modules: Vec<ComplexityNode> = by_module
        .into_iter()
        .map(|(module, files)| {
            let root = common_dir(files.iter().map(|f| f.path.as_str()));
            let mut dir = Dir::default();
            for f in &files {
                let rel = f.path.strip_prefix(&root).unwrap_or(&f.path).trim_start_matches('/');
                dir.insert(rel, f);
            }
            let mut node =
                dir.into_node(module.to_string(), module.to_string(), NodeKind::Module, &root);
            node.label = module.to_string();
            node
        })
        .collect();
    sort_nodes(&mut modules);
    let total = modules.iter().fold(ComplexityTotal::default(), |t, m| ComplexityTotal {
        functions: t.functions + m.functions,
        complex: t.complex + m.complex,
        max_cyclo: t.max_cyclo.max(m.max_cyclo),
        files: t.files + m.files,
    });
    ComplexityTree { total, modules }
}

/// The longest directory every path shares (`alpha` for `alpha/a.rs` and
/// `alpha/tests/b.rs`); empty when they share none.
fn common_dir<'a>(paths: impl Iterator<Item = &'a str>) -> String {
    let mut common: Option<Vec<&str>> = None;
    for p in paths {
        let dirs: Vec<&str> = p.split('/').collect();
        let dirs = &dirs[..dirs.len().saturating_sub(1)];
        common = Some(match common {
            None => dirs.to_vec(),
            Some(c) => c.iter().zip(dirs).take_while(|(a, b)| a == b).map(|(a, _)| *a).collect(),
        });
    }
    common.unwrap_or_default().join("/")
}

#[derive(Default)]
struct Dir<'a> {
    dirs: std::collections::BTreeMap<&'a str, Dir<'a>>,
    files: Vec<&'a MeasuredFile>,
}

impl<'a> Dir<'a> {
    fn insert(&mut self, rel: &'a str, f: &'a MeasuredFile) {
        match rel.split_once('/') {
            Some((head, rest)) => self.dirs.entry(head).or_default().insert(rest, f),
            None => self.files.push(f),
        }
    }

    /// This directory as a node: its folders (single-folder chains joined) and
    /// files, summed, most complex first.
    fn into_node(self, id: String, label: String, kind: NodeKind, path: &str) -> ComplexityNode {
        let mut children: Vec<ComplexityNode> = self
            .dirs
            .into_iter()
            .map(|(name, mut sub)| {
                let mut label = name.to_string();
                let mut path =
                    if path.is_empty() { name.to_string() } else { format!("{path}/{name}") };
                while sub.files.is_empty() && sub.dirs.len() == 1 {
                    let (next, inner) = sub.dirs.pop_first().expect("one folder");
                    label = format!("{label}/{next}");
                    path = format!("{path}/{next}");
                    sub = inner;
                }
                sub.into_node(path.clone(), label, NodeKind::Folder, &path)
            })
            .collect();
        children.extend(self.files.into_iter().map(|f| ComplexityNode {
            id: f.path.clone(),
            label: f.path.rsplit('/').next().unwrap_or(&f.path).to_string(),
            kind: NodeKind::File,
            functions: f.functions,
            complex: f.complex,
            max_cyclo: f.max_cyclo,
            files: 1,
            children: vec![],
        }));
        sort_nodes(&mut children);
        let sum = |g: fn(&ComplexityNode) -> i64| children.iter().map(g).sum::<i64>();
        ComplexityNode {
            id,
            label,
            kind,
            functions: sum(|c| c.functions),
            complex: sum(|c| c.complex),
            max_cyclo: children.iter().map(|c| c.max_cyclo).max().unwrap_or(0),
            files: sum(|c| c.files),
            children,
        }
    }
}

/// Most complex functions first, then the larger, then by name — stable.
fn sort_nodes(nodes: &mut [ComplexityNode]) {
    nodes.sort_by(|a, b| {
        b.complex.cmp(&a.complex).then(b.functions.cmp(&a.functions)).then(a.label.cmp(&b.label))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(id: &str, group: &str, symbols: i64) -> Module {
        Module {
            id: id.into(),
            label: id.into(),
            group: group.into(),
            package: "pkg".into(),
            files: 1,
            symbols,
            roles: RoleCounts { code: 1, ..Default::default() },
            complexity: None,
        }
    }

    fn measured(
        module: &str,
        path: &str,
        functions: i64,
        complex: i64,
        max_cyclo: i64,
    ) -> MeasuredFile {
        MeasuredFile { module: module.into(), path: path.into(), functions, complex, max_cyclo }
    }

    /// The complexity rule lives here, once (#156): the app reads the tier the
    /// daemon sends instead of re-deriving the thresholds, and the metrics count
    /// the same outliers the lens shows.
    /// Review G6: the Complexity lens's rows carry the same tier, so the lens's red
    /// rows are the metric's outliers — one rule, read in two places.
    #[test]
    #[ignore = "pending: #156"]
    fn the_complexity_tree_carries_each_rows_tier() {
        let t = complexity_tree(&[
            measured("pkg/alpha", "alpha/a.rs", 4, 1, 14),
            measured("pkg/beta", "beta/b.rs", 10, 1, 12),
        ]);
        let tier = |id: &str| {
            let m = t.modules.iter().find(|m| m.id == id).expect("module row");
            serde_json::to_value(m).unwrap()["tier"].clone()
        };
        assert_eq!(
            (tier("pkg/alpha"), tier("pkg/beta")),
            (serde_json::json!("danger"), serde_json::json!("neutral"))
        );
        let file = serde_json::to_value(&t.modules[0].children[0]).unwrap();
        assert!(file["tier"].is_string(), "every row carries one: {file}");
    }

    #[test]
    #[ignore = "pending: #156"]
    fn a_module_complexity_carries_its_tier() {
        let c = |functions, complex| Complexity { functions, complex };
        assert_eq!(c(10, 3).tier(), Some(ComplexityTier::Danger));
        assert_eq!(c(10, 2).tier(), Some(ComplexityTier::Warning), "20% is not over a fifth");
        assert_eq!(c(10, 1).tier(), Some(ComplexityTier::Neutral), "10% is not over a tenth");
        assert_eq!(c(0, 0).tier(), None, "nothing measured has no tier");
        assert_eq!(
            serde_json::to_value(c(4, 1)).unwrap(),
            serde_json::json!({"functions": 4, "complex": 1, "tier": "danger"})
        );
    }

    fn ids(modules: &[Module]) -> Vec<&str> {
        let mut v: Vec<&str> = modules.iter().map(|m| m.id.as_str()).collect();
        v.sort_unstable();
        v
    }

    /// A focus (user, 2026-11-03) narrows the map to the focused modules and the
    /// modules one dependency away — in either direction — before anything is
    /// folded, so the neighbourhood is drawn whole. Two hops away is out.
    #[test]
    fn a_focus_keeps_the_module_and_its_direct_neighbours() {
        let modules = vec![m("a", "L0", 1), m("b", "L1", 1), m("c", "L1", 1), m("d", "L2", 1)];
        let edges = vec![e("a", "b", "calls", 3), e("c", "a", "calls", 2), e("b", "d", "calls", 5)];
        let f = focus_scope(&modules, &edges, &["a".to_string()].into());
        assert_eq!(ids(&f.modules), ["a", "b", "c"]);
        assert_eq!(f.neighbours, ["b", "c"]);
    }

    /// Only the dependencies that touch a focused module are kept: two
    /// neighbours' own traffic is the rest of the map, not the focus.
    #[test]
    fn a_focus_keeps_only_the_edges_that_touch_it() {
        let modules = vec![m("a", "L0", 1), m("b", "L1", 1), m("c", "L1", 1)];
        let edges = vec![
            e("a", "b", "calls", 3),
            e("a", "c", "calls", 1),
            e("b", "c", "calls", 9),
            e("a", "a", "calls", 4),
        ];
        let f = focus_scope(&modules, &edges, &["a".to_string()].into());
        let kept: Vec<(&str, &str)> =
            f.edges.iter().map(|e| (e.source.as_str(), e.target.as_str())).collect();
        assert_eq!(kept, [("a", "b"), ("a", "c"), ("a", "a")]);
    }

    /// A segment is a Group-by group: every module in it is focused, and its
    /// neighbours are the modules outside it that it depends on or is used by.
    #[test]
    fn a_segment_focuses_every_module_in_it() {
        let modules = vec![m("a", "L1", 1), m("b", "L1", 1), m("c", "L2", 1), m("d", "L3", 1)];
        let edges = vec![e("a", "b", "calls", 1), e("b", "c", "calls", 2)];
        let members = segment_members(&modules, "L1");
        assert_eq!(members, ["a".to_string(), "b".to_string()].into());
        let f = focus_scope(&modules, &edges, &members);
        assert_eq!(ids(&f.modules), ["a", "b", "c"]);
        assert_eq!(f.neighbours, ["c"]);
    }

    /// A focus that names nothing in the workspace narrows to nothing — the
    /// caller answers it as not found, never as an empty map.
    #[test]
    fn a_focus_on_no_known_module_keeps_nothing() {
        let modules = vec![m("a", "L0", 1)];
        let f = focus_scope(&modules, &[], &["zzz".to_string()].into());
        assert!(f.modules.is_empty() && f.edges.is_empty() && f.neighbours.is_empty());
    }

    /// The Complexity lens is a tree (user, 2026-11-02): module → folder → file,
    /// each level carrying its functions, how many are over cyclomatic 10, the
    /// highest cyclomatic under it and how many files were measured.
    #[test]
    fn complexity_rolls_up_module_folder_file() {
        let t = complexity_tree(&[
            measured("pkg/alpha", "alpha/a.rs", 4, 1, 14),
            measured("pkg/alpha", "alpha/tests/a_test.rs", 2, 0, 3),
            measured("pkg/alpha", "alpha/tests/b_test.rs", 3, 2, 22),
            measured("pkg/beta", "beta/b.rs", 1, 0, 2),
        ]);
        assert_eq!(
            (t.total.functions, t.total.complex, t.total.max_cyclo, t.total.files),
            (10, 3, 22, 4)
        );
        let alpha = &t.modules[0];
        assert_eq!((alpha.id.as_str(), alpha.kind), ("pkg/alpha", NodeKind::Module));
        assert_eq!((alpha.functions, alpha.complex, alpha.max_cyclo, alpha.files), (9, 3, 22, 3));
        // The module's own directory is its root; under it, the tests folder and a.rs.
        let labels: Vec<&str> = alpha.children.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(labels, vec!["tests", "a.rs"], "most complex first");
        let tests = &alpha.children[0];
        assert_eq!((tests.kind, tests.complex, tests.files), (NodeKind::Folder, 2, 2));
        assert_eq!(tests.children[0].id, "alpha/tests/b_test.rs");
        assert_eq!(tests.children[0].kind, NodeKind::File);
    }

    #[test]
    fn complexity_orders_modules_by_complex_functions_then_size() {
        let t = complexity_tree(&[
            measured("pkg/quiet", "quiet/q.rs", 50, 0, 4),
            measured("pkg/busy", "busy/b.rs", 5, 3, 30),
            measured("pkg/also", "also/a.rs", 9, 3, 12),
        ]);
        let ids: Vec<&str> = t.modules.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, vec!["pkg/also", "pkg/busy", "pkg/quiet"]);
    }

    /// A folder with one folder in it and no files of its own reads as one row
    /// (`src/lib`), so a deep tree does not open on a staircase of single folders.
    #[test]
    fn complexity_collapses_single_folder_chains() {
        let t = complexity_tree(&[
            measured("pkg/m", "m/src/lib/deep/x.rs", 2, 1, 11),
            measured("pkg/m", "m/src/lib/deep/y.rs", 2, 0, 5),
            measured("pkg/m", "m/main.rs", 1, 0, 1),
        ]);
        let m = &t.modules[0];
        let folder = m.children.iter().find(|c| c.kind == NodeKind::Folder).unwrap();
        assert_eq!(folder.label, "src/lib/deep");
        assert_eq!(folder.children.len(), 2);
    }

    #[test]
    fn complexity_of_nothing_measured_is_an_empty_tree() {
        let t = complexity_tree(&[]);
        assert!(t.modules.is_empty());
        assert_eq!((t.total.functions, t.total.files), (0, 0));
    }

    /// A module's files by role, so tests and docs never read as code size (M17).
    #[test]
    fn files_are_counted_by_role_per_module() {
        let paths = [
            ("pkg/a", "src/a/lib.rs"),
            ("pkg/a", "src/a/tests/lib_test.rs"),
            ("pkg/a", "src/a/README.md"),
            ("pkg/b", "src/b/mod.rs"),
        ]
        .map(|(m, p)| (m.to_string(), p.to_string()));
        let counts = role_counts(&paths);
        assert_eq!(counts["pkg/a"], RoleCounts { code: 1, test: 1, docs: 1, ..Default::default() });
        assert_eq!(counts["pkg/b"], RoleCounts { code: 1, ..Default::default() });
    }

    /// A block carries its module's package and roles; a fold sums its members'
    /// roles, and keeps a package only when they share one.
    #[test]
    fn blocks_carry_package_and_roles_and_folds_sum_them() {
        let mut mods: Vec<Module> = (0..3).map(|i| m(&format!("m{i}"), "g", 100 - i)).collect();
        mods[1].package = "other".into();
        mods[2].roles = RoleCounts { code: 2, test: 3, ..Default::default() };
        let l0 = level0(&mods, &[], 2);
        let kept = l0.blocks.iter().find(|b| b.id == "m0").unwrap();
        assert_eq!((kept.package.as_deref(), kept.roles.code), (Some("pkg"), 1));
        let fold = l0.blocks.iter().find(|b| b.id == fold_id("g")).unwrap();
        assert_eq!(fold.roles, RoleCounts { code: 3, test: 3, ..Default::default() });
        assert_eq!(fold.package, None, "members in two packages name none");
    }

    /// The referenced libraries are one block's worth: how many, and the most used.
    #[test]
    fn libraries_summarise_as_a_count_and_the_most_used() {
        let rows = vec![
            serde_json::json!({"name": "tokio", "repo_count": 3}),
            serde_json::json!({"name": "serde", "repo_count": 5}),
            serde_json::json!({"name": "axum", "repo_count": 1}),
        ];
        assert_eq!(
            libraries_summary(&rows, 2),
            serde_json::json!({"count": 3, "top": ["serde", "tokio"]})
        );
        assert_eq!(libraries_summary(&[], 2), serde_json::json!({"count": 0, "top": []}));
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
