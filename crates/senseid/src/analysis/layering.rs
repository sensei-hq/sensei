//! Cycles and layers over a dependency graph (#222).
//!
//! Two diagrams share one computation. **Cycles** collapses each group of
//! mutually dependent units into a single node and names the weakest link to
//! cut; **Layers** ranks what remains and says how each dependency sits against
//! that ranking. Layers cannot be computed before Cycles — longest-path depth is
//! only defined on a DAG — so the strongly-connected components come first and
//! the rank is taken over the condensation.
//!
//! ## Why this is Rust rather than SQL
//!
//! The first design put the whole thing in a recursive CTE. Adversarial review
//! found the shape that kills it: `(s, t, len)` enumeration with `UNION`
//! terminates only on an acyclic input, so ONE surviving self-loop makes the
//! query run forever — it does not error, it hangs. Acyclicity would have been
//! an unasserted precondition of one query held up by the correctness of
//! another. Tarjan plus a Kahn-ordered DP is linear, terminates by
//! construction, and — the part that actually matters — is testable without a
//! database, which is why `analysis` exists at all.
//!
//! ## The grain is the caller's
//!
//! Nothing here knows what a unit IS. `sensei.module_edges` supplies modules and
//! `sensei.structure_edges` supplies files; the Cycles screen toggles between
//! them. Keeping the algorithm grain-free is what lets one implementation serve
//! both rather than two drifting copies.
//!
//! ## The node universe is GIVEN, never inferred from the edges
//!
//! [`analyse`] takes `units` separately. Deriving them from the dependency list
//! loses every unit that has no cross-unit dependency — measured on project
//! `sensei`, 9 modules appear only as a dependency on THEMSELVES and would have
//! vanished from the report entirely, carrying no layer and appearing in no
//! class, with nothing saying they were dropped.
//!
//! ## Only a cycle can climb, and that is the finding
//!
//! For every edge BETWEEN components, `layer(source) < layer(target)` holds by
//! the definition of longest path — any level derived from a graph strictly
//! descends along that graph's own edges. So nothing between components climbs,
//! and [`only_a_cycle_can_climb`] pins it.
//!
//! INSIDE a component is the opposite, and an earlier version of this module got
//! it wrong. `a → b → c → a` cannot be drawn with every arrow pointing down: one
//! of the three closes the loop, and that one IS the call breaking the downward
//! flow. Because a cycle has no internal depth its members share a rank, so
//! layer arithmetic calls every such edge `level` — true, and useless, because
//! `ViolationsControl` filters on `up`. A graph whose only violations were its
//! cycles therefore showed none at all: measured on project `sensei` at module
//! grain, 88 `level` edges, every one inside the single 22-module component.
//!
//! [`feedback_order`] fixes that by ordering each component's members so as few
//! edges as possible run backwards; the ones that do are marked
//! [`Conformance::Up`]. Every `up` edge is therefore a loop closing, which is
//! what makes "these calls break the downward flow" readable rather than a
//! count.
//!
//! What a DECLARED layering is still for is NARROWER than it looks: not
//! "are there violations" — that is answerable here — but "does the measured
//! structure match the one you intended". The mockup hardcodes one
//! (`docs/mockups/Sensei/lib/arch-data.js:59-65`) and flags itself "ILLUSTRATIVE
//! until the index exports them". Callers still report which kind of layering
//! produced a payload.

use std::collections::HashMap;

/// One directed dependency: `source` uses `target`, `occurrences` times.
///
/// The direction is "depends on", matching `sensei.edges` and the mockup's
/// arrows — which point AT what is depended on.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Dep {
    pub source: String,
    pub target: String,
    pub occurrences: i64,
}

impl Dep {
    /// Convenience for tests and call sites that build one inline.
    pub fn new(source: &str, target: &str, occurrences: i64) -> Self {
        Dep { source: source.to_string(), target: target.to_string(), occurrences }
    }
}

/// How a dependency sits against the layering.
///
/// The vocabulary is `@rokkit/graph`'s `Conformance`
/// (`dist/types.d.ts:144-151`), not a parallel one: the component derives these
/// labels itself when the host omits them, so inventing different names here
/// would mean the screen and the daemon disagreed about what a word meant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Conformance {
    /// Exactly one layer down. The conforming case.
    Down,
    /// More than one layer down. LEGAL under relaxed layering — which is the
    /// default everywhere from POSA's "Layers" to the Clean Architecture
    /// dependency rule — and worth seeing, which is why it has its own name
    /// instead of being folded in with `Up`.
    Skip,
    /// Climbing. The violation, and unreachable under a derived layering.
    Up,
    /// Within one layer — which is what every dependency inside a collapsed
    /// cycle is, since its members share the component's rank.
    Level,
}

/// Where one unit sits once cycles are collapsed and the rest is ranked.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Placed {
    pub id: String,
    /// 0 at the TOP, foundations at the bottom — `@rokkit/graph`'s convention
    /// (`dist/types.d.ts:76-79`), and the opposite of the natural
    /// longest-path-to-a-sink reading. A unit nothing depends on is 0.
    pub layer: usize,
    /// Index into [`Layering::cycles`]'s coordinate space — every unit has one,
    /// including the singletons that are in no cycle.
    ///
    /// POSITIONAL AND NOT DURABLE. It is assigned per call from the member
    /// ordering, so adding one unit can renumber the rest. Nothing may persist
    /// it or trend on it.
    pub component: usize,
    /// How many units share this component. 1 means "in no cycle".
    pub component_size: usize,
}

/// A dependency with its verdict attached.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ClassifiedDep {
    pub source: String,
    pub target: String,
    pub occurrences: i64,
    pub conformance: Conformance,
    /// True for the one edge inside a cycle that is cheapest to cut.
    pub weakest: bool,
}

/// A group of mutually dependent units, collapsed to one node.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Cycle {
    pub component: usize,
    /// Sorted, so the node's caption is stable between calls.
    pub members: Vec<String>,
    pub layer: usize,
    /// The dependencies that close the loop, heaviest first — the mockup lists
    /// them inside the collapsed box in exactly that order.
    pub inner: Vec<Dep>,
    /// The weakest link: fewest occurrences, so cutting it costs least.
    ///
    /// CHEAPEST, NOT NECESSARILY THE BACK EDGE, and the difference is worth
    /// knowing before quoting it as "cut here and the loop opens". Every edge
    /// inside a strongly-connected component lies on SOME cycle — strong
    /// connectivity guarantees a path back — so removing any of them breaks at
    /// least one. It does not follow that removing it makes the component
    /// acyclic: a 22-member component holds many cycles, and only removing the
    /// whole feedback set (the [`Conformance::Up`] edges) is guaranteed to.
    ///
    /// Measured on project `sensei` the two coincide — the cheapest inner edge
    /// IS the cheapest back edge, `senseid/adapters → senseid/tasks ×2` — but
    /// that is an observation, not a property. The rule stays the mockup's
    /// (`…v8.dc.html:2468`) because changing it needs a case where it is wrong,
    /// and this corpus does not provide one.
    ///
    /// `None` only when a component of two or more somehow carries no inner
    /// dependency, which cannot happen — mutual reachability requires them.
    pub cut: Option<Dep>,
}

/// Everything both diagrams read, computed once.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct Layering {
    pub units: Vec<Placed>,
    pub deps: Vec<ClassifiedDep>,
    /// Multi-member components only, by descending size then first member.
    pub cycles: Vec<Cycle>,
    /// Dependencies from a unit onto ITSELF. Dropped from the graph — they are
    /// a loop no layering can rank and would hang a naive longest-path walk —
    /// but kept here rather than discarded, because at module grain this is
    /// internal cohesion and silently losing it is how a unit disappears.
    pub self_deps: Vec<Dep>,
    /// Dependencies naming a unit the universe does not contain.
    ///
    /// REPORTED RATHER THAN ABSORBED, and this is not a theoretical case:
    /// measured 2026-10-05 on project `sensei`, 32 of 234 module dependencies
    /// (13.7%) name an endpoint that owns no file. The cause is upstream — an
    /// fqn's third segment is the module for most adapters but a SYMBOL for
    /// some (`c·senseid·ACCEPT_INPUT·item`), so a top-level symbol mints a
    /// module-shaped name no file is modal for.
    ///
    /// Minting a node for them would put a C macro on the diagram labelled as a
    /// module. Dropping them silently would hide a 13.7% hole behind a picture
    /// that looks complete. So they are dropped AND counted, and the caller puts
    /// the count next to the diagram.
    pub dropped: Vec<Dep>,
    /// Number of layers, i.e. `max(layer) + 1`, or 0 when there are no units.
    pub depth: usize,
}

/// Rank a dependency graph: cycles first, then layers over the condensation.
///
/// `units` is the node universe and is authoritative — a dependency naming a
/// unit outside it is DROPPED, because drawing an edge to a node the diagram
/// does not have is worse than omitting it.
pub fn analyse(units: &[String], deps: &[Dep]) -> Layering {
    // Sorted, deduplicated, and indexed once. Every later step works in index
    // space, and the sort is what makes the whole result order-independent:
    // the caller's row order comes from a database with no ORDER BY guarantee.
    let mut ids: Vec<String> = units.to_vec();
    ids.sort();
    ids.dedup();
    let index: HashMap<&str, usize> =
        ids.iter().enumerate().map(|(i, s)| (s.as_str(), i)).collect();
    let n = ids.len();

    let (edges, self_deps, dropped) = partition_deps(deps, &index);

    let adjacency = adjacency_of(n, &edges);
    let component_of = strongly_connected(n, &adjacency);
    let layer_of = rank_condensation(n, &edges, &component_of);

    let sizes = component_sizes(&component_of);
    let units_out = place(&ids, &component_of, &layer_of, &sizes);
    let cuts = weakest_links(&edges, &ids, &component_of);
    // One feedback order per MULTI-member component. A singleton has no inside,
    // so it contributes nothing and is skipped rather than ordered trivially.
    let mut feedback_pos: HashMap<usize, usize> = HashMap::new();
    let mut by_component: HashMap<usize, Vec<usize>> = HashMap::new();
    for u in 0..n {
        if sizes[component_of[u]] > 1 {
            by_component.entry(component_of[u]).or_default().push(u);
        }
    }
    for members in by_component.values() {
        feedback_pos.extend(feedback_order(members, &edges));
    }
    let deps_out = classify(&edges, &ids, &component_of, &layer_of, &cuts, &feedback_pos);
    let cycles = collapse(&ids, &component_of, &layer_of, &sizes, &edges, &cuts);
    let depth = layer_of.iter().copied().max().map_or(0, |m| m + 1);

    Layering { units: units_out, deps: deps_out, cycles, self_deps, dropped, depth }
}

/// Split the caller's dependencies into graph edges, self-dependencies, and the
/// ones naming a unit that is not in the universe.
///
/// Returned as three lists rather than filtered in place so each population can
/// be counted — a silent drop here is the defect this shape exists to prevent.
#[allow(clippy::type_complexity)]
fn partition_deps(
    deps: &[Dep],
    index: &HashMap<&str, usize>,
) -> (Vec<(usize, usize, i64)>, Vec<Dep>, Vec<Dep>) {
    let (mut edges, mut self_deps, mut dropped) = (Vec::new(), Vec::new(), Vec::new());
    for d in deps {
        match (index.get(d.source.as_str()), index.get(d.target.as_str())) {
            (Some(&s), Some(&t)) if s == t => self_deps.push(d.clone()),
            (Some(&s), Some(&t)) => edges.push((s, t, d.occurrences)),
            _ => dropped.push(d.clone()),
        }
    }
    // Parallel edges fold together: the caller may hand us one row per kind, and
    // a layering question is about WHETHER one unit depends on another, not how
    // many ways. Occurrences sum, which is what the cut heuristic weighs.
    edges.sort_unstable_by_key(|&(s, t, _)| (s, t));
    let mut folded: Vec<(usize, usize, i64)> = Vec::with_capacity(edges.len());
    for (s, t, w) in edges {
        match folded.last_mut() {
            Some(last) if last.0 == s && last.1 == t => last.2 += w,
            _ => folded.push((s, t, w)),
        }
    }
    self_deps.sort_by(|a, b| a.source.cmp(&b.source).then(b.occurrences.cmp(&a.occurrences)));
    (folded, self_deps, dropped)
}

/// Successor lists in index space.
fn adjacency_of(n: usize, edges: &[(usize, usize, i64)]) -> Vec<Vec<usize>> {
    let mut adj = vec![Vec::new(); n];
    for &(s, t, _) in edges {
        adj[s].push(t);
    }
    adj
}

/// Tarjan's strongly-connected components, iteratively.
///
/// ITERATIVE ON PURPOSE. The recursive formulation is shorter and overflows the
/// stack on a deep graph — the one input shape a code graph reliably produces.
///
/// Returns a component index per unit. Indices are assigned in Tarjan's
/// completion order, which depends only on the sorted unit order, so two calls
/// on the same graph number the components identically.
fn strongly_connected(n: usize, adjacency: &[Vec<usize>]) -> Vec<usize> {
    const UNVISITED: usize = usize::MAX;
    let (mut index_of, mut low, mut on_stack) =
        (vec![UNVISITED; n], vec![0usize; n], vec![false; n]);
    let mut component = vec![UNVISITED; n];
    let (mut stack, mut next_index, mut next_component) = (Vec::new(), 0usize, 0usize);

    for root in 0..n {
        if index_of[root] != UNVISITED {
            continue;
        }
        // (node, how many successors already walked) — the explicit frame that
        // replaces recursion.
        let mut call: Vec<(usize, usize)> = vec![(root, 0)];
        index_of[root] = next_index;
        low[root] = next_index;
        next_index += 1;
        stack.push(root);
        on_stack[root] = true;

        while let Some(&mut (v, ref mut walked)) = call.last_mut() {
            if *walked < adjacency[v].len() {
                let w = adjacency[v][*walked];
                *walked += 1;
                if index_of[w] == UNVISITED {
                    index_of[w] = next_index;
                    low[w] = next_index;
                    next_index += 1;
                    stack.push(w);
                    on_stack[w] = true;
                    call.push((w, 0));
                } else if on_stack[w] {
                    low[v] = low[v].min(index_of[w]);
                }
                continue;
            }
            // v is finished. If it is a root, everything above it on the stack
            // is its component.
            if low[v] == index_of[v] {
                while let Some(w) = stack.pop() {
                    on_stack[w] = false;
                    component[w] = next_component;
                    if w == v {
                        break;
                    }
                }
                next_component += 1;
            }
            call.pop();
            if let Some(&mut (parent, _)) = call.last_mut() {
                low[parent] = low[parent].min(low[v]);
            }
        }
    }
    component
}

/// Longest path from a source, over the CONDENSATION, in rokkit's direction.
///
/// `layer = 0` for a unit nothing depends on; a unit is one below the deepest
/// thing that depends on it. Members of one component share a rank, because the
/// condensation is what is ranked and a cycle has no internal order to rank by.
///
/// Kahn's order makes this a single pass with no recursion and no iteration
/// limit to tune: each component is finalised only after every component that
/// depends on it, so `max` over predecessors is already settled when read.
fn rank_condensation(
    n: usize,
    edges: &[(usize, usize, i64)],
    component_of: &[usize],
) -> Vec<usize> {
    let components = component_of.iter().copied().max().map_or(0, |m| m + 1);
    let mut succ: Vec<Vec<usize>> = vec![Vec::new(); components];
    let mut in_degree = vec![0usize; components];
    let mut seen = std::collections::HashSet::new();
    for &(s, t, _) in edges {
        let (cs, ct) = (component_of[s], component_of[t]);
        if cs == ct || !seen.insert((cs, ct)) {
            continue;
        }
        succ[cs].push(ct);
        in_degree[ct] += 1;
    }

    let mut layer = vec![0usize; components];
    let mut queue: std::collections::VecDeque<usize> =
        (0..components).filter(|&c| in_degree[c] == 0).collect();
    while let Some(c) = queue.pop_front() {
        for &d in &succ[c] {
            layer[d] = layer[d].max(layer[c] + 1);
            in_degree[d] -= 1;
            if in_degree[d] == 0 {
                queue.push_back(d);
            }
        }
    }
    (0..n).map(|u| layer[component_of[u]]).collect()
}

/// How many units share each component, indexed by component.
fn component_sizes(component_of: &[usize]) -> Vec<usize> {
    let components = component_of.iter().copied().max().map_or(0, |m| m + 1);
    let mut sizes = vec![0usize; components];
    for &c in component_of {
        sizes[c] += 1;
    }
    sizes
}

/// The per-unit rows, in the sorted unit order the caller can rely on.
fn place(
    ids: &[String],
    component_of: &[usize],
    layer_of: &[usize],
    sizes: &[usize],
) -> Vec<Placed> {
    ids.iter()
        .enumerate()
        .map(|(u, id)| Placed {
            id: id.clone(),
            layer: layer_of[u],
            component: component_of[u],
            component_size: sizes[component_of[u]],
        })
        .collect()
}

/// The cheapest edge to cut inside each multi-member component, by component.
///
/// FEWEST OCCURRENCES WINS, which is the mockup's rule
/// (`…v8.dc.html:2468`, `inner(i).reduce((a,b)=>b.n<a.n?b:a)`) and the honest
/// one to state: it is "the dependency used least", not a minimum feedback arc
/// set — that is NP-hard, and a heuristic dressed up as an optimum would be a
/// worse answer than a simple rule a reader can check.
///
/// Ties break on the unit names so the same graph always names the same cut.
fn weakest_links(
    edges: &[(usize, usize, i64)],
    ids: &[String],
    component_of: &[usize],
) -> HashMap<usize, (usize, usize)> {
    let mut best: HashMap<usize, (i64, &str, &str, usize, usize)> = HashMap::new();
    for &(s, t, w) in edges {
        if component_of[s] != component_of[t] {
            continue;
        }
        let key = (w, ids[s].as_str(), ids[t].as_str(), s, t);
        best.entry(component_of[s])
            .and_modify(|cur| {
                if (key.0, key.1, key.2) < (cur.0, cur.1, cur.2) {
                    *cur = key;
                }
            })
            .or_insert(key);
    }
    best.into_iter().map(|(c, (_, _, _, s, t))| (c, (s, t))).collect()
}

/// Order one component's members so that as few edges as possible run backwards
/// — Eades, Lin and Smyth's greedy linear arrangement (GR, 1993).
///
/// WHY A COMPONENT NEEDS AN ORDER AT ALL. Its members share a rank, because a
/// cycle has no internal depth. But `a → b → c → a` cannot be drawn with every
/// arrow pointing down: one of the three closes the loop, and THAT is the call
/// breaking the downward flow. Without an order there is nothing to call the
/// back edge, and every intra-cycle edge reads as "within the layer" — which is
/// how a graph whose only violations are its cycles showed none at all.
///
/// The edges running backwards in this order are a FEEDBACK ARC SET: remove them
/// all and the component is acyclic. GR bounds that set at `m/2 − n/6`, which
/// matters because the minimum one is NP-hard and a heuristic presented as an
/// optimum would be a worse answer than a stated heuristic.
///
/// The order is NOT a layer and must never become one — see
/// [`the_feedback_order_does_not_become_a_layer`].
///
/// Deterministic: ties break on the member's index, and indices come from the
/// sorted unit list, so the same component always yields the same back edges.
fn feedback_order(members: &[usize], edges: &[(usize, usize, i64)]) -> HashMap<usize, usize> {
    let inside: std::collections::HashSet<usize> = members.iter().copied().collect();
    let within: Vec<(usize, usize)> = edges
        .iter()
        .filter(|&&(s, t, _)| inside.contains(&s) && inside.contains(&t) && s != t)
        .map(|&(s, t, _)| (s, t))
        .collect();

    let mut alive: std::collections::BTreeSet<usize> = inside.iter().copied().collect();
    let (mut front, mut back): (Vec<usize>, Vec<usize>) = (Vec::new(), Vec::new());

    // Degrees over the LIVE subgraph only — the whole point of the peel is that
    // removing a node turns its neighbours into sources and sinks.
    let degrees = |alive: &std::collections::BTreeSet<usize>| {
        let mut out: HashMap<usize, (usize, usize)> = alive.iter().map(|&n| (n, (0, 0))).collect();
        for &(s, t) in &within {
            if alive.contains(&s) && alive.contains(&t) {
                out.entry(s).or_default().0 += 1; // out-degree
                out.entry(t).or_default().1 += 1; // in-degree
            }
        }
        out
    };

    while !alive.is_empty() {
        loop {
            let deg = degrees(&alive);
            // Sinks go to the BACK, in reverse discovery order, so the first
            // sink found ends up last — nothing can point away from it.
            if let Some(&n) = alive.iter().find(|n| deg[n].0 == 0) {
                alive.remove(&n);
                back.push(n);
                continue;
            }
            // Sources go to the FRONT: nothing points at them.
            if let Some(&n) = alive.iter().find(|n| deg[n].1 == 0) {
                alive.remove(&n);
                front.push(n);
                continue;
            }
            break;
        }
        if alive.is_empty() {
            break;
        }
        // Everything left has both an in- and an out-edge, so the component is
        // cyclic here and SOME edge has to run backwards. Taking the node whose
        // out-degree most exceeds its in-degree costs the fewest of them.
        let deg = degrees(&alive);
        let pick = *alive
            .iter()
            .max_by_key(|n| (deg[n].0 as i64 - deg[n].1 as i64, std::cmp::Reverse(**n)))
            .expect("alive is non-empty");
        alive.remove(&pick);
        front.push(pick);
    }

    back.reverse();
    front.extend(back);
    front.into_iter().enumerate().map(|(pos, n)| (n, pos)).collect()
}

/// Attach a verdict to every edge.
fn classify(
    edges: &[(usize, usize, i64)],
    ids: &[String],
    component_of: &[usize],
    layer_of: &[usize],
    cuts: &HashMap<usize, (usize, usize)>,
    feedback_pos: &HashMap<usize, usize>,
) -> Vec<ClassifiedDep> {
    edges
        .iter()
        .map(|&(s, t, w)| {
            let same = component_of[s] == component_of[t];
            let conformance = if same {
                // INSIDE a cycle. The members share a rank, so layer arithmetic
                // says "level" for all of them — which is true and useless. The
                // feedback order says which arrow closes the loop, and that one
                // is the call breaking the downward flow.
                match (feedback_pos.get(&s), feedback_pos.get(&t)) {
                    (Some(a), Some(b)) if a > b => Conformance::Up,
                    _ => Conformance::Level,
                }
            } else {
                // Signed, because `Up` has to be REACHABLE in this expression
                // even though the ranking makes it unreachable in practice. A
                // `usize` subtraction would panic instead of classifying, and a
                // branch that cannot be exercised is a branch nobody checks.
                match layer_of[t] as i64 - layer_of[s] as i64 {
                    1 => Conformance::Down,
                    d if d > 1 => Conformance::Skip,
                    0 => Conformance::Level,
                    _ => Conformance::Up,
                }
            };
            ClassifiedDep {
                source: ids[s].clone(),
                target: ids[t].clone(),
                occurrences: w,
                conformance,
                weakest: same && cuts.get(&component_of[s]) == Some(&(s, t)),
            }
        })
        .collect()
}

/// The multi-member components, as collapsed nodes.
///
/// `cuts` is passed in rather than recomputed: `classify` already needs it to
/// flag one edge, and deriving it twice is how the flag on an edge and the
/// `cut` on its cycle come to name different dependencies.
fn collapse(
    ids: &[String],
    component_of: &[usize],
    layer_of: &[usize],
    sizes: &[usize],
    edges: &[(usize, usize, i64)],
    cuts: &HashMap<usize, (usize, usize)>,
) -> Vec<Cycle> {
    // Members and the component's rank in one pass. Every member shares the
    // rank — it is the CONDENSATION that was ranked — so the last write says
    // the same thing as the first.
    let mut by_component: HashMap<usize, (Vec<String>, usize)> = HashMap::new();
    for (u, id) in ids.iter().enumerate() {
        if sizes[component_of[u]] > 1 {
            let entry = by_component.entry(component_of[u]).or_insert_with(|| (Vec::new(), 0));
            entry.0.push(id.clone());
            entry.1 = layer_of[u];
        }
    }

    let mut cycles: Vec<Cycle> = by_component
        .into_iter()
        .map(|(component, (mut members, layer))| {
            members.sort();
            let mut inner: Vec<Dep> = edges
                .iter()
                .filter(|&&(s, t, _)| component_of[s] == component && component_of[t] == component)
                .map(|&(s, t, w)| Dep::new(&ids[s], &ids[t], w))
                .collect();
            // Heaviest first, as the collapsed box lists them; the cut is the
            // tail by construction, and the name ties keep it deterministic.
            inner.sort_by(|a, b| {
                b.occurrences
                    .cmp(&a.occurrences)
                    .then(a.source.cmp(&b.source))
                    .then(a.target.cmp(&b.target))
            });
            // Taken from `inner` rather than re-read off `edges`, so the edge
            // the box flags and the edge the screen lists are the same object.
            let cut = cuts
                .get(&component)
                .and_then(|&(s, t)| inner.iter().find(|d| d.source == ids[s] && d.target == ids[t]))
                .cloned();
            Cycle { component, members, layer, inner, cut }
        })
        .collect();
    cycles.sort_by(|a, b| {
        b.members.len().cmp(&a.members.len()).then(a.members[0].cmp(&b.members[0]))
    });
    cycles
}

#[cfg(test)]
mod tests {
    use super::*;

    fn units(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    fn layer_of(l: &Layering, id: &str) -> usize {
        l.units.iter().find(|u| u.id == id).unwrap_or_else(|| panic!("{id} missing")).layer
    }

    fn verdict(l: &Layering, s: &str, t: &str) -> Conformance {
        l.deps
            .iter()
            .find(|d| d.source == s && d.target == t)
            .unwrap_or_else(|| panic!("{s} -> {t} missing"))
            .conformance
    }

    /// The caller sits ABOVE what it calls, in rokkit's 0-at-the-top direction.
    ///
    /// Mutation that must break this test: swap `layer[d].max(layer[c] + 1)` for
    /// `layer[c].max(layer[d] + 1)` in `rank_condensation`.
    #[test]
    fn a_caller_ranks_above_what_it_calls() {
        let l = analyse(&units(&["a", "b"]), &[Dep::new("a", "b", 1)]);
        assert_eq!(layer_of(&l, "a"), 0, "nothing depends on a, so a is the top");
        assert_eq!(layer_of(&l, "b"), 1, "b is used by a, so it sits one below");
        assert_eq!(verdict(&l, "a", "b"), Conformance::Down);
        assert_eq!(l.depth, 2);
    }

    /// A unit with no dependency at all is still placed.
    ///
    /// THE NODE-UNIVERSE DEFECT, pinned. Deriving the universe from the edge
    /// list loses it silently, carrying no layer and appearing in no class —
    /// measured on project `sensei`, 9 modules are in exactly this state.
    ///
    /// Mutation that must break this test: build `ids` from `deps` instead of
    /// from `units`.
    #[test]
    fn a_unit_in_no_dependency_is_still_placed() {
        let l = analyse(&units(&["a", "b", "lonely"]), &[Dep::new("a", "b", 1)]);
        assert_eq!(l.units.len(), 3, "every unit in the universe is placed");
        assert_eq!(layer_of(&l, "lonely"), 0);
    }

    /// Mutually dependent units collapse into one component and share a rank.
    ///
    /// Mutation that must break this test: return each node as its own
    /// component from `strongly_connected`.
    #[test]
    fn a_mutual_pair_collapses_into_one_component_sharing_a_layer() {
        let l = analyse(&units(&["a", "b"]), &[Dep::new("a", "b", 2), Dep::new("b", "a", 3)]);
        let (a, b) = (&l.units[0], &l.units[1]);
        assert_eq!(a.component, b.component, "a and b are one component");
        assert_eq!(a.component_size, 2);
        assert_eq!(a.layer, b.layer, "a cycle has no internal order to rank by");
        // One of the two closes the loop; which one is the feedback order's
        // business, so this asserts the PAIR rather than a direction.
        assert_eq!(
            l.deps.iter().filter(|d| d.conformance == Conformance::Up).count(),
            1,
            "a cycle closes with exactly one edge"
        );
        assert_eq!(l.cycles.len(), 1);
        assert_eq!(l.cycles[0].members, vec!["a", "b"]);
    }

    /// The cut is the dependency used LEAST, which is the mockup's rule.
    ///
    /// Mutation that must break this test: pick the maximum in `weakest_links`.
    #[test]
    fn a_cycle_names_the_dependency_that_is_cheapest_to_cut() {
        let l = analyse(
            &units(&["a", "b", "c"]),
            &[Dep::new("a", "b", 9), Dep::new("b", "c", 3), Dep::new("c", "a", 5)],
        );
        assert_eq!(l.cycles.len(), 1, "one three-member cycle");
        assert_eq!(l.cycles[0].members, vec!["a", "b", "c"]);
        assert_eq!(l.cycles[0].cut, Some(Dep::new("b", "c", 3)), "3 is the fewest");
        assert_eq!(
            l.cycles[0].inner.first().map(|d| d.occurrences),
            Some(9),
            "inner edges are listed heaviest first"
        );
        let weakest: Vec<_> =
            l.deps.iter().filter(|d| d.weakest).map(|d| (&d.source, &d.target)).collect();
        assert_eq!(weakest.len(), 1, "exactly one edge carries the cut flag");
    }

    /// A dependency reaching more than one layer down is `skip`, not `down`.
    ///
    /// Legal under relaxed layering — which is why it has its own name rather
    /// than being reported as a violation.
    ///
    /// Mutation that must break this test: collapse the `d > 1` arm into
    /// `Conformance::Down`.
    #[test]
    fn a_dependency_reaching_past_a_layer_is_a_skip() {
        let l = analyse(
            &units(&["a", "b", "c"]),
            &[Dep::new("a", "b", 1), Dep::new("b", "c", 1), Dep::new("a", "c", 1)],
        );
        assert_eq!(layer_of(&l, "a"), 0);
        assert_eq!(layer_of(&l, "b"), 1);
        assert_eq!(layer_of(&l, "c"), 2);
        assert_eq!(verdict(&l, "a", "b"), Conformance::Down);
        assert_eq!(verdict(&l, "b", "c"), Conformance::Down);
        assert_eq!(verdict(&l, "a", "c"), Conformance::Skip, "two layers down is a skip");
    }

    /// A MUTUAL PAIR has exactly one climbing edge, not zero.
    ///
    /// `a ↔ b` cannot be drawn with both arrows pointing down — one of them
    /// closes the loop, and that one IS the call that breaks the downward flow.
    /// Before [`feedback_order`], both were `Level` (the two share a rank, which
    /// is correct), and `ViolationsControl` filters on `up` — so the screen
    /// showed no violations over a graph whose every cycle edge is one.
    ///
    /// Mutation that must break this test: return `Conformance::Level` for every
    /// intra-component edge.
    #[test]
    fn a_mutual_pair_has_exactly_one_climbing_edge() {
        let l = analyse(&units(&["a", "b"]), &[Dep::new("a", "b", 2), Dep::new("b", "a", 3)]);
        assert_eq!(
            l.deps.iter().filter(|d| d.conformance == Conformance::Up).count(),
            1,
            "one of the two closes the loop: {:?}",
            l.deps
        );
        // The other is within the layer, because the pair shares a rank.
        assert_eq!(l.deps.iter().filter(|d| d.conformance == Conformance::Level).count(), 1);
    }

    /// A three-cycle closes with ONE edge, not three.
    ///
    /// The feedback order is what makes this a single back edge rather than
    /// "every edge in a cycle climbs" — the latter is true of no ordering and
    /// would make the finding unactionable.
    ///
    /// Mutation that must break this test: mark every intra-component edge `Up`.
    #[test]
    fn a_three_cycle_closes_with_one_edge() {
        let l = analyse(
            &units(&["a", "b", "c"]),
            &[Dep::new("a", "b", 9), Dep::new("b", "c", 3), Dep::new("c", "a", 5)],
        );
        assert_eq!(
            l.deps.iter().filter(|d| d.conformance == Conformance::Up).count(),
            1,
            "exactly one back edge: {:?}",
            l.deps
        );
        assert_eq!(l.deps.iter().filter(|d| d.conformance == Conformance::Level).count(), 2);
    }

    /// EVERY node of a cycle keeps its rank; only the EDGES are ordered.
    ///
    /// The feedback order exists to say which arrow closes the loop. It must not
    /// leak into the layering — a cycle has no internal depth, and splitting its
    /// members across bands would invent one.
    ///
    /// Mutation that must break this test: add the member's feedback position to
    /// its layer.
    #[test]
    fn the_feedback_order_does_not_become_a_layer() {
        let l = analyse(
            &units(&["top", "x", "y", "z", "leaf"]),
            &[
                Dep::new("top", "x", 1),
                Dep::new("x", "y", 1),
                Dep::new("y", "z", 1),
                Dep::new("z", "x", 1),
                Dep::new("y", "leaf", 1),
            ],
        );
        assert_eq!(layer_of(&l, "x"), layer_of(&l, "y"), "cycle members share a rank");
        assert_eq!(layer_of(&l, "y"), layer_of(&l, "z"));
        assert_eq!(layer_of(&l, "top"), 0);
        assert_eq!(layer_of(&l, "leaf"), layer_of(&l, "x") + 1);
        assert_eq!(l.depth, 3, "three bands, not five");
    }

    /// Climbing is confined to cycles — NOTHING between components climbs.
    ///
    /// This is the half of the original vacuity claim that survives, and it is
    /// what makes the `up` edges readable: every one of them is a loop closing,
    /// so "these calls break the downward flow" names cycles and nothing else.
    ///
    /// Mutation that must break this test: classify an inter-component edge by
    /// `layer_of[s] - layer_of[t]` instead of `layer_of[t] - layer_of[s]`.
    #[test]
    fn only_a_cycle_can_climb() {
        let l = analyse(
            &units(&["app", "api", "core", "util", "db", "x", "y"]),
            &[
                Dep::new("app", "api", 7),
                Dep::new("api", "core", 5),
                Dep::new("core", "util", 11),
                Dep::new("app", "util", 2),
                Dep::new("core", "db", 1),
                Dep::new("db", "util", 4),
                Dep::new("x", "y", 3),
                Dep::new("y", "x", 1),
                Dep::new("x", "core", 6),
            ],
        );
        let cycle_members: std::collections::HashSet<&str> =
            l.cycles.iter().flat_map(|c| c.members.iter().map(String::as_str)).collect();
        assert!(!cycle_members.is_empty(), "the fixture has a cycle, so this is not vacuous");
        for d in l.deps.iter().filter(|d| d.conformance == Conformance::Up) {
            assert!(
                cycle_members.contains(d.source.as_str())
                    && cycle_members.contains(d.target.as_str()),
                "{} -> {} climbs but is not inside a cycle",
                d.source,
                d.target
            );
        }
        assert_eq!(
            l.deps.iter().filter(|d| d.conformance == Conformance::Up).count(),
            1,
            "the one 2-cycle contributes exactly one"
        );
    }

    /// A unit depending on ITSELF is kept as a fact, not silently dropped.
    ///
    /// It is removed from the graph — a self-loop is a cycle no rank can order,
    /// and it is what hangs a naive longest-path walk — but a unit whose only
    /// dependency is on itself must still appear.
    ///
    /// Mutation that must break this test: drop the `s == t` arm in
    /// `partition_deps` so self-dependencies fall into `edges`.
    #[test]
    fn a_unit_depending_only_on_itself_is_kept_and_placed() {
        let l = analyse(&units(&["solo"]), &[Dep::new("solo", "solo", 12)]);
        assert_eq!(l.self_deps, vec![Dep::new("solo", "solo", 12)]);
        assert!(l.deps.is_empty(), "a self-dependency is not a graph edge");
        assert_eq!(layer_of(&l, "solo"), 0);
        assert_eq!(l.units[0].component_size, 1, "depending on itself is not a cycle");
        assert!(l.cycles.is_empty());
    }

    /// The answer does not depend on the order the rows arrived in.
    ///
    /// The caller reads a view with no `ORDER BY`, so two runs can hand us the
    /// same graph in different orders; a payload that changed would make every
    /// screenshot and every snapshot test a coin flip.
    ///
    /// Mutation that must break this test: remove the `ids.sort()` in `analyse`.
    #[test]
    fn the_result_does_not_depend_on_input_order() {
        let u = units(&["a", "b", "c", "d"]);
        let forward = [
            Dep::new("a", "b", 1),
            Dep::new("b", "c", 2),
            Dep::new("c", "b", 3),
            Dep::new("c", "d", 4),
        ];
        let mut reversed: Vec<Dep> = forward.to_vec();
        reversed.reverse();
        let mut shuffled_units = u.clone();
        shuffled_units.reverse();
        assert_eq!(analyse(&u, &forward), analyse(&shuffled_units, &reversed));
    }

    /// A dependency naming a unit the universe does not contain is dropped.
    ///
    /// Drawing an edge to a node the diagram has no box for is worse than
    /// omitting it — the line would end in empty space.
    ///
    /// Mutation that must break this test: make the `_ =>` arm in
    /// `partition_deps` push into `edges` with a minted index, or stop carrying
    /// `dropped` out of `analyse`.
    ///
    /// The universe holds two units, not one, so a minted index lands on a REAL
    /// node and shows up as an extra edge rather than panicking out of bounds —
    /// the test has to distinguish "dropped" from "crashed".
    ///
    /// The second half is the one that matters in production: on project
    /// `sensei` this population is 13.7% of module dependencies, so a diagram
    /// that omits it without saying so is 13.7% wrong and looks complete.
    #[test]
    fn a_dependency_on_a_unit_outside_the_universe_is_dropped_and_counted() {
        let l = analyse(&units(&["a", "b"]), &[Dep::new("a", "b", 1), Dep::new("a", "ghost", 5)]);
        assert_eq!(l.deps.len(), 1, "only the dependency between two known units");
        assert_eq!(l.deps[0].target, "b");
        assert_eq!(l.units.len(), 2, "and no node was minted for the ghost");
        assert_eq!(l.dropped, vec![Dep::new("a", "ghost", 5)], "the omission is reported");
    }

    /// Parallel dependencies fold, and their occurrences sum.
    ///
    /// The caller may pass one row per edge kind. Layering asks WHETHER one unit
    /// depends on another; the count is only the cut heuristic's weight, so it
    /// has to be the total rather than whichever row arrived last.
    ///
    /// Mutation that must break this test: `last.2 = w` instead of `+= w`.
    #[test]
    fn parallel_dependencies_fold_and_their_counts_sum() {
        let l = analyse(
            &units(&["a", "b"]),
            &[Dep::new("a", "b", 3), Dep::new("a", "b", 4), Dep::new("a", "b", 5)],
        );
        assert_eq!(l.deps.len(), 1, "one relationship, not three");
        assert_eq!(l.deps[0].occurrences, 12);
    }

    /// A cycle is collapsed BEFORE ranking, so what hangs off it still ranks.
    ///
    /// Mutation that must break this test: rank over `edges` directly instead of
    /// over the condensation in `rank_condensation`.
    #[test]
    fn a_cycle_is_collapsed_before_the_rest_is_ranked() {
        let l = analyse(
            &units(&["top", "x", "y", "leaf"]),
            &[
                Dep::new("top", "x", 1),
                Dep::new("x", "y", 1),
                Dep::new("y", "x", 1),
                Dep::new("y", "leaf", 1),
            ],
        );
        assert_eq!(layer_of(&l, "top"), 0);
        assert_eq!(layer_of(&l, "x"), 1);
        assert_eq!(layer_of(&l, "y"), 1, "both cycle members share the component's rank");
        assert_eq!(layer_of(&l, "leaf"), 2);
        assert_eq!(l.depth, 3);
    }

    /// An empty graph is an empty answer, not a panic.
    #[test]
    fn no_units_is_an_empty_layering() {
        let l = analyse(&[], &[]);
        assert_eq!(l, Layering::default());
        assert_eq!(l.depth, 0);
    }

    /// A long chain does not overflow the stack.
    ///
    /// Tarjan is written iteratively for exactly this: the recursive form blows
    /// the stack on a deep graph, which a code graph reliably produces.
    ///
    /// Mutation that must break this test: drop the `+ 1` from
    /// `layer[d].max(layer[c] + 1)` — the depth assertion is what keeps this
    /// from being a bare "it did not crash".
    #[test]
    fn a_very_deep_chain_does_not_overflow_the_stack() {
        let ids: Vec<String> = (0..20_000).map(|i| format!("m{i:06}")).collect();
        let deps: Vec<Dep> =
            (0..ids.len() - 1).map(|i| Dep::new(&ids[i], &ids[i + 1], 1)).collect();
        let l = analyse(&ids, &deps);
        assert_eq!(l.depth, 20_000);
        assert!(l.cycles.is_empty());
    }
}
