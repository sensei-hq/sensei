//! **Two barriers a good graph should clear**, over ANY corpus.
//!
//! A class is one node; an independent function is one node. Each should appear
//! as the CALLEE end of some edge, and which end the caller sits on says a
//! different thing:
//!
//! 1. **Reached by a test.** With high coverage, a source node nothing tests is
//!    either untested or an edge the graph lost. This is the easier barrier and
//!    the one to clear first, because a test calls its subject directly and by
//!    name — the simplest edge there is.
//! 2. **Reached by other source.** A node only tests reach is exercised but not
//!    USED, which is either a genuine entry point (a task the scheduler calls, a
//!    handler a router registers, a public library surface) or a gap.
//!
//! Neither is asserted at zero. The point is the decomposition: a graph is good
//! when every miss has a name, and the names here are few and checkable.
//!
//! # Why this is a module and not a test
//!
//! It is run over TWO corpora that share no reader. This repository's own Rust
//! and TypeScript come through `acceptance::read_the_corpus`; Java comes from
//! `SENSEI_CORPUS`, because there is no Java here to measure. A second copy of
//! a measurement is not a second measurement — it is two numbers that drift
//! apart and then disagree about which language regressed. This probe has
//! already produced three classifier bugs of its own, each of which made the
//! graph look worse than it was, and each was fixed in one place only because
//! there was one place.

use std::collections::{BTreeMap, BTreeSet};

use super::facts::{
    Evidence, FileFacts, Language, Observation, ReachedBy, RelationKind, Resolution, SymbolKind,
};
use super::fqn::{self, Origin, Reach};

/// One file of a corpus: what the walk read, and the text it was read from.
///
/// The TEXT is carried rather than re-opened. The barrier needs it for the
/// inline test boundary, and a corpus that is not this repository — Java's is
/// somebody else's checkout — has no workspace root to join a relative path
/// against.
pub(super) struct Unit<'a> {
    pub(super) path: &'a str,
    pub(super) text: &'a str,
    pub(super) facts: &'a FileFacts,
}

/// What one language's source nodes did against the two barriers.
pub(super) struct Tally {
    pub(super) nodes: usize,
    pub(super) exercised: usize,
    pub(super) no_test: usize,
    pub(super) no_source: usize,
    pub(super) neither: usize,
}

/// Which declarations of a file are TESTS — the one classifier, for every
/// language and both corpora.
///
/// Two rules, because there are two conventions and a language may use both:
///
/// - the PATH. Delegated to [`crate::languages::is_test_path`], which is
///   already the single source of truth for `nodes.is_test` and already knows
///   every convention here: a `tests`/`spec`/`e2e` path segment, `*.spec.ts`,
///   Rust's sibling `tests.rs` and `*_tests.rs`, and — the one Java needs —
///   `src/test/java` plus JUnit's `*Test`/`*Tests`/`*IT` class names. Java's
///   convention is a FILE convention, so for Java the path rule is the whole
///   rule.
/// - the inline region. Rust states its tests in the same file behind
///   `#[cfg(test)]`, which no path can show, so the language is asked where
///   that region starts (see [`inline_tests_begin`]).
///
/// Getting this wrong is not a small error. Matching test modules by NAME put
/// 1,562 of them on the source side; missing whole-file test modules put 1,758
/// tests into the "nothing reaches it" bucket and is most of why Rust once read
/// as 81% untested.
fn test_boundary(path: &str, text: &str, language: Language) -> u32 {
    if crate::languages::is_test_path(path, Some(language.as_str())) {
        return 0;
    }
    inline_tests_begin(text, language).unwrap_or(u32::MAX)
}

/// The 1-based line at which a file's INLINE test region begins, for a language
/// that has one.
///
/// Rust is the only language here that does: `#[cfg(test)] mod tests` sits in
/// the same file as the code it exercises, so no path convention can find it
/// and the marker has to be read out of the text. Java's JUnit tests are
/// separate files under `src/test/java`, TypeScript's are `*.spec.ts`
/// siblings, and Python's are `test_*.py` under pytest's discovery rule — all
/// three are answered by the path, and none has an in-file marker to look for.
fn inline_tests_begin(text: &str, language: Language) -> Option<u32> {
    let marker = match language {
        Language::Rust => "#[cfg(test)]",
        Language::TypeScript | Language::Java | Language::Python => return None,
    };
    text.lines().position(|l| l.trim_start().starts_with(marker)).map(|i| i as u32 + 1)
}

/// Whether a declaration is a NODE for this measurement: something a reader
/// navigates to. A field is a property of one, and counting fields would make
/// the denominator a different question.
fn a_node(kind: SymbolKind) -> bool {
    matches!(
        kind,
        SymbolKind::Function
            | SymbolKind::Method
            | SymbolKind::Class
            | SymbolKind::Struct
            | SymbolKind::Enum
            | SymbolKind::Trait
            | SymbolKind::Interface
    )
}

/// The area a path belongs to, at the grain a person navigates: the crate or
/// app, then the directory under its source root. Deeper would make every file
/// its own area and say nothing.
fn area_of(path: &str) -> String {
    let segments: Vec<&str> = path.split('/').collect();
    let root = match segments.iter().position(|s| *s == "src") {
        Some(at) => at,
        None => return segments.first().copied().unwrap_or(path).to_string(),
    };
    segments[..=(root + 1).min(segments.len() - 1)].join("/")
}

/// One row of the by-kind report.
#[derive(Default)]
pub(super) struct Kind {
    pub(super) nodes: usize,
    pub(super) linked: usize,
    /// Called from a NON-test caller.
    pub(super) from_source: usize,
    /// Called from a test.
    pub(super) from_test: usize,
    /// Unlinked, and NO use site anywhere names it. Nothing calls it, so the
    /// graph has nothing to lose — an entry point, a registered handler, a
    /// public surface, or genuinely dead.
    pub(super) never_named: usize,
    /// Unlinked, and an unresolved use site carried this node's EXACT identity.
    /// No collision is possible; the edge was meant and it is missing.
    pub(super) lost_exact: usize,
    /// Unlinked, no identity names it, and an unresolved use site AT THIS
    /// NODE'S OWN REACH carried its bare name. A defect, at lower confidence:
    /// two declarations can share a name.
    pub(super) lost_by_name: usize,
    /// Unlinked, SOME unresolved use site of the corpus carried its bare name,
    /// and a narrowing ruled that site out — so it is not in [`Kind::lost`].
    ///
    /// **This column is what makes `lost` comparable across runs.**
    /// [`Kind::lost_by_name`] is defined by whichever narrowings
    /// [`NamedBy::verdict`] applies, and three were added during one sweep —
    /// reach, then language, then [`Via`]. Each shrank the column for a reason
    /// that has nothing to do with the resolver, and the drops were read as
    /// resolver progress because the table gave no way to tell them apart:
    /// about 79% of that sweep's TypeScript `lost` reduction and 56% of its Rust
    /// one was the definition moving underneath the number.
    ///
    /// So the table now prints both sides. A run whose `lost` fell while this
    /// rose by the same amount changed the DEFINITION; one whose `lost` fell
    /// while this held still changed the RESOLVER. Neither is legible from
    /// `lost` alone, and a number nobody can attribute is a number nobody should
    /// quote.
    pub(super) narrowed_out: usize,
}

impl Kind {
    /// Unlinked and something named it, at either grade of evidence. The
    /// defect column.
    pub(super) fn lost(&self) -> usize {
        self.lost_exact + self.lost_by_name
    }

    /// Unlinked for any reason — the defect and the nodes nothing ever named.
    pub(super) fn not_called(&self) -> usize {
        self.never_named + self.lost()
    }

    /// Fold another row in, for the TOTAL line. Every field, so a column added
    /// to the table cannot be left out of its own total.
    fn absorb(&mut self, other: &Kind) {
        self.nodes += other.nodes;
        self.linked += other.linked;
        self.from_source += other.from_source;
        self.from_test += other.from_test;
        self.never_named += other.never_named;
        self.lost_exact += other.lost_exact;
        self.lost_by_name += other.lost_by_name;
        self.narrowed_out += other.narrowed_out;
    }

    /// This row as the CALLABLE table prints it, in [`call_headings`] order.
    fn cells(&self) -> [String; 8] {
        [
            self.nodes,
            self.from_source,
            self.from_test,
            self.not_called(),
            self.lost(),
            self.lost_exact,
            self.lost_by_name,
            self.narrowed_out,
        ]
        .map(|n| n.to_string())
    }

    /// This row as the CONTAINER table prints it, in [`container_headings`]
    /// order: the COUNT, and nothing else.
    ///
    /// Every other column is dropped rather than printed as a zero, and the
    /// reach columns are the ones that matter here. A container's `from_source`
    /// and `from_test` are 0 because no walk emits an import edge, not because
    /// nothing imports these modules — every one of them is imported all over
    /// this repository. Printing that 0, under any heading, states a
    /// measurement nobody took: `not called` was the wrong word for it and
    /// `not imported` was an outright false claim. Both were the same 325 rows
    /// of noise this table exists to remove.
    ///
    /// The evidence columns go for the related reason: a container has no edge
    /// that could go missing, so a `lost` of 0 is a question never asked, and a
    /// zero in a defect column reads as one that was answered clean.
    fn container_cells(&self) -> [String; 6] {
        [
            self.nodes,
            self.from_source,
            self.from_test,
            self.not_called(),
            self.lost(),
            self.narrowed_out,
        ]
        .map(|n| n.to_string())
    }
}

/// The columns whose MEANING depends on how the kind is reached — and whether
/// the graph records enough to have any.
///
/// [`ReachedBy::Call`] has three: a reference names a callable, the walk
/// records references, so both how much reached it and how much did not are
/// measured facts.
///
/// [`ReachedBy::Import`] has NONE, and that is a statement about the GRAPH
/// rather than about modules. Nothing emits an import edge yet, so whether a
/// module was entered is not something this report can answer, and inventing a
/// column for it would put a number where a capability is missing.
fn reach_headings(reached: ReachedBy) -> &'static [&'static str] {
    match reached {
        ReachedBy::Call => &["calls", "test calls", "not called"],
        // NOW REAL. These were empty while nothing emitted an import edge, and
        // the count alone was the whole claim a container could make. An import
        // reference mints `Reach::Mod`, so "how many modules is nothing
        // importing" is a measurement rather than an assertion about a
        // capability we did not have.
        ReachedBy::Import => &["imports", "test imports", "not imported"],
    }
}

/// The callable table's columns, in order. Beside [`Kind::cells`] so a heading
/// and the number under it cannot be reordered apart, and pinned the same
/// width by `every_table_has_one_cell_per_heading`.
fn call_headings() -> [&'static str; 8] {
    let reach = reach_headings(ReachedBy::Call);
    ["nodes", reach[0], reach[1], reach[2], "lost", "exact", "by name", "narrowed"]
}

/// The container table's one column. Beside [`Kind::container_cells`], and one
/// column for the reason given there.
fn container_headings() -> [&'static str; 6] {
    let reach = reach_headings(ReachedBy::Import);
    ["nodes", reach[0], reach[1], reach[2], "lost", "narrowed"]
}

/// Every kind's row folded into one total PER WAY OF BEING REACHED.
///
/// Two totals rather than one, because a combined total has to pick a
/// vocabulary and either choice is wrong for half its input. It matters at the
/// scale this corpus is heading for: one file yields one module, so 389 Rust
/// files contribute 389 containers, and a combined `not called` would report
/// mostly uncalled modules — a number that looks like a large defect and is
/// not one.
fn totals_by_reach(kinds: &BTreeMap<SymbolKind, Kind>) -> BTreeMap<ReachedBy, Kind> {
    let mut totals: BTreeMap<ReachedBy, Kind> = BTreeMap::new();
    for (kind, row) in kinds {
        totals.entry(kind.reached_by()).or_default().absorb(row);
    }
    totals
}

/// One line of a by-kind table — the heading, a kind, or the total.
///
/// One function because the widths belong in one place. Three copies of a
/// format string is three things to keep in step, and a heading that has
/// drifted off its column is read as a different measurement. It takes a SLICE
/// so the short container row and the long callable row share those widths and
/// the two tables line up under each other.
fn a_row(label: &str, cells: &[String]) {
    const WIDTHS: [usize; 8] = [7, 8, 12, 12, 7, 7, 8, 9];
    let mut line = format!("  {label:<16}");
    for (cell, width) in cells.iter().zip(WIDTHS) {
        line.push_str(&format!(" {cell:>width$}"));
    }
    println!("{line}");
}

/// How strongly an unresolved use site points at a node no edge reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Lost {
    /// A use site carried this node's exact identity.
    Exact,
    /// No identity, but a use site at this node's reach carried its bare name.
    ByName,
    /// Nothing named it. There was no edge here to lose.
    Nothing,
}

/// What the UNRESOLVED references of a corpus can prove about a node no edge
/// reached.
///
/// Two grades of evidence, kept apart because they are not the same claim:
///
/// - an IDENTITY. A walk that cannot place a use site often records what it
///   CONSIDERED as [`Observation::Candidate`], which is a full identity. If
///   that identity is a node's own, the use site meant that node and no other,
///   and the missing edge is certain.
/// - a NAME. Otherwise all the walk has is the bare name at the use site,
///   because that is what unresolved means. A name is shared: an `is_empty`
///   the standard library declares and one this repository declares are the
///   same eight characters, so a name match can only ever be an upper bound.
///
/// The name match is narrowed by REACH, which is not a guess and costs nothing.
/// [`Evidence::reach`] is a function of the use SYNTAX — `x.name` is
/// [`Reach::Field`], `a::b::name` and `name()` are [`Reach::Item`] — and a
/// declaration's identity ends in the reach its own form minted. A use site at
/// one reach cannot produce the identity of a declaration at another, so a name
/// matched across a reach boundary is a collision by construction, never a lost
/// edge.
///
/// The LANGUAGE narrows it the same way and for the same reason, one segment
/// further left: the language is the first segment of every identity, so a
/// TypeScript use site cannot produce a Rust declaration's fqn. MEASURED: 183
/// Rust field declarations and 55 TypeScript ones were reported as lost edges
/// on the strength of a use site in the other language — `hardware.rs` declares
/// `ram_gb`, no Rust use site spells it, and the desktop app reading `ram_gb`
/// off a JSON payload was the whole of the evidence.
///
/// MEASURED over this repository: narrowing by reach drops 1,304 name matches,
/// and the drop is one-sided in a way that says what it is — 1,278 of them are
/// declarations at item reach whose name appears only at FIELD reach, i.e. a
/// member read of somebody's property that happens to spell the declaration's
/// name. It takes TypeScript's `Const` row from 1,390 names matched down to
/// 357. One worked example, the `base` the drop is named after: `app`'s
/// `senseiApi` declares a local `const base`, the only `.base` in the whole
/// corpus is `SIZE_PX.base` in a `dojo` component, and before the narrowing
/// that member read was the evidence blaming the resolver for `base`.
pub(super) struct NamedBy<'a> {
    exact: BTreeSet<&'a str>,
    /// Keyed by the LANGUAGE, by [`Reach::as_str`] and by [`Via`] — the first
    /// two labels are already written and read under everywhere else, so the
    /// two sides of the lookup cannot spell either differently.
    by_reach: BTreeMap<(Language, &'static str, Via), BTreeSet<&'a str>>,
    /// The same names with NO key at all — the pool as it stood before any of
    /// the three narrowings above existed.
    ///
    /// Kept so the table can print what the narrowings REMOVE rather than only
    /// what they leave. See [`Kind::narrowed_out`]: without it a narrowing and
    /// a resolver fix move the `lost` column identically and no reader of two
    /// runs can tell which happened.
    any_name: BTreeSet<&'a str>,
}

/// HOW a use site went looking for its target, as far as its evidence shows.
///
/// The third narrowing of the bare-name match, and the same argument as the
/// other two: a use site of one shape cannot produce the identity of a
/// declaration another shape mints. Here the shape is whether the site had a
/// RECEIVER — [`Observation::Receiver`] is recorded by the one place that turns
/// a receiver into a type, so it is a fact about the syntax and not a guess.
///
/// [`SymbolKind::can_be_reached_through_a_receiver`] is the other half and owns
/// the reasoning; this type only says which bucket a site went into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Via {
    /// `x.name` — the site asked some receiver's type for a member.
    AReceiver,
    /// A bare name, a path, or an import: no receiver was involved.
    NameAlone,
}

impl<'a> NamedBy<'a> {
    /// No evidence at all — the base a caller fills one table of, and what
    /// [`NamedBy::of`] starts from. The idiom [`TypeHomes::unknown`] already
    /// uses, here for the same reason: a literal repeated at four sites is four
    /// places to forget a table when a fourth is added.
    fn nothing() -> Self {
        Self { exact: BTreeSet::new(), by_reach: BTreeMap::new(), any_name: BTreeSet::new() }
    }

    /// Read every unresolved use site of the corpus into the two sets.
    ///
    /// Relations are read alongside references, and that symmetry is the point:
    /// a RESOLVED relation parent already counts as an edge that reached the
    /// node, so an unresolved one has to count as an attempt that failed. Left
    /// out, a type that only an `impl` header names reads as "nothing ever
    /// named it" — the one bucket that is explicitly not a defect.
    fn of(units: &'a [Unit<'a>]) -> Self {
        let mut named = Self::nothing();
        for unit in units {
            let references = unit.facts.references.iter().map(|r| &r.target);
            let relations = unit.facts.relations.iter().map(|r| &r.parent);
            for target in references.chain(relations) {
                if let Resolution::Unresolved { evidence, .. } = target {
                    named.saw(unit.facts.language, evidence);
                }
            }
        }
        named
    }

    /// One use site's evidence. An identity is recorded INSTEAD of the name,
    /// never as well: a walk that got as far as minting a candidate has said
    /// which node it meant, and letting its bare name stand too would put that
    /// use site behind every other declaration that shares the name.
    fn saw(&mut self, language: Language, evidence: &'a Evidence) {
        let mut minted = false;
        for observation in &evidence.saw {
            if let Observation::Candidate(fqn) = observation {
                self.exact.insert(fqn.as_str());
                minted = true;
            }
        }
        if !minted {
            let via = match evidence.saw.iter().any(|o| matches!(o, Observation::Receiver(_))) {
                true => Via::AReceiver,
                false => Via::NameAlone,
            };
            self.by_reach
                .entry((language, evidence.reach.as_str(), via))
                .or_default()
                .insert(evidence.name.as_str());
            self.any_name.insert(evidence.name.as_str());
        }
    }

    /// Did ANY unresolved use site of the corpus carry this bare name, whatever
    /// its language, reach or shape? The pool [`NamedBy::verdict`] narrows.
    ///
    /// Not a second verdict and never read as one — see [`Kind::narrowed_out`].
    /// It answers the question a reader of two runs needs and cannot otherwise
    /// ask: how much of the `lost` column moved because the narrowings moved.
    fn named_before_narrowing(&self, name: &str) -> bool {
        self.any_name.contains(name)
    }

    /// The grade of evidence, if any, naming one declaration.
    ///
    /// THE REACH IS THE DISCRIMINATOR, and it is exact rather than a proxy. A
    /// declaration's identity ends in the reach its own form minted and a use
    /// site's target ends in the reach its SYNTAX implies, so evidence is
    /// admissible for a declaration exactly when the two spell the same reach.
    /// A module is the one kind at [`Reach::Mod`]; a call mints [`Reach::Item`]
    /// and can therefore never be found for one, while an import mints `mod`
    /// and can.
    ///
    /// This used to short-circuit on [`SymbolKind::can_be_named`] instead, and
    /// the comment here said why: the reach narrowing reached the same zero on
    /// its own, "a coincidence of two rules agreeing", and asking the kind
    /// first stopped the zero depending on it. That held while NO use site
    /// minted `mod`. An import does, so the coincidence is over — and the
    /// short-circuit would now hide a real measurement, answering zero for a
    /// module that genuinely lost an import edge.
    ///
    /// So the kind is still asked, but for the right thing: which SHAPE of
    /// reference can reach it ([`SymbolKind::reached_by`]), which then selects
    /// the evidence a declaration of that kind is allowed to be judged on.
    fn verdict(&self, kind: SymbolKind, fqn: &str, name: &str) -> Lost {
        if self.exact.contains(fqn) {
            return Lost::Exact;
        }
        // A declaration's identity was minted by `fqn::define`, so reading it
        // back cannot fail — and if it ever does, that is a defect in the
        // identity and not a node to quietly report as unnamed.
        let parsed = fqn::parse(fqn).expect("a declaration carries an identity this can read");
        // The LANGUAGE and the REACH both come off the declaration's own
        // identity rather than from the caller, so neither can disagree with
        // the string the node is filed under.
        let Origin::Local { lang, reach } = parsed.origin else {
            // An external carries no reach (see `Form::Lib`) and declares
            // nothing here, so no use site in this corpus reaches it.
            return Lost::Nothing;
        };
        // Which SHAPES of use site could have produced this declaration's
        // identity. A kind no receiver can reach is looked up in the
        // `NameAlone` bucket only, so a member read that happens to spell it is
        // not mistaken for the edge it lost.
        let vias: &[Via] = match kind.can_be_reached_through_a_receiver() {
            true => &[Via::AReceiver, Via::NameAlone],
            false => &[Via::NameAlone],
        };
        let named = vias.iter().any(|via| {
            self.by_reach
                .get(&(lang, reach.as_str(), *via))
                .is_some_and(|names| names.contains(name))
        });
        match named {
            true => Lost::ByName,
            false => Lost::Nothing,
        }
    }
}

/// Every declared node by KIND, and for each: linked, or unlinked and why.
///
/// The split that matters is the last columns. "Nothing reaches it" alone
/// cannot tell a task the scheduler calls by string from an edge the resolver
/// dropped, and those need opposite responses — one is the code, one is us.
///
/// The signal separating them is whether any UNRESOLVED use site names the
/// node, at the two grades of evidence [`NamedBy`] describes. If something
/// tried to name it and the ladder came back empty, that is a miss. If nothing
/// names it at all, the graph never had an edge to lose.
///
/// Keyed by [`SymbolKind`] and not by its label, because the row IS about a
/// kind: stringify it here and the printer can no longer ask the kind anything,
/// which is how a kind that cannot be named ends up re-decided by whoever is
/// formatting the table.
///
/// A kind nothing can name keeps its row, and its `lost` reads zero by
/// construction — see [`NamedBy::verdict`]. Dropping the row instead would take
/// the count of those declarations out of the table, and the count is a true
/// statement worth reading: it is the only thing the table can say about them.
pub(super) fn by_kind(units: &[Unit<'_>]) -> BTreeMap<&'static str, BTreeMap<SymbolKind, Kind>> {
    let named = NamedBy::of(units);
    let mut from_source: BTreeSet<&str> = BTreeSet::new();
    let mut from_test: BTreeSet<&str> = BTreeSet::new();
    for unit in units {
        let boundary = test_boundary(unit.path, unit.text, unit.facts.language);
        for r in &unit.facts.references {
            let Resolution::Resolved { fqn, .. } = &r.target else { continue };
            if r.at.start_line >= boundary {
                from_test.insert(fqn.as_str());
            } else {
                from_source.insert(fqn.as_str());
            }
        }
        for rel in &unit.facts.relations {
            // CONTAINMENT IS NOT A REACH. A supertype or an owning type is
            // NAMED by the declaration that points at it — `impl Draw for
            // Widget` writes `Draw` — so counting it as reached from source is
            // reading what the file says. A `Contains` points the other way:
            // the parent HOLDS the child and the child's declaration names it
            // nowhere. Counting it would mark every module reached by the mere
            // existence of something inside it, which is the trivial-reach
            // noise the container split was built to remove.
            if rel.kind == RelationKind::Contains {
                continue;
            }
            if let Resolution::Resolved { fqn, .. } = &rel.parent {
                from_source.insert(fqn.as_str());
            }
        }
    }

    let mut rows: BTreeMap<&'static str, BTreeMap<SymbolKind, Kind>> = BTreeMap::new();
    for unit in units {
        let language = unit.facts.language.as_str();
        let boundary = test_boundary(unit.path, unit.text, unit.facts.language);
        for symbol in &unit.facts.symbols {
            if symbol.span.start_line >= boundary {
                continue; // a test: the harness calls it, never us
            }
            let row = rows.entry(language).or_default().entry(symbol.kind).or_default();
            row.nodes += 1;
            let by_source = from_source.contains(symbol.fqn.as_str());
            let by_test = from_test.contains(symbol.fqn.as_str());
            if by_source {
                row.from_source += 1;
            }
            if by_test {
                row.from_test += 1;
            }
            if by_source || by_test {
                row.linked += 1;
                continue;
            }
            match named.verdict(symbol.kind, symbol.fqn.as_str(), symbol.name.as_str()) {
                Lost::Exact => row.lost_exact += 1,
                Lost::ByName => row.lost_by_name += 1,
                Lost::Nothing => {
                    row.never_named += 1;
                    // A narrowing is what put it here, and the table says so.
                    // The kind check is NOT one of the three — it is a fact
                    // about the grammar, not a filter over evidence — so a kind
                    // nothing can name is left out rather than counted as
                    // something a narrowing removed.
                    if symbol.kind.can_be_named()
                        && named.named_before_narrowing(symbol.name.as_str())
                    {
                        row.narrowed_out += 1;
                    }
                }
            }
        }
    }
    rows
}

/// Run both barriers over a corpus, print the decomposition, and hand back the
/// per-language tally so a caller can assert on it.
///
/// Printing and measuring are one pass deliberately. The decomposition IS the
/// result — a count alone cannot be argued with, and every wrong diagnosis this
/// measurement has produced was a count somebody explained before they split
/// it.
pub(super) fn two_barriers(units: &[Unit<'_>]) -> BTreeMap<&'static str, Tally> {
    // Where each file's test region begins. Past it, a declaration is a test.
    // By POSITION rather than by module name: the modules in this repository
    // are called `forge_token_observe_tests`, `probe_classification` and
    // `adjacency_policy_tests`, and matching the name `tests` misclassified
    // 1,562 of them as source.
    let mut boundary: BTreeMap<&str, u32> = BTreeMap::new();
    for unit in units {
        boundary.insert(unit.path, test_boundary(unit.path, unit.text, unit.facts.language));
    }

    // Every declared identity, and which side of the line it sits on.
    let mut side: BTreeMap<&str, bool> = BTreeMap::new(); // true = test
    for unit in units {
        let at = boundary[unit.path];
        for symbol in &unit.facts.symbols {
            side.insert(symbol.fqn.as_str(), symbol.span.start_line >= at);
        }
    }

    // For each identity: is it reached from a test, and from source?
    let mut by_test: BTreeSet<&str> = BTreeSet::new();
    let mut by_source: BTreeSet<&str> = BTreeSet::new();
    for unit in units {
        let at = boundary[unit.path];
        for reference in &unit.facts.references {
            let Resolution::Resolved { fqn, .. } = &reference.target else { continue };
            // The CALLER's side. A use site at file scope belongs to the file,
            // which `side` may not hold — fall back to the line, which is the
            // conservative reading: it can only make barrier 2 look better
            // satisfied, never barrier 1.
            let from_a_test = match side.get(reference.from.as_str()) {
                Some(is_test) => *is_test,
                None => reference.at.start_line >= at,
            };
            // A node calling ITSELF proves nothing about being reached.
            if reference.from.as_str() == fqn.as_str() {
                continue;
            }
            if from_a_test {
                by_test.insert(fqn.as_str());
            } else {
                by_source.insert(fqn.as_str());
            }
        }
        for relation in &unit.facts.relations {
            if let Resolution::Resolved { fqn, .. } = &relation.parent {
                by_source.insert(fqn.as_str());
            }
        }
    }

    // TRANSITIVE reach from the tests. A test calls an entry point, which calls
    // the internals; every one of those is exercised without ever being NAMED
    // by a test. One hop measures naming, the closure measures exercise, and
    // conflating them is what makes a healthy graph look like 19% coverage.
    let mut calls_from: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for unit in units {
        for reference in &unit.facts.references {
            if let Resolution::Resolved { fqn, .. } = &reference.target
                && reference.from.as_str() != fqn.as_str()
            {
                calls_from.entry(reference.from.as_str()).or_default().push(fqn.as_str());
            }
        }
    }
    let mut exercised: BTreeSet<&str> = by_test.clone();
    let mut frontier: Vec<&str> = exercised.iter().copied().collect();
    let mut hops = 0u32;
    while !frontier.is_empty() && hops < 40 {
        let mut next = Vec::new();
        for node in frontier {
            for called in calls_from.get(node).into_iter().flatten() {
                if exercised.insert(called) {
                    next.push(*called);
                }
            }
        }
        frontier = next;
        hops += 1;
    }

    let mut per: BTreeMap<&'static str, Tally> = BTreeMap::new();
    let mut untested: Vec<String> = Vec::new();
    let mut unused: Vec<String> = Vec::new();
    // The same set, kept as facts rather than as formatted lines, because the
    // decomposition below is the point of the barrier and a `Vec<String>` can
    // only be sorted.
    let mut orphans: Vec<(&str, SymbolKind, &str, &str)> = Vec::new();
    for unit in units {
        let language = unit.facts.language.as_str();
        for symbol in unit.facts.symbols.iter().filter(|s| a_node(s.kind)) {
            if side[symbol.fqn.as_str()] {
                continue; // a test node: the harness calls it, not us
            }
            let t = per.entry(language).or_insert(Tally {
                nodes: 0,
                exercised: 0,
                no_test: 0,
                no_source: 0,
                neither: 0,
            });
            t.nodes += 1;
            let tested = by_test.contains(symbol.fqn.as_str());
            let used = by_source.contains(symbol.fqn.as_str());
            if exercised.contains(symbol.fqn.as_str()) {
                t.exercised += 1;
            }
            if !tested {
                t.no_test += 1;
                untested.push(format!("{} {}:{}", symbol.name, unit.path, symbol.span.start_line));
            }
            if !used {
                t.no_source += 1;
                unused.push(format!("{} {}:{}", symbol.name, unit.path, symbol.span.start_line));
            }
            if !tested && !used {
                t.neither += 1;
                orphans.push((language, symbol.kind, unit.path, symbol.name.as_str()));
            }
        }
    }

    println!(
        "\n  identities reached from a test: {} | from source: {}",
        by_test.len(),
        by_source.len()
    );
    let test_files = units.iter().filter(|u| boundary[u.path] == 0).count();
    let test_nodes = side.values().filter(|t| **t).count();
    println!("  test files in corpus: {test_files} | test-side declarations: {test_nodes}");
    println!("  transitively exercised from tests: {} (closure over {hops} hops)", exercised.len());
    // The table: what was identified, and who calls it.
    //
    // `not called` is the only column that needs explaining, and every entry in
    // it is listed below with a reason. `lost` inside it is a resolver defect —
    // something names it and the ladder came back empty. The target is zero,
    // and it is split by how strong the evidence is: `exact` is a use site that
    // carried this very identity, `by name` is a use site that spelled the name
    // at the right reach and could be a namesake. See [`NamedBy`].
    //
    // CONTAINERS are tabled separately, under headings that name what actually
    // reaches them. Nothing calls a module, so it can appear in neither the
    // `calls` column nor the `not called` one without the number being read as
    // a defect somebody should close.
    for (language, kinds) in &by_kind(units) {
        println!("\n## {language}\n");
        let totals = totals_by_reach(kinds);

        let section = |reached: ReachedBy, headings: &[String], short: bool| {
            let Some(total) = totals.get(&reached) else { return };
            let mut rows: Vec<_> =
                kinds.iter().filter(|(kind, _)| kind.reached_by() == reached).collect();
            rows.sort_by_key(|(_, k)| std::cmp::Reverse(k.nodes));
            a_row("kind", headings);
            for (kind, k) in rows {
                let cells = match short {
                    true => k.container_cells().to_vec(),
                    false => k.cells().to_vec(),
                };
                a_row(&format!("{kind:?}"), &cells);
            }
            let cells = match short {
                true => total.container_cells().to_vec(),
                false => total.cells().to_vec(),
            };
            a_row("TOTAL", &cells);
        };

        section(ReachedBy::Call, &call_headings().map(str::to_string), false);
        if totals.contains_key(&ReachedBy::Import) {
            println!(
                "\n  CONTAINERS — entered by an IMPORT, never called. Counted apart because \
                 `not called` is not a statement about one, and none of it is folded \
                 into the table above."
            );
            section(ReachedBy::Import, &container_headings().map(str::to_string), true);
        }
    }

    println!("\n## Two barriers, per language\n");
    println!(
        "  {:<12} {:>7} {:>12} {:>16} {:>14} {:>10}",
        "", "nodes", "no test edge", "exercised (all hops)", "no source edge", "neither"
    );
    for language in Language::all() {
        let l = language.as_str();
        let Some(t) = per.get(l) else { continue };
        if t.nodes == 0 {
            continue;
        }
        let pct = |n: usize| 100.0 * n as f64 / t.nodes as f64;
        println!(
            "  {l:<12} {:>7} {:>7} {:>4.0}% {:>11} {:>4.0}% {:>9} {:>4.0}% {:>5} {:>4.0}%",
            t.nodes,
            t.no_test,
            pct(t.no_test),
            t.exercised,
            pct(t.exercised),
            t.no_source,
            pct(t.no_source),
            t.neither,
            pct(t.neither)
        );
    }

    let head = |what: &str, mut list: Vec<String>| {
        list.sort();
        println!("\n  {what}, first 25 of {}:", list.len());
        for line in list.iter().take(25) {
            println!("    {line}");
        }
    };
    head("BARRIER 1 — no test reaches it", untested);
    head("BARRIER 2 — no source reaches it", unused);

    decompose(units, &orphans);
    per
}

/// What is LEFT in NEITHER, split three ways.
///
/// The goal is not a smaller number, it is a number every entry of which has a
/// name. "Named by nothing" is separated from "named and not placed" because
/// they are different work: the first is a node with genuinely no caller — an
/// entry point, or dead code — and the second is an edge the resolver lost.
fn decompose(units: &[Unit<'_>], orphans: &[(&str, SymbolKind, &str, &str)]) {
    let mut named_by: BTreeMap<&str, BTreeMap<String, usize>> = BTreeMap::new();
    for unit in units {
        for reference in &unit.facts.references {
            if let Resolution::Unresolved { reason, evidence } = &reference.target {
                *named_by
                    .entry(evidence.name.as_str())
                    .or_default()
                    .entry(format!("{reason:?}"))
                    .or_default() += 1;
            }
        }
    }

    for language in Language::all() {
        let l = language.as_str();
        let mine: Vec<&(&str, SymbolKind, &str, &str)> =
            orphans.iter().filter(|(lang, ..)| *lang == l).collect();
        if mine.is_empty() {
            continue;
        }
        let mut by_kind: BTreeMap<String, usize> = BTreeMap::new();
        let mut by_area: BTreeMap<String, usize> = BTreeMap::new();
        let mut by_why: BTreeMap<&str, usize> = BTreeMap::new();
        // The one bucket the goal is actually about: a node no UNRESOLVED use
        // site names, so there is no miss to go and chase. Sampled rather than
        // counted, because the readings left need a person and there are three
        // of them, all three VERIFIED against the source:
        //
        // - a registered entry point — `tasks::handlers::advance_run` is
        //   reached through the task table in `tasks/mod.rs`, which names it as
        //   a string, so no call edge exists to find;
        // - reached only from INSIDE a macro invocation — `doctor::blue` is
        //   called once, as an argument to `println!`. The walk emits no
        //   reference from inside a macro at all (see `Reason::MacroExpansion`),
        //   so the use site is not a miss either — it does not exist;
        // - genuinely dead.
        //
        // A count here is therefore an upper bound on dead code and nothing
        // more, which is why it is printed as a sample beside its reading.
        let mut nothing_names_it: Vec<String> = Vec::new();
        for (_, kind, path, name) in &mine {
            *by_kind.entry(format!("{kind:?}")).or_default() += 1;
            *by_area.entry(area_of(path)).or_default() += 1;
            let why = match named_by.get(name) {
                None => "named by no use site at all",
                Some(reasons) => reasons
                    .iter()
                    .max_by_key(|(reason, n)| (**n, std::cmp::Reverse((*reason).clone())))
                    .map(|(reason, _)| match reason.as_str() {
                        "ReceiverTypeUnknown" => "named, receiver untyped",
                        "ExternalBoundary" => "named, read as outside",
                        "NoImportInScope" => "named, nothing binds it",
                        "AmbiguousCandidates" => "named, two candidates",
                        "Plumbing" => "named, filtered as plumbing",
                        _ => "named, other reason",
                    })
                    .unwrap_or("named by no use site at all"),
            };
            *by_why.entry(why).or_default() += 1;
            if !named_by.contains_key(name) {
                nothing_names_it.push(format!("{name:<40} {path}"));
            }
        }
        let ranked = |map: &BTreeMap<String, usize>, take: usize| -> Vec<String> {
            let mut v: Vec<(&String, &usize)> = map.iter().collect();
            v.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
            v.into_iter().take(take).map(|(k, n)| format!("{n} {k}")).collect()
        };
        println!("\n## What is left in NEITHER — {l}, {} nodes\n", mine.len());
        println!("  by kind: {}", ranked(&by_kind, 8).join(" | "));
        println!("  by area: {}", ranked(&by_area, 12).join(" | "));
        let mut why: Vec<(&&str, &usize)> = by_why.iter().collect();
        why.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        for (what, n) in why {
            println!("  {n:>6}  {what}");
        }
        let mut sample: Vec<String> =
            mine.iter().map(|(_, _, path, name)| format!("{name:<44} {path}")).collect();
        sample.sort();
        println!("  NEITHER — first 25 of {}:", sample.len());
        for line in sample.iter().take(25) {
            println!("      {line}");
        }
        nothing_names_it.sort();
        println!("  nothing names it — first 15 of {}:", nothing_names_it.len());
        for line in nothing_names_it.iter().take(15) {
            println!("      {line}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexer::facts::Language;
    use crate::indexer::fqn;
    use crate::indexer::lang::{self, Source, TypeHomes};
    use crate::indexer::resolve::{World, members_declared_by, resolve};

    /// Walk and place a handful of in-memory files, the same two-pass way a
    /// corpus is read: every file walked once with no type table so the
    /// declarations can be collected, then again with it, then the ladder (R6).
    fn placed(files: &[(&str, &str)]) -> Vec<FileFacts> {
        let read_all = |types: &TypeHomes| -> Vec<FileFacts> {
            files
                .iter()
                .map(|(path, text)| {
                    let ext = format!(".{}", path.rsplit('.').next().unwrap_or(""));
                    let adapter = lang::adapter_for_ext(&ext)
                        .unwrap_or_else(|| panic!("no adapter reads {path}"));
                    // The module is DERIVED, the way the corpus reader derives
                    // it, and not left empty. A TypeScript file with no module
                    // path has no identity at all, so an empty one made the
                    // reader hand back an error the old `filter_map` swallowed —
                    // and `report_over` zips this list against `files`, so a
                    // silently dropped file pairs every later file's text with
                    // the wrong facts.
                    let module = adapter
                        .module_path(path, crate::indexer::acceptance::package_root_of(path));
                    let source = Source { package: "unnamed", module: &module, path, text };
                    adapter
                        .read(&source, types)
                        .unwrap_or_else(|e| panic!("{path} could not be read: {e:?}"))
                })
                .collect()
        };
        let first = read_all(&TypeHomes::unknown());
        let homes = TypeHomes::of(
            first.iter().flat_map(|f| f.symbols.iter().map(|s| (f.package.as_str(), s))),
        );
        let anchored = read_all(&homes);
        let first_party: BTreeSet<String> = anchored.iter().map(|f| f.package.clone()).collect();
        let first_party_members = crate::indexer::resolve::member_names_of(anchored.iter());
        let declared_members = members_declared_by(anchored.iter());
        let returns = crate::indexer::resolve::returns_declared_by(anchored.iter());
        let scanned = BTreeSet::new();
        let world = World {
            first_party: &first_party,
            first_party_members: &first_party_members,
            declared_members: &declared_members,
            returns: &returns,
            scanned: &scanned,
        };
        anchored
            .into_iter()
            .map(|facts| {
                let grammar = lang::adapter_for(facts.language).grammar();
                resolve(facts, grammar, &world)
            })
            .collect()
    }

    /// Walk and place a handful of files, then read the by-kind report off
    /// them. The two report tests below differ only in their fixture.
    fn report_over(files: &[(&str, &str)]) -> BTreeMap<&'static str, BTreeMap<SymbolKind, Kind>> {
        let corpus = placed(files);
        let units: Vec<Unit<'_>> = corpus
            .iter()
            .zip(files.iter())
            .map(|(facts, (path, text))| Unit { path, text, facts })
            .collect();
        // The map borrows the units, so the rows have to be lifted out of the
        // borrow before the corpus goes out of scope at the end of this call.
        by_kind(&units)
    }

    /// A MODULE is a CONTAINER: nothing calls it, it is IMPORTED, and the
    /// things inside it are what get called. It keeps its row, because how
    /// many were declared is a true statement and the only one available; what
    /// it cannot have is a lost edge.
    ///
    /// The fixture is the exact shape that made the old report wrong: a module
    /// named `inner`, and — in the same corpus — a member read that also
    /// spells `inner` and cannot be placed. A bare-name match reads those two
    /// as the same name and reports the module as an edge somebody lost. No
    /// edge was ever possible: a module is entered by an import, which is not
    /// a reference at all.
    #[test]
    fn a_module_keeps_its_row_and_has_no_edge_to_lose() {
        let rows = report_over(&[(
            "crates/x/src/lib.rs",
            "pub mod inner {\n\
             \x20   pub fn f() {}\n\
             }\n\
             pub fn g(v: Outside) -> usize { v.inner }\n",
        )]);

        let rust = rows.get("rust").expect("the rust fixture produced rows");
        let module = rust.get(&SymbolKind::Module).expect("the module is counted, in the table");
        assert_eq!(
            (module.nodes, module.from_source, module.from_test),
            (2, 0, 0),
            "TWO containers now: the inline `mod inner`, and the FILE, which is \
             itself a module and now says so"
        );
        assert_eq!(
            (module.lost(), module.never_named),
            (0, 2),
            "and `lost` is 0: an unplaceable `v.inner` is a member read, not a module entry"
        );
    }

    /// A CONTAINER IS MEASURED UNDER THE WORD THAT REACHES IT — `imports`, and
    /// never `calls`.
    ///
    /// `not imported` was the second wrong answer here and worse than the
    /// first. `not called` was merely the wrong word; `not imported: 325`
    /// is a false CLAIM, because every one of those modules is imported all
    /// over this repository. What is missing is not the imports, it is the
    /// EDGES: nothing emits an import edge yet, so the graph cannot say
    /// whether a module was entered, and a column of 325 asserts that it can.
    ///
    /// That is the same shape as the rule against fabricating a value on a
    /// failure path: an honest zero is only honest when the thing genuinely is
    /// zero, never when it stands in for a measurement nobody took. And the
    /// point of the exercise was to REMOVE a misleading 325, not to reword it.
    ///
    /// So a container's row is its count, which the container test above
    /// already calls the one true statement available. When `RefKind::Imports`
    /// lands, that step adds the columns along with the edges that fill them.
    #[test]
    fn a_container_is_counted_and_nothing_further_is_claimed_about_it() {
        assert_eq!(
            reach_headings(ReachedBy::Import),
            ["imports", "test imports", "not imported"],
            "an import mints `mod`, so what entered a module is a measurement the table takes"
        );
        assert_eq!(
            reach_headings(ReachedBy::Call),
            ["calls", "test calls", "not called"],
            "a callable is reached by references the graph DOES record, so its reach is reportable"
        );
        assert_eq!(
            container_headings(),
            ["nodes", "imports", "test imports", "not imported", "lost", "narrowed"],
            "a container is counted AND measured, under the word that reaches it"
        );
    }

    /// A heading and the number under it cannot drift apart in COUNT either.
    ///
    /// The guard for the whole two-table arrangement: adding a column to the
    /// headings and forgetting the cells (or the reverse) prints a table whose
    /// numbers sit under the wrong words, which is exactly the class of defect
    /// this section was rewritten twice to remove.
    #[test]
    fn every_table_has_one_cell_per_heading() {
        let row = Kind::default();
        assert_eq!(
            call_headings().len(),
            row.cells().len(),
            "the callable table's headings and cells must be the same width"
        );
        assert_eq!(
            container_headings().len(),
            row.container_cells().len(),
            "and the container table's"
        );
    }

    /// The two totals do not fold together, and THIS is the claim that matters
    /// once one file yields one module: a corpus of 389 Rust files contributes
    /// 389 containers, and a single `not called` total carrying them reports
    /// the overwhelming majority of its own column as uncalled modules.
    ///
    /// The fixture is the smallest corpus with both: one module (a container)
    /// and one function nothing calls (a callable). Folded, `not called` reads
    /// 2 and says nothing true about either.
    #[test]
    fn a_container_is_totalled_apart_from_what_can_be_called() {
        let rows = report_over(&[(
            "crates/x/src/lib.rs",
            "pub mod inner {\n\
             \x20   pub fn f() {}\n\
             }\n\
             pub fn never_called() {}\n",
        )]);
        let rust = rows.get("rust").expect("the rust fixture produced rows");
        let totals = totals_by_reach(rust);

        let callable = totals.get(&ReachedBy::Call).expect("the fixture declares callable kinds");
        let container = totals.get(&ReachedBy::Import).expect("and one container");

        assert_eq!(
            (callable.nodes, callable.not_called()),
            (2, 2),
            "`never_called` and the module's own `f` are callable and uncalled; the module is \
             not in this total at all"
        );
        assert_eq!(
            container.container_cells()[0],
            "2",
            "both containers are counted: the inline `mod inner` and the FILE"
        );
        assert_eq!(
            container.lost(),
            0,
            "and a container still has no edge to lose — the split does not invent one"
        );
    }

    /// **A MODULE IS JUDGED ON IMPORT EVIDENCE, AND NEVER ON A CALL.**
    ///
    /// This replaces a test that asserted the opposite — that a container was
    /// answered zero BEFORE any evidence was read. That was right while no use
    /// site minted [`Reach::Mod`]: the kind short-circuit and the reach
    /// narrowing agreed, and asking the kind first meant the zero did not
    /// depend on the agreement. An import mints `mod`, so the agreement is
    /// over, and the short-circuit would now answer zero for a module that
    /// genuinely lost an import edge — a number nobody measured.
    ///
    /// Both directions, because one alone proves nothing. Evidence at `mod`
    /// reach must now REACH a module; evidence at `item` reach must still not,
    /// and the same item-reach evidence must still be read normally for a kind
    /// a call can name. Built by hand at the by-name grade rather than the
    /// exact one, because `exact` answers before the reach is consulted and
    /// would pass whatever the narrowing did.
    #[test]
    fn a_module_is_judged_on_import_evidence_and_never_on_a_call() {
        let identity = |reach| {
            fqn::define(&fqn::Form::Item {
                lang: Language::Rust,
                package: "x",
                module: "outer",
                name: "inner",
                reach,
            })
            .expect("an identity")
        };
        let a_module = identity(Reach::Mod);
        let a_function = identity(Reach::Item);
        let evidence_at = |reach: Reach| NamedBy {
            by_reach: BTreeMap::from([(
                (Language::Rust, reach.as_str(), Via::NameAlone),
                BTreeSet::from(["inner"]),
            )]),
            ..NamedBy::nothing()
        };

        assert_eq!(
            evidence_at(Reach::Mod).verdict(SymbolKind::Module, a_module.as_str(), "inner"),
            Lost::ByName,
            "an import mints `mod`, so a module CAN lose an edge and the report must say so"
        );
        assert_eq!(
            evidence_at(Reach::Item).verdict(SymbolKind::Module, a_module.as_str(), "inner"),
            Lost::Nothing,
            "a call mints `item` and can never reach a module: `a::b::c()` names `c`, never `a`"
        );
        assert_eq!(
            evidence_at(Reach::Item).verdict(SymbolKind::Function, a_function.as_str(), "inner"),
            Lost::ByName,
            "and the same item-reach evidence about a kind a call CAN name is read normally"
        );
    }

    /// A name is shared; a REACH is not. `x.TOTAL` and `TOTAL` are two
    /// different questions, and only one of them can ever reach a const.
    ///
    /// Both halves in one fixture, because the failure mode is a rule that
    /// answers one of them and not the other: the unplaceable member read
    /// `v.TOTAL` is evidence about the FIELD named `TOTAL` and is no evidence
    /// at all about the CONST named `TOTAL`.
    #[test]
    fn a_name_that_collides_across_a_reach_is_not_a_lost_edge() {
        let rows = report_over(&[(
            "crates/x/src/lib.rs",
            "pub const TOTAL: usize = 1;\n\
             pub struct S {\n\
             \x20   pub TOTAL: usize,\n\
             }\n\
             pub fn peek(v: Outside) -> usize { v.TOTAL }\n",
        )]);
        let rust = rows.get("rust").expect("the rust fixture produced rows");

        let field = rust.get(&SymbolKind::Field).expect("the struct declares a field");
        assert_eq!(
            (field.lost_exact, field.lost_by_name),
            (0, 1),
            "a member read at field reach names the field, by name and not by identity"
        );

        let konst = rust.get(&SymbolKind::Const).expect("the file declares a const");
        assert_eq!(
            (konst.lost_exact, konst.lost_by_name, konst.never_named),
            (0, 0, 1),
            "and the same four characters at field reach are no evidence about a const"
        );
    }

    /// A name is shared across LANGUAGES as well, and THAT match is impossible
    /// by construction rather than merely unlikely.
    ///
    /// The language is the FIRST SEGMENT of every identity, so a TypeScript use
    /// site cannot produce a Rust declaration's fqn — the same argument the
    /// reach narrowing makes, one segment further left, and costing just as
    /// little. Without it the bare-name set is one pool that every language
    /// pours into and every language drinks from.
    ///
    /// MEASURED over this repository: 183 Rust field declarations and 55
    /// TypeScript ones were reported as lost edges on the strength of a use site
    /// in the OTHER language. `crates/bootstrap/src/hardware.rs:37` declares
    /// `ram_gb` and no unresolved Rust use site spells it; the desktop app reads
    /// `ram_gb` off a JSON payload, and that read was the evidence.
    ///
    /// MUTATION: drop the language from the `by_reach` key and the Rust field
    /// below is reported lost again.
    #[test]
    fn a_name_that_collides_across_a_language_is_not_a_lost_edge() {
        let rows = report_over(&[
            ("crates/x/src/lib.rs", "pub struct Hw {\n\x20   pub ram_gb: usize,\n}\n"),
            // The receiver is deliberately UNTYPED. A typed one mints a
            // candidate, and a use site that minted an identity does not also
            // lend its bare name — so a fixture written that way would pass
            // with the pooling still in place.
            ("app/src/lib/hw.ts", "export function peek(v): number { return v.ram_gb; }\n"),
        ]);

        let rust = rows.get("rust").expect("the rust fixture produced rows");
        let field = rust.get(&SymbolKind::Field).expect("the struct declares a field");
        assert_eq!(
            (field.lost(), field.never_named),
            (0, 1),
            "the only `ram_gb` anybody reads is in another language, which can never mint a \
             rust identity"
        );
        // And the reading really did happen, so the fixture is not passing by
        // having produced no evidence at all.
        let ts = rows.get("typescript").expect("the typescript fixture produced rows");
        assert_eq!(
            ts.get(&SymbolKind::Function).map(|f| f.nodes),
            Some(1),
            "the typescript file was read; a fixture that produced nothing would pass vacuously"
        );
    }

    /// **What the narrowings TOOK OUT of the lost column, in the table.**
    ///
    /// The three narrowings above are each correct and each shrinks `lost` for
    /// a reason that is not the resolver getting better. Added one at a time
    /// during a single sweep, they moved the column under the numbers being
    /// quoted beside it: about 79% of that sweep's TypeScript `lost` reduction
    /// and 56% of its Rust one was the definition changing, and nothing in the
    /// table said so. A reader comparing two runs could not have known.
    ///
    /// The same fixture as the language narrowing, read the other way round:
    /// `ram_gb` is not lost, AND the reason it is not is a narrowing rather than
    /// an absence of evidence. Both halves in one assertion, because either
    /// alone is the ambiguity this column exists to remove.
    ///
    /// MUTATION: drop the `any_name` insert from [`NamedBy::saw`]. The column
    /// reads 0 and the run is indistinguishable from one where nothing ever
    /// spelled `ram_gb`.
    #[test]
    fn the_table_says_how_many_nodes_a_narrowing_took_out_of_the_lost_column() {
        let rows = report_over(&[
            ("crates/x/src/lib.rs", "pub struct Hw {\n\x20   pub ram_gb: usize,\n}\n"),
            ("app/src/lib/hw.ts", "export function peek(v): number { return v.ram_gb; }\n"),
        ]);
        let rust = rows.get("rust").expect("the rust fixture produced rows");
        let field = rust.get(&SymbolKind::Field).expect("the struct declares a field");
        assert_eq!(
            (field.lost(), field.narrowed_out),
            (0, 1),
            "the field is not lost, and it is not lost BECAUSE a narrowing ruled the only site \
             that spells it out — a run cannot be compared with one that narrowed differently \
             unless the table says which"
        );

        // A declaration nothing anywhere names is NOT in this column, so it
        // cannot be passing by counting every unlinked node.
        let function = rows
            .get("typescript")
            .and_then(|ts| ts.get(&SymbolKind::Function))
            .expect("the typescript fixture declares a function");
        assert_eq!(
            (function.lost(), function.narrowed_out),
            (0, 0),
            "nobody calls `peek` and nobody spells it either; there is no narrowing to report"
        );
    }

    /// A name reached THROUGH A RECEIVER is no evidence about a binding, for
    /// the same reason the reach and the language are not: a use site of that
    /// shape could not have produced the declaration's identity.
    ///
    /// `xs.slice(...)` asks the type of `xs` for a member. A `const` is a
    /// member of nothing — it is reached by its bare name, or through an
    /// import, and never through a dot. The one dotted spelling that does
    /// reach a module's exports is a NAMESPACE import, and `member_of` already
    /// refuses to treat that as a receiver at all, so it never lends a name
    /// here.
    ///
    /// MEASURED over this repository: every one of the 145 TypeScript `Const`,
    /// 22 `Static` and 141 `Function` declarations reported lost was named
    /// ONLY by receiver-carried sites — 4,241, 1,242 and 6,410 of them
    /// respectively, and not one site of any other shape. The names say it
    /// plainly: `arr.map`, `arr.filter`, `arr.slice`, `Date.now`,
    /// `console.error`. Rust's 8 `Function` losses are the same collision —
    /// a free `fn walk` "named" by 35 `x.walk` field reads.
    #[test]
    fn a_name_reached_through_a_receiver_is_not_a_lost_edge_for_a_binding() {
        let rows = report_over(&[
            (
                "app/src/lib/limits.ts",
                "const slice = 3;\nexport function cap(): number {\n\x20   return slice;\n}\n",
            ),
            // UNTYPED receiver, for the reason the sibling above records: a
            // typed one mints a candidate, and a use site that minted an
            // identity does not also lend its bare name.
            (
                "app/src/lib/rows.ts",
                "export function firstTwo(xs) {\n\x20   return xs.slice(0, 2);\n}\n",
            ),
        ]);

        let ts = rows.get("typescript").expect("the typescript fixture produced rows");
        let konst = ts.get(&SymbolKind::Const).expect("the fixture declares a const");
        assert_eq!(
            (konst.lost(), konst.never_named),
            (0, 1),
            "the only `slice` anybody reaches is a member of some receiver's type, and a const \
             is a member of no type"
        );
    }

    /// And the same narrowing must NOT reach a member, which is exactly what a
    /// receiver-carried site does name.
    ///
    /// The sibling above and this one are one rule read from both sides. Widen
    /// it to every kind and this fixture's field silently stops being reported;
    /// the field row is the whole of the column being driven to zero, so a
    /// narrowing that swallowed it would read as progress.
    #[test]
    fn a_name_reached_through_a_receiver_is_still_a_lost_edge_for_a_member() {
        let rows = report_over(&[
            ("app/src/lib/row.ts", "export class Row {\n\x20   slice = 3;\n}\n"),
            ("app/src/lib/peek.ts", "export function peek(xs) {\n\x20   return xs.slice;\n}\n"),
        ]);

        let ts = rows.get("typescript").expect("the typescript fixture produced rows");
        let field = ts.get(&SymbolKind::Field).expect("the class declares a field");
        assert_eq!(
            (field.lost(), field.never_named),
            (1, 0),
            "a field IS reached through a receiver, so a receiver-carried name is the very \
             evidence that its edge went missing"
        );
    }

    /// A header that names a supertype is a use site, and for one language it
    /// is the ONLY record of that use site.
    ///
    /// A resolved relation parent already counts as an edge that reached the
    /// type — the linked column is built from relations as well as references —
    /// so an unresolved one has to count as an attempt that failed. The fixture
    /// is Java rather than Rust, and that is the whole point of it: `impl Draw
    /// for W` emits a `TypeUse` REFERENCE beside its `TraitImpl` relation, so a
    /// Rust fixture passes whether relation parents are read or not and proves
    /// nothing. MEASURED: dropping relation parents leaves this repository's
    /// own Rust and TypeScript table byte-identical. Java's `implements Draw`
    /// emits the relation alone, so here the read is the only thing standing
    /// between `Draw` and the bucket that says nobody named it.
    #[test]
    fn a_header_naming_a_supertype_is_a_use_site_even_with_no_reference_beside_it() {
        // A MARKER interface, with no members of its own. One that declares a
        // member owns it, and an `Owns` relation's parent resolves by
        // construction — so the type would be linked whatever this evidence
        // scan does, and the fixture would measure nothing.
        let rows = report_over(&[
            ("server/src/main/java/com/x/Draw.java", "package com.x;\npublic interface Draw {}\n"),
            (
                "server/src/main/java/com/y/W.java",
                "package com.y;\npublic class W implements Draw {}\n",
            ),
        ]);
        let java = rows.get("java").expect("the java fixture produced rows");
        let interface = java.get(&SymbolKind::Interface).expect("the fixture declares one");
        assert_eq!(
            (interface.linked, interface.lost_exact, interface.lost_by_name, interface.never_named),
            (0, 0, 1, 0),
            "the implements header named `Draw` and the ladder came back empty, which is a \
             lost edge and not a type nobody names"
        );
    }

    /// The two grades of evidence, and which one wins.
    ///
    /// An [`Observation::Candidate`] is a full identity the walk considered, so
    /// a node it matches was MEANT — there is no collision to argue about. A
    /// bare name is shared by construction. When both are available the
    /// identity answers, because counting a node in both columns would make
    /// `lost` a sum of two overlapping sets.
    #[test]
    fn an_identity_outranks_a_name_as_evidence_that_an_edge_was_lost() {
        let widget = fqn::define(&fqn::Form::Member {
            lang: Language::Rust,
            package: "x",
            module: "widget",
            ty: "Widget",
            member: "draw",
            reach: Reach::Item,
        })
        .expect("a member identity");
        let gadget = fqn::define(&fqn::Form::Member {
            lang: Language::Rust,
            package: "x",
            module: "gadget",
            ty: "Gadget",
            member: "draw",
            reach: Reach::Item,
        })
        .expect("a member identity");

        let named = NamedBy {
            exact: BTreeSet::from([widget.as_str()]),
            // THROUGH A RECEIVER, because `widget.draw()` is: a method is
            // exactly the kind such a site can name, so the narrowing must
            // leave the name match below standing.
            by_reach: BTreeMap::from([(
                (Language::Rust, Reach::Item.as_str(), Via::AReceiver),
                BTreeSet::from(["draw"]),
            )]),
            ..NamedBy::nothing()
        };

        assert_eq!(
            named.verdict(SymbolKind::Method, widget.as_str(), "draw"),
            Lost::Exact,
            "the use site carried this very identity, so nothing is being guessed"
        );
        assert_eq!(
            named.verdict(SymbolKind::Method, gadget.as_str(), "draw"),
            Lost::ByName,
            "the same name on another type is a name match and is reported as one"
        );
        assert_eq!(
            named.verdict(SymbolKind::Method, gadget.as_str(), "resize"),
            Lost::Nothing,
            "and a name no use site spells at this reach is not evidence of anything"
        );
    }

    /// A use site that MINTED an identity does not also lend its bare name.
    ///
    /// The node it protects is not the one the candidate names — `verdict`
    /// reads `exact` first, so that node is reported as exact either way. It is
    /// every NAMESAKE. Let the name stand as well and one unplaceable
    /// `widget.draw()` becomes evidence against every other `draw` declared in
    /// the corpus, which is how an upper bound stops being worth reading.
    #[test]
    fn a_use_site_that_minted_an_identity_does_not_also_lend_its_bare_name() {
        let widget = fqn::define(&fqn::Form::Member {
            lang: Language::Rust,
            package: "x",
            module: "widget",
            ty: "Widget",
            member: "draw",
            reach: Reach::Item,
        })
        .expect("a member identity");
        let considered = Evidence {
            name: "draw".to_string(),
            node_kind: "call_expression".to_string(),
            reach: Reach::Item,
            saw: vec![Observation::Candidate(widget.clone())],
        };
        let named_only = Evidence {
            name: "resize".to_string(),
            node_kind: "call_expression".to_string(),
            reach: Reach::Item,
            saw: vec![],
        };

        let mut named = NamedBy::nothing();
        named.saw(Language::Rust, &considered);
        named.saw(Language::Rust, &named_only);

        assert_eq!(
            named.exact,
            BTreeSet::from([widget.as_str()]),
            "the walk said which node it meant, and that is what is recorded"
        );
        assert_eq!(
            named.by_reach.get(&(Language::Rust, Reach::Item.as_str(), Via::NameAlone)),
            Some(&BTreeSet::from(["resize"])),
            "only the use site with no identity of its own falls back to its bare name"
        );
    }

    /// END TO END, through the real Java adapter and the real ladder: a JUnit
    /// test in the Maven test tree reaches the class it exercises, and the
    /// barrier counts that as a test edge.
    ///
    /// The fixture-level classifier test above proves the RULE; this proves the
    /// rule is the one the barrier applies. They are different failures — the
    /// barrier could know Java's convention and still tally the test's own
    /// declarations as source nodes needing coverage, which is what makes a
    /// corpus of 871 JUnit files read as 100% untested.
    ///
    /// The JUnit file sits in a DIFFERENT package and imports its subject, and
    /// that is deliberate rather than convenient: Maven's own convention puts a
    /// test in the SAME package as the class it exercises, where Java requires
    /// no import — and the ladder has no rung that places a bare name against a
    /// sibling file of one package, so those edges are misses for a reason that
    /// has nothing to do with this classifier. Measuring the classifier through
    /// that gap would measure the gap. What the gap costs the real corpus is
    /// reported by the corpus run, not hidden here.
    #[test]
    fn a_junit_test_reaches_the_production_class_through_the_barrier() {
        let files = [
            (
                "server/src/main/java/com/x/Greeter.java",
                "package com.x;\n\
                 public class Greeter {\n\
                 \x20 public static String greet() { return \"hi\"; }\n\
                 }\n",
            ),
            (
                "server/src/test/java/com/y/GreeterTest.java",
                "package com.y;\n\
                 import com.x.Greeter;\n\
                 public class GreeterTest {\n\
                 \x20 public void greets() { Greeter subject = null; Greeter.greet(); }\n\
                 }\n",
            ),
        ];
        let corpus = placed(&files);
        let units: Vec<Unit<'_>> = corpus
            .iter()
            .zip(files.iter())
            .map(|(facts, (path, text))| Unit { path, text, facts })
            .collect();

        let per = two_barriers(&units);
        let java = per.get("java").expect("the java corpus produced source nodes");
        // `Greeter` and `greet`, and only those: `GreeterTest` and `greets` sit
        // on the test side and are not nodes anybody has to cover.
        assert_eq!(java.nodes, 2, "the JUnit class's own declarations are not source nodes");
        assert_eq!(
            java.no_test, 0,
            "both the class and its method are named by the JUnit file, so both are reached \
             by a test"
        );
    }

    /// Java states its tests in a SEPARATE FILE — `src/test/java`, a `*Test`
    /// class name, `@Test` on the method — and never in a `#[cfg(test)]` region
    /// of the file under test. A classifier that only knows Rust's convention
    /// puts every JUnit test on the SOURCE side, where it stops counting as a
    /// test edge and starts counting as a node that needs one: both barriers
    /// move, in opposite directions, and the corpus reads as untested.
    #[test]
    fn a_junit_test_is_a_test_and_the_class_it_exercises_is_not() {
        let production = "server/src/main/java/com/x/Greeter.java";
        let junit = "server/src/test/java/com/x/GreeterTest.java";
        // A JUnit class in the MAIN tree, which Maven allows and which only the
        // class-name convention catches.
        let misplaced = "server/src/main/java/com/x/PaymentIT.java";

        assert_eq!(
            test_boundary(production, "package com.x;\n", Language::Java),
            u32::MAX,
            "a production class under src/main/java has no test region at all"
        );
        assert_eq!(
            test_boundary(junit, "package com.x;\n", Language::Java),
            0,
            "every declaration of a file under src/test/java is a test"
        );
        assert_eq!(
            test_boundary(misplaced, "package com.x;\n", Language::Java),
            0,
            "a JUnit class name is the convention even outside src/test/java"
        );
        // And the Rust marker is NOT Java's. A Java file that happens to carry
        // the characters must not be cut in half by a rule from another
        // language.
        assert_eq!(
            inline_tests_begin("class A { String s = \"#[cfg(test)]\"; }", Language::Java),
            None,
            "Java has no inline test region, so nothing in its text starts one"
        );
    }

    /// Rust's convention, which the path cannot see. Both halves: the inline
    /// region, and the sibling file whose whole contents are tests.
    #[test]
    fn rust_states_its_tests_inline_and_in_a_sibling_file() {
        let inline = "pub fn a() {}\n#[cfg(test)]\nmod tests {\n    fn t() {}\n}\n";
        assert_eq!(
            test_boundary("crates/x/src/thing.rs", inline, Language::Rust),
            2,
            "the test region starts at the line the marker sits on"
        );
        assert_eq!(
            test_boundary("crates/x/src/thing.rs", "pub fn a() {}\n", Language::Rust),
            u32::MAX,
            "a file with no marker declares no tests"
        );
        assert_eq!(
            test_boundary("crates/x/src/db/tests.rs", "fn t() {}\n", Language::Rust),
            0,
            "the sibling test module carries no marker of its own — the \
             attribute sits on the `mod tests;` in the parent"
        );
        assert_eq!(
            test_boundary("crates/x/src/db/run_tests.rs", "fn t() {}\n", Language::Rust),
            0,
            "and the same idiom with a prefix"
        );
    }

    /// TypeScript's is the path and only the path.
    #[test]
    fn typescript_states_its_tests_in_a_sibling_spec_file() {
        assert_eq!(
            test_boundary("app/src/lib/buckets.spec.ts", "", Language::TypeScript),
            0,
            "a spec sibling is all tests"
        );
        assert_eq!(
            test_boundary("app/src/lib/buckets.ts", "", Language::TypeScript),
            u32::MAX,
            "and its subject is none"
        );
    }
}
