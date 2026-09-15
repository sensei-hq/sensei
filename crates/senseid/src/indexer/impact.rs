//! Impact — the blast radius of a change, and the measured edge of it.
//!
//! "What breaks if I change this" is the question the graph exists to answer,
//! and it is a REVERSE reachability query: every symbol that reaches the one
//! being changed, out to some distance. The traversal itself is unremarkable.
//! What this module is actually for is the second half of the answer.
//!
//! **A blast radius that does not say where it stops is a lie told confidently.**
//! A caller the ladder could not place is not absent from the codebase; it is
//! absent from the graph. Answer "3 callers" over a graph that failed to place
//! nine more and the reader refactors with a number that is wrong by a factor
//! of four, with nothing on screen to suggest doubt.
//!
//! So every query returns two populations:
//!
//! - [`Impact::reached`] — callers the ladder PLACED. Each one is an edge some
//!   rung vouched for.
//! - [`Impact::boundary`] — use sites that NAME something in the radius and
//!   that the ladder could not place, each carrying the [`Reason`] it could
//!   not. This is the amount by which `reached` understates, stated rather
//!   than hidden.
//!
//! The reason is already on every unresolved reference — `resolve` has attached
//! one to each since the ladder was built, and A3 asserts the coverage is 100%
//! in every language. Until now it stopped at the indexer boundary and no
//! reader ever saw it. This module is the door.
//!
//! The alternative, taken by at least one shipping code-graph product, is to
//! guess: score same-named declarations by directory proximity and emit the
//! best one as an edge at 0.4 confidence. That inflates the radius with edges
//! nobody can distinguish from the vouched-for ones at the point of use. R4
//! says a wrong edge is worse than a missing one, and a missing edge that
//! announces itself is better than both.

// Reachable only from tests until stage 10. The LEGACY indexer under
// `crate::languages` still produces the graph the daemon serves, and the MCP
// handlers read it from Postgres; this module answers over the new indexer's
// facts, so it acquires its caller when the cutover gives those facts a home.
// See the note above the module list in `mod.rs`.
#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use super::facts::{FileFacts, Fqn, Reason, RefKind, Reference, Resolution, Span, Symbol};

/// A caller the ladder placed, and how far it sits from the change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reached {
    /// The symbol whose body holds the use site.
    pub symbol: Fqn,
    /// Hops from the symbol being changed. A direct caller is 1.
    pub depth: u32,
    /// The symbol in the radius that this one reaches. `via` at depth 1 is the
    /// root itself, which is what makes a path reconstructible without storing
    /// one per node.
    pub via: Fqn,
    pub kind: RefKind,
    pub at: Span,
    pub file: String,
}

/// A use site that names something in the radius and that the ladder could not
/// place — one unit of "the answer above is incomplete, here is where".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Boundary {
    /// The bare name at the use site, which is what matched something in the
    /// radius.
    pub name: String,
    /// Why the ladder stopped. The whole point of the struct.
    pub reason: Reason,
    /// The symbol whose body holds the site. NOT in [`Impact::reached`] — a
    /// site inside a symbol already in the radius cannot grow it, so it is not
    /// reported as an edge of it.
    pub in_symbol: Fqn,
    pub at: Span,
    pub file: String,
}

/// Boundary sites listed in one answer. The COUNT is never capped — see
/// [`Impact::boundary_total`].
pub const MAX_BOUNDARY_SITES: usize = 50;

/// The answer to "what does changing this touch".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Impact {
    pub of: Fqn,
    /// Placed callers, nearest first.
    pub reached: Vec<Reached>,
    /// Where the radius stops, and why. At most [`MAX_BOUNDARY_SITES`] of
    /// them — [`Impact::boundary_total`] is how many there are.
    pub boundary: Vec<Boundary>,
    /// Every site that casts doubt on this radius, including the ones
    /// [`Impact::boundary`] had no room for. A list that silently stood in for
    /// a population would be the same overconfident answer this module exists
    /// to prevent, one level down.
    pub boundary_total: usize,
    /// Every doubting site by reason, tallied over the WHOLE population before
    /// the list was capped. Counting the capped page instead left the report's
    /// own two totals — 193,902 sites against a 59,987-site breakdown —
    /// irreconcilable on the face of it.
    boundary_reasons: BTreeMap<&'static str, usize>,
    /// True when the depth limit cut the traversal short — there are more
    /// placed callers beyond it. Distinct from a boundary, which is the graph
    /// running out rather than the query.
    pub truncated: bool,
}

impl Impact {
    /// How many placed callers sit at each distance.
    pub fn by_depth(&self) -> BTreeMap<u32, usize> {
        let mut out = BTreeMap::new();
        for reached in &self.reached {
            *out.entry(reached.depth).or_insert(0) += 1;
        }
        out
    }

    /// How many doubting sites each reason accounts for, over the whole
    /// population — `by_reason().values().sum()` equals
    /// [`Impact::boundary_total`] whether the list was capped or not.
    ///
    /// Keyed by the stable label rather than by [`Reason`], which is
    /// deliberately not `Ord` — the variants have no natural order and giving
    /// them one by declaration position would make a reordering of the enum a
    /// silent change to every report built on it.
    pub fn by_reason(&self) -> &BTreeMap<&'static str, usize> {
        &self.boundary_reasons
    }

    /// True when [`Impact::boundary`] lists fewer sites than there are.
    pub fn boundary_capped(&self) -> bool {
        self.boundary_total > self.boundary.len()
    }

    /// The share of candidate call sites the ladder actually placed, over the
    /// whole radius. `None` when there is nothing to divide — no callers and no
    /// boundary is not 100% confidence, it is no evidence.
    ///
    /// Over `boundary_total` and not over the listed sites, or capping the list
    /// would raise the confidence of exactly the symbols whose names are least
    /// discriminating.
    pub fn confidence(&self) -> Option<f64> {
        let total = self.reached.len() + self.boundary_total;
        (total > 0).then(|| self.reached.len() as f64 / total as f64)
    }
}

/// One placed edge, in the reverse direction the query walks.
struct Edge {
    from: Fqn,
    kind: RefKind,
    at: Span,
    file: String,
}

/// One use site the ladder could not place.
struct Miss {
    from: Fqn,
    reason: Reason,
    at: Span,
    file: String,
}

/// The resolved graph, indexed for reverse reachability.
///
/// Built once from the facts of a whole scan and then queried many times. It
/// borrows rather than owns because the facts are already in memory at the one
/// place this is constructed, and copying a corpus-sized fact set per query is
/// the kind of cost that turns an interactive tool into a batch one.
pub struct Graph<'a> {
    /// Placed use sites, keyed by what they name. The reverse of the call
    /// graph, which is the direction "who is affected" reads.
    callers_of: BTreeMap<&'a Fqn, Vec<Edge>>,
    /// Unplaced use sites, keyed by the BARE NAME at the site.
    ///
    /// Bare, because an unplaced site has no identity to key on — that is what
    /// unplaced means. Matching a radius member's name against it is a
    /// deliberate over-approximation: it is the set of sites that COULD be
    /// callers, which is exactly the doubt the answer needs to carry. It is
    /// never promoted to an edge (R4).
    misses_by_name: BTreeMap<&'a str, Vec<Miss>>,
    /// Every declared symbol, by identity. Gives a radius member its name so
    /// the miss index can be consulted for it.
    names: BTreeMap<&'a Fqn, &'a str>,
    /// Declarations by bare name, for turning what a person typed into the
    /// identities it could mean.
    declared: BTreeMap<&'a str, Vec<&'a Fqn>>,
}

impl<'a> Graph<'a> {
    /// Index a whole scan.
    pub fn of(files: &'a [FileFacts]) -> Self {
        let mut callers_of: BTreeMap<&'a Fqn, Vec<Edge>> = BTreeMap::new();
        let mut misses_by_name: BTreeMap<&'a str, Vec<Miss>> = BTreeMap::new();
        let mut names: BTreeMap<&'a Fqn, &'a str> = BTreeMap::new();
        let mut declared: BTreeMap<&'a str, Vec<&'a Fqn>> = BTreeMap::new();

        for file in files {
            for symbol in &file.symbols {
                let Symbol { fqn, name, .. } = symbol;
                names.insert(fqn, name.as_str());
                declared.entry(name.as_str()).or_default().push(fqn);
            }
            for reference in &file.references {
                let Reference { from, kind, at, target } = reference;
                match target {
                    Resolution::Resolved { fqn: to, .. } => {
                        callers_of.entry(to).or_default().push(Edge {
                            from: from.clone(),
                            kind: *kind,
                            at: *at,
                            file: file.path.clone(),
                        })
                    }
                    Resolution::Unresolved { reason, evidence } => {
                        misses_by_name.entry(evidence.name.as_str()).or_default().push(Miss {
                            from: from.clone(),
                            reason: *reason,
                            at: *at,
                            file: file.path.clone(),
                        })
                    }
                }
            }
        }
        Self { callers_of, misses_by_name, names, declared }
    }

    /// Every identity a bare name could mean.
    ///
    /// A list and not an `Option` because a name is not an identity: `read` is
    /// declared eleven times in this workspace. Picking one for the caller
    /// would be the same guess R4 forbids in the ladder, made at the query
    /// layer where it is even less visible.
    pub fn named(&self, name: &str) -> Vec<&'a Fqn> {
        // A name nothing declares has no identities. That empty is a fact of
        // the corpus, not a lookup that failed.
        match self.declared.get(name) {
            Some(identities) => identities.clone(),
            None => Vec::new(),
        }
    }

    /// Reverse-reachability from one symbol, out to `depth` hops.
    ///
    /// Breadth-first, so a symbol reachable by two paths is reported at its
    /// SHORTEST distance and exactly once. A cycle terminates for the same
    /// reason: an identity already seen is never re-queued.
    pub fn impact_of(&self, symbol: &Fqn, depth: u32) -> Impact {
        let mut reached: Vec<Reached> = Vec::new();
        let mut radius: BTreeSet<Fqn> = BTreeSet::new();
        let mut queue: VecDeque<(Fqn, u32)> = VecDeque::new();

        // The root is IN the radius from the start: a self-recursive call must
        // not report the symbol as its own caller, and a cycle back to it must
        // not either.
        radius.insert(symbol.clone());
        queue.push_back((symbol.clone(), 0));

        let mut cut_short = false;
        while let Some((at_symbol, distance)) = queue.pop_front() {
            let Some(edges) = self.callers_of.get(&at_symbol) else { continue };
            for edge in edges {
                if !radius.insert(edge.from.clone()) {
                    continue;
                }
                reached.push(Reached {
                    symbol: edge.from.clone(),
                    depth: distance + 1,
                    via: at_symbol.clone(),
                    kind: edge.kind,
                    at: edge.at,
                    file: edge.file.clone(),
                });
                // Past the limit the caller is still REPORTED — it was found —
                // but it is not expanded, and the query says so. Dropping it
                // instead would make `truncated` the only evidence that the
                // outermost ring exists at all.
                if distance + 1 < depth {
                    queue.push_back((edge.from.clone(), distance + 1));
                } else {
                    cut_short = true;
                }
            }
        }
        reached.sort_by(|a, b| {
            (a.depth, &a.file, a.at.start_line).cmp(&(b.depth, &b.file, b.at.start_line))
        });

        // Truncation means there is more to FIND, not merely more to visit. A
        // frontier symbol with no callers of its own ends the graph as surely
        // as the limit does, and reporting that as truncated would attach doubt
        // to an answer that is in fact complete.
        //
        // ...and the caller beyond it must be one the radius does not already
        // hold. A cycle back into the radius adds nobody, so `contains_key`
        // alone made a COMPLETE answer report itself truncated. Found by the
        // SQL twin of this traversal, whose fixture had a cycle where this
        // one's did not — the same rule written twice, wrong in both.
        let truncated = cut_short
            && reached.iter().filter(|r| r.depth == depth).any(|r| {
                self.callers_of
                    .get(&r.symbol)
                    .is_some_and(|edges| edges.iter().any(|e| !radius.contains(&e.from)))
            });

        let (boundary, boundary_total, boundary_reasons) = self.boundary_of(&radius);
        Impact {
            of: symbol.clone(),
            reached,
            boundary,
            boundary_total,
            boundary_reasons,
            truncated,
        }
    }

    /// Every unplaceable site naming something in the radius, minus the ones
    /// that could not grow it.
    ///
    /// A site whose own symbol is ALREADY in the radius is excluded: resolving
    /// it would add an edge between two symbols the reader has already been
    /// shown, so it is not a limit on the answer. What is left is the set of
    /// sites that, if the ladder could place them, would make the radius
    /// bigger — the statement of by how much this answer may be short.
    fn boundary_of(
        &self,
        radius: &BTreeSet<Fqn>,
    ) -> (Vec<Boundary>, usize, BTreeMap<&'static str, usize>) {
        let mut out: Vec<Boundary> = Vec::new();
        let mut seen: BTreeSet<(&str, &Fqn, u32, u32)> = BTreeSet::new();
        for member in radius {
            let Some(name) = self.names.get(member) else { continue };
            let Some(misses) = self.misses_by_name.get(*name) else { continue };
            for miss in misses {
                if !miss.reason.casts_doubt() || radius.contains(&miss.from) {
                    continue;
                }
                // One site can name two radius members only if two members
                // share a bare name, and then it is ONE doubt, not two.
                if !seen.insert((name, &miss.from, miss.at.start_line, miss.at.start_col)) {
                    continue;
                }
                out.push(Boundary {
                    name: (*name).to_string(),
                    reason: miss.reason,
                    in_symbol: miss.from.clone(),
                    at: miss.at,
                    file: miss.file.clone(),
                });
            }
        }
        out.sort_by(|a, b| {
            (&a.file, a.at.start_line, &a.name).cmp(&(&b.file, b.at.start_line, &b.name))
        });
        let total = out.len();
        let mut reasons: BTreeMap<&'static str, usize> = BTreeMap::new();
        for edge in &out {
            *reasons.entry(edge.reason.as_label()).or_insert(0) += 1;
        }
        out.truncate(MAX_BOUNDARY_SITES);
        (out, total, reasons)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexer::facts::{Language, SymbolKind};
    use crate::indexer::walked_rust;

    /// A four-deep chain in one module. Free functions calling free functions
    /// is the simplest thing the ladder places, which is what a traversal test
    /// wants: a failure here is the traversal's, not the ladder's.
    const CHAIN: &str = r#"
pub fn bottom() {}
pub fn middle() { bottom(); }
pub fn upper() { middle(); }
pub fn top() { upper(); }
"#;

    fn chain() -> Vec<FileFacts> {
        vec![walked_rust("chain", "src/chain.rs", CHAIN)]
    }

    /// The identity of a declared symbol, by its bare name. Panics rather than
    /// returning an option: a fixture whose symbol the walk did not find is a
    /// broken fixture, and a test that silently skipped it would be green.
    fn fqn_of(files: &[FileFacts], name: &str) -> Fqn {
        files
            .iter()
            .flat_map(|f| f.symbols.iter())
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("the fixture declares no {name}"))
            .fqn
            .clone()
    }

    /// The declared name at the end of an identity.
    ///
    /// Via `fqn::parse`, whose `tail` already excludes the trailing REACH
    /// segment. Splitting on the separator and taking the last piece yields
    /// `item` for every one of these — the reach, not the name — which is what
    /// the first run of these tests asserted against.
    fn last_segment(fqn: &Fqn) -> String {
        crate::indexer::fqn::parse(fqn.as_str())
            .expect("an fqn the walk minted parses")
            .tail
            .last()
            .unwrap_or(&"")
            .to_string()
    }

    fn names_at(impact: &Impact, depth: u32) -> Vec<String> {
        let mut out: Vec<String> = impact
            .reached
            .iter()
            .filter(|r| r.depth == depth)
            .map(|r| last_segment(&r.symbol))
            .collect();
        out.sort();
        out
    }

    #[test]
    fn a_chain_reports_each_caller_at_its_distance() {
        let files = chain();
        let graph = Graph::of(&files);
        let impact = graph.impact_of(&fqn_of(&files, "bottom"), 3);

        assert_eq!(names_at(&impact, 1), ["middle"], "the direct caller sits at 1");
        assert_eq!(names_at(&impact, 2), ["upper"]);
        assert_eq!(names_at(&impact, 3), ["top"]);
        assert_eq!(impact.reached.len(), 3, "and nothing else is in the radius");
        assert!(!impact.truncated, "the graph ran out before the limit did");
    }

    #[test]
    fn the_depth_limit_reports_the_outermost_ring_and_says_more_remains() {
        let files = chain();
        let graph = Graph::of(&files);
        let impact = graph.impact_of(&fqn_of(&files, "bottom"), 2);

        assert_eq!(names_at(&impact, 1), ["middle"]);
        assert_eq!(names_at(&impact, 2), ["upper"], "the ring AT the limit is still reported");
        assert_eq!(impact.reached.len(), 2, "`top` is beyond it");
        assert!(impact.truncated, "and the answer says so, because `upper` has a caller");
    }

    /// The distinction the flag exists for. Stopping because the graph ended is
    /// a complete answer; stopping because the query ended is not. A `truncated`
    /// that cannot tell them apart makes every exact answer look doubtful.
    #[test]
    fn a_frontier_whose_symbols_have_no_callers_is_not_truncated() {
        let files = chain();
        let graph = Graph::of(&files);
        let impact = graph.impact_of(&fqn_of(&files, "bottom"), 4);

        assert_eq!(impact.reached.len(), 3);
        assert!(!impact.truncated, "depth 4 over a 3-deep chain is exact, not cut short");
    }

    /// A cycle back INTO the radius is not evidence of anything beyond it.
    ///
    /// `top` sits at the depth limit and does have a caller — `bottom`, the
    /// root. Reporting that as truncated tells a reader a complete answer might
    /// be partial, and the only way to act on it is to re-run deeper and get
    /// the same thing. Found by the SQL twin of this traversal; the rule was
    /// written twice and was wrong in both.
    #[test]
    fn a_caller_already_in_the_radius_does_not_make_the_answer_look_truncated() {
        let files = vec![walked_rust(
            "ring",
            "src/ring.rs",
            r#"
pub fn bottom() { top(); }
pub fn middle() { bottom(); }
pub fn upper() { middle(); }
pub fn top() { upper(); }
"#,
        )];
        let graph = Graph::of(&files);
        let exact = graph.impact_of(&fqn_of(&files, "bottom"), 3);

        assert_eq!(names_at(&exact, 3), ["top"], "`top` is at the limit");
        assert!(
            !exact.truncated,
            "and its only caller is `bottom`, the root — already in the radius, so nothing \
             lies beyond and the answer is exact"
        );
    }

    #[test]
    fn a_cycle_terminates_and_the_root_is_never_its_own_caller() {
        let files = vec![walked_rust(
            "cyc",
            "src/cyc.rs",
            r#"
pub fn ping() { pong(); }
pub fn pong() { ping(); }
pub fn recurse() { recurse(); }
"#,
        )];
        let graph = Graph::of(&files);

        let two = graph.impact_of(&fqn_of(&files, "ping"), 10);
        assert_eq!(names_at(&two, 1), ["pong"]);
        assert_eq!(two.reached.len(), 1, "the cycle back to `ping` does not re-report it");

        let self_call = graph.impact_of(&fqn_of(&files, "recurse"), 10);
        assert!(self_call.reached.is_empty(), "a function is not in its own blast radius");
    }

    #[test]
    fn a_shorter_path_wins_and_a_symbol_is_reported_once() {
        let files = vec![walked_rust(
            "d",
            "src/d.rs",
            r#"
pub fn leaf() {}
pub fn mid() { leaf(); }
pub fn both() { leaf(); mid(); }
"#,
        )];
        let graph = Graph::of(&files);
        let impact = graph.impact_of(&fqn_of(&files, "leaf"), 5);

        assert_eq!(names_at(&impact, 1), ["both", "mid"], "`both` calls leaf directly");
        assert_eq!(impact.reached.len(), 2, "and is not reported again via `mid`");
    }

    #[test]
    fn named_returns_every_identity_a_bare_name_could_mean() {
        let files = vec![
            walked_rust("one", "src/one.rs", "pub fn twin() {}"),
            walked_rust("two", "src/two.rs", "pub fn twin() {}"),
        ];
        let graph = Graph::of(&files);
        assert_eq!(graph.named("twin").len(), 2, "two declarations, two identities, no pick");
        assert!(graph.named("absent").is_empty());
    }

    #[test]
    fn the_reason_labels_round_trip_and_cover_every_variant() {
        for reason in Reason::ALL {
            assert_eq!(
                Reason::from_label(reason.as_label()),
                Some(*reason),
                "{reason:?} does not survive its own label"
            );
        }
        assert_eq!(Reason::ALL.len(), 10, "a new variant needs a label and a place in ALL");
        assert_eq!(Reason::from_label("not_a_reason"), None);
    }

    /// The module's reason to exist, on the smallest case that shows it.
    ///
    /// Three sites call `start`. One has a typed receiver and is placed; two do
    /// not and are NOT placed. An answer of "1 caller" is defensible and wrong
    /// by two thirds. The boundary is the two, each carrying the reason the
    /// ladder gives for stopping — which is the reason that has been on the
    /// reference all along with no reader to show it to.
    #[test]
    fn a_caller_the_ladder_could_not_place_is_a_boundary_that_names_its_reason() {
        let files = vec![walked_rust(
            "e",
            "src/e.rs",
            r#"
pub struct Engine;
impl Engine {
    pub fn start(&self) {}
}
pub fn boot(e: &Engine) { e.start(); }
pub fn blind() { let e = make(); e.start(); }
pub fn viaparam(e: Mystery) { e.start(); }
"#,
        )];
        let graph = Graph::of(&files);
        let impact = graph.impact_of(&fqn_of(&files, "start"), 3);

        assert_eq!(names_at(&impact, 1), ["boot"], "only the typed receiver is placed");

        let mut sites: Vec<(&str, &str)> =
            impact.boundary.iter().map(|b| (b.reason.as_label(), b.name.as_str())).collect();
        sites.sort_unstable();
        assert_eq!(
            sites,
            [("no_import_in_scope", "start"), ("receiver_type_unknown", "start")],
            "and the two it could not place are reported WITH the reason, not dropped"
        );
        assert!(
            impact.boundary.iter().all(|b| b.at.start_line > 0 && !b.file.is_empty()),
            "every boundary site is somewhere a person can go and look"
        );
        assert_eq!(impact.confidence(), Some(1.0 / 3.0), "one placed of three candidates");
    }

    /// A miss cannot widen a radius it is already inside.
    ///
    /// Counting it would make a symbol look less certain the MORE of it the
    /// ladder placed — the caller is in the answer either way, so resolving its
    /// second, unplaced site adds nobody.
    #[test]
    fn a_miss_inside_the_radius_is_not_a_limit_on_it() {
        let files = vec![walked_rust(
            "f",
            "src/f.rs",
            r#"
pub struct Thing;
impl Thing {
    pub fn poke(&self) {}
}
pub fn caller(t: &Thing) { t.poke(); let u = make(); u.poke(); }
"#,
        )];
        let graph = Graph::of(&files);
        let impact = graph.impact_of(&fqn_of(&files, "poke"), 3);

        assert_eq!(names_at(&impact, 1), ["caller"]);
        assert!(
            impact.boundary.is_empty(),
            "the unplaced `u.poke()` sits inside `caller`, which is already in the radius: {:?}",
            impact.boundary
        );
        assert_eq!(impact.confidence(), Some(1.0), "so the answer is complete, and says so");
    }

    /// No callers and no doubt is not certainty.
    #[test]
    fn a_symbol_nothing_references_has_no_confidence_to_report() {
        let files = vec![walked_rust("g", "src/g.rs", "pub fn lonely() {}")];
        let graph = Graph::of(&files);
        let impact = graph.impact_of(&fqn_of(&files, "lonely"), 3);

        assert!(impact.reached.is_empty());
        assert!(impact.boundary.is_empty());
        assert_eq!(impact.confidence(), None, "nothing divided by nothing is not 100%");
    }

    #[test]
    fn the_reasons_in_a_radius_are_counted_by_label() {
        let files = vec![walked_rust(
            "h",
            "src/h.rs",
            r#"
pub struct Box2;
impl Box2 {
    pub fn open(&self) {}
}
pub fn one() { let a = make(); a.open(); }
pub fn two() { let b = make(); b.open(); }
pub fn three(c: Mystery) { c.open(); }
"#,
        )];
        let graph = Graph::of(&files);
        let impact = graph.impact_of(&fqn_of(&files, "open"), 3);

        let counts = impact.by_reason();
        assert_eq!(counts.get("receiver_type_unknown"), Some(&2));
        assert_eq!(counts.get("no_import_in_scope"), Some(&1));
        assert_eq!(counts.values().sum::<usize>(), impact.boundary.len());
    }

    #[test]
    fn the_placed_callers_are_counted_by_distance() {
        let files = chain();
        let graph = Graph::of(&files);
        let impact = graph.impact_of(&fqn_of(&files, "bottom"), 3);
        assert_eq!(impact.by_depth(), BTreeMap::from([(1, 1), (2, 1), (3, 1)]));
    }

    // ── over the real corpus ─────────────────────────────────────────────

    /// THE canonical impact report. Fixed rows, one column per language.
    ///
    ///     cargo test -p senseid --bin senseid -- --ignored --nocapture \
    ///       indexer::impact::tests::report
    ///
    /// Same contract as `acceptance::report` and for the same reason: a number
    /// retyped into prose acquires a new label each time, and a reader cannot
    /// tell a movement from a rephrasing.
    #[test]
    #[ignore]
    fn report() {
        let corpus = crate::indexer::acceptance::read_the_corpus();
        let files: Vec<FileFacts> = corpus.into_iter().map(|r| r.facts).collect();
        let graph = Graph::of(&files);

        // Every function and method in the corpus is a candidate change.
        let subjects: Vec<(&Fqn, Language)> = files
            .iter()
            .flat_map(|f| {
                f.symbols
                    .iter()
                    .filter(|s| matches!(s.kind, SymbolKind::Function | SymbolKind::Method))
                    .map(move |s| (&s.fqn, f.language))
            })
            .collect();

        let mut rows: BTreeMap<&str, BTreeMap<Language, usize>> = BTreeMap::new();
        let mut bump = |row: &'static str, lang: Language| {
            *rows.entry(row).or_default().entry(lang).or_insert(0) += 1;
        };
        let mut radius_total: BTreeMap<Language, usize> = BTreeMap::new();
        let mut boundary_total: BTreeMap<Language, usize> = BTreeMap::new();

        for (fqn, lang) in &subjects {
            let impact = graph.impact_of(fqn, 3);
            bump("subjects", *lang);
            *radius_total.entry(*lang).or_insert(0) += impact.reached.len();
            *boundary_total.entry(*lang).or_insert(0) += impact.boundary_total;
            if impact.reached.is_empty() && impact.boundary_total == 0 {
                bump("  no caller, no doubt", *lang);
            } else if impact.boundary_total == 0 {
                bump("  radius is exact", *lang);
            } else if impact.reached.is_empty() {
                bump("  only doubt, no placed caller", *lang);
            } else {
                bump("  radius with a stated boundary", *lang);
            }
            if impact.truncated {
                bump("  cut short at depth 3", *lang);
            }
        }

        let langs: BTreeSet<Language> = subjects.iter().map(|(_, l)| *l).collect();
        println!("\n## Blast radius, depth 3, over {} files\n", files.len());
        print!("{:38}", "");
        for lang in &langs {
            print!("{:>14}", format!("{lang:?}"));
        }
        println!();
        for (row, by_lang) in &rows {
            print!("{row:38}");
            for lang in &langs {
                print!("{:>14}", by_lang.get(lang).copied().unwrap_or(0));
            }
            println!();
        }
        print!("{:38}", "placed callers, summed");
        for lang in &langs {
            print!("{:>14}", radius_total.get(lang).copied().unwrap_or(0));
        }
        println!();
        print!("{:38}", "boundary sites, summed");
        for lang in &langs {
            print!("{:>14}", boundary_total.get(lang).copied().unwrap_or(0));
        }
        println!("\n");

        // What the doubt is MADE of, across every radius in the corpus.
        let mut reasons: BTreeMap<&'static str, usize> = BTreeMap::new();
        for (fqn, _) in &subjects {
            for (label, n) in graph.impact_of(fqn, 3).by_reason() {
                *reasons.entry(*label).or_insert(0) += n;
            }
        }
        println!("### what the boundaries are made of\n");
        let mut ranked: Vec<_> = reasons.iter().collect();
        ranked.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
        for (label, n) in ranked {
            println!("  {n:>9}  {label}");
        }

        assert!(!subjects.is_empty(), "the corpus declares no functions, so nothing was measured");
    }

    /// The property the report's numbers rest on: a placed caller is never
    /// invented. Every `reached` entry must correspond to a reference the
    /// ladder actually resolved to the subject — checked against the facts
    /// directly rather than against the index built from them.
    #[test]
    #[ignore]
    fn every_placed_caller_in_the_corpus_is_a_reference_the_ladder_resolved() {
        let corpus = crate::indexer::acceptance::read_the_corpus();
        let files: Vec<FileFacts> = corpus.into_iter().map(|r| r.facts).collect();
        let graph = Graph::of(&files);

        // An independent index, built the other way round: every (from, to)
        // pair the facts state. If `impact_of` can produce a pair that is not
        // in here, it invented an edge.
        let mut stated: BTreeSet<(&Fqn, &Fqn)> = BTreeSet::new();
        for file in &files {
            for reference in &file.references {
                if let Resolution::Resolved { fqn: to, .. } = &reference.target {
                    stated.insert((&reference.from, to));
                }
            }
        }

        let subjects: Vec<&Fqn> = files
            .iter()
            .flat_map(|f| f.symbols.iter())
            .filter(|s| matches!(s.kind, SymbolKind::Function | SymbolKind::Method))
            .map(|s| &s.fqn)
            .take(4_000)
            .collect();

        let mut checked = 0usize;
        for subject in &subjects {
            let impact = graph.impact_of(subject, 1);
            for reached in &impact.reached {
                assert!(
                    stated.contains(&(&reached.symbol, *subject)),
                    "{} was reported as a direct caller of {} but no reference says so",
                    reached.symbol,
                    subject
                );
                checked += 1;
            }
        }
        println!("{checked} direct-caller claims, every one backed by a resolved reference");
        assert!(checked > 1_000, "only {checked} claims checked — too few to mean anything");
    }

    /// A reason that POSITIVELY places a site outside is not doubt about a
    /// symbol inside.
    ///
    /// `Plumbing` is documented as "filtering, not failure": `.clone()` on an
    /// untyped receiver is plumbing, and it is not a candidate caller of a
    /// first-party `clone` however many we declare. `ExternalBoundary` is
    /// decided by "nothing we index declares this name as a member" — the
    /// ladder has already answered, and re-counting its answer as uncertainty
    /// would make a graph look least certain exactly where it is most sure.
    ///
    /// MEASURED, which is why this is a rule: before this, the two reasons
    /// contributed 10,398 of the corpus's boundary sites, headed by `map`
    /// (1,995), `into` (1,352), `as_str` (1,107) and `collect` (1,094) — every
    /// one a standard-library method that happens to share a name with
    /// something here.
    #[test]
    fn a_site_the_ladder_placed_outside_is_not_doubt_about_something_inside() {
        let files = vec![walked_rust(
            "p",
            "src/p.rs",
            r#"
pub struct W;
impl W {
    pub fn clone(&self) -> W { W }
    pub fn real(&self) {}
}
pub fn plumbing() { let x = make(); x.clone(); }
pub fn genuine() { let y = make(); y.real(); }
"#,
        )];
        let graph = Graph::of(&files);

        let filtered = graph.impact_of(&fqn_of(&files, "clone"), 3);
        assert!(
            filtered.boundary.is_empty(),
            "filtered plumbing is not a candidate caller: {:?}",
            filtered.boundary.iter().map(|b| b.reason.as_label()).collect::<Vec<_>>()
        );

        let kept = graph.impact_of(&fqn_of(&files, "real"), 3);
        assert_eq!(
            kept.boundary.iter().map(|b| b.reason.as_label()).collect::<Vec<_>>(),
            ["receiver_type_unknown"],
            "a receiver the walk could not type IS doubt, and stays"
        );
    }

    /// A name shared with half the standard library produces a boundary too
    /// large to list. Capping it is fine; capping it silently is not — a
    /// truncated list that does not say it was truncated reads as the whole
    /// population, which is the same lie the module exists to prevent.
    #[test]
    fn a_capped_boundary_reports_how_many_it_did_not_list() {
        let mut body = String::from("pub struct Z;\nimpl Z {\n    pub fn tick(&self) {}\n}\n");
        for i in 0..(MAX_BOUNDARY_SITES + 7) {
            body.push_str(&format!("pub fn user{i}() {{ let v = make(); v.tick(); }}\n"));
        }
        let files = vec![walked_rust("q", "src/q.rs", &body)];
        let graph = Graph::of(&files);
        let impact = graph.impact_of(&fqn_of(&files, "tick"), 3);

        assert_eq!(impact.boundary.len(), MAX_BOUNDARY_SITES, "the list is capped");
        assert_eq!(
            impact.boundary_total,
            MAX_BOUNDARY_SITES + 7,
            "and the COUNT is the whole population, not the length of the list"
        );
        assert!(impact.boundary_capped());
        assert_eq!(
            impact.by_reason().values().sum::<usize>(),
            impact.boundary_total,
            "the reason tally covers every site, not just the listed page — two numbers a \
             reader expects to reconcile must reconcile"
        );
    }
}
