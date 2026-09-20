//! Acceptance — §6's criteria and R8, measured over this repository's real
//! source, in every language the registry reads.
//!
//! **Thresholds, not comparisons** (§6's first line). What replaced the
//! differential harness, and the replacement is the point: "does this agree
//! with the producer being replaced" is a question about a transition, and
//! letting it set the agenda meant chasing a number that says nothing about
//! whether the graph is any good. A graph is judged against what a reader needs
//! from it:
//!
//! - **can I see every call and reference** — A2, A3;
//! - **does one symbol have one identity** — A7, without which "who calls this"
//!   silently answers for half the callers;
//! - **can the OO patterns be derived from nodes and edges alone** — R8, which
//!   is what makes "follow the approach already here" answerable instead of
//!   inviting a fourth way of doing the same thing;
//! - **is a dependency ours or somebody else's** — R5, A4.
//!
//! Every check here runs over the REAL corpus and reports its number. A fixture
//! proves the walk handles what its author thought of; three shipping
//! applications and a Rust workspace are where the unthought-of forms live.

use std::collections::{BTreeMap, BTreeSet};

use super::barrier;
use super::facts::{FileFacts, RefKind, RelationKind, Resolution, SymbolKind};
use super::fqn::Reach;
use super::lang::{self, Source, TypeHomes};
use super::resolve::{World, member_names_of, members_declared_by, resolve, returns_declared_by};

/// The directory a file's package is rooted at, from the file's own path.
///
/// Everything before `/src/`, or the first segment when there is none. Getting
/// this wrong is not a small error: `module_path` strips the root, so a root of
/// `.` left `crates/senseid/src/db/pg_store` standing as the module segment of
/// every identity that file declares — which collapsed four different `Row`
/// types onto one fqn and reported 1,111 identity collisions that were the
/// harness's own doing.
pub(super) fn package_root_of(path: &str) -> &str {
    match path.find("/src/") {
        Some(at) => &path[..at],
        None => path.split('/').next().unwrap_or("."),
    }
}

/// One file's facts, with the package that owns it and the text it was read
/// from.
///
/// The TEXT is kept because the coverage barrier needs it — a `#[cfg(test)]`
/// region is a property of the source and of nothing else — and re-opening the
/// file at that point would make the measurement depend on the disk still
/// holding what the walk read.
pub(super) struct Read {
    pub(super) package: String,
    pub(super) path: String,
    pub(super) text: String,
    pub(super) facts: FileFacts,
}

/// Read the whole corpus — every language — through the adapter each extension
/// dispatches to.
///
/// The barrier is here: every file is walked once with no type table so the
/// type DECLARATIONS can be collected, then walked again with the table, which
/// is what makes a member's identity independent of which file came first (R6).
pub(super) fn read_the_corpus() -> Vec<Read> {
    let mut sources: Vec<(String, String, String)> = Vec::new();
    for (abs, text) in super::corpus_rust_sources() {
        let package = super::package_of(&abs);
        let rel = super::workspace_relative(&abs);
        sources.push((package, rel, text));
    }
    for (rel, text) in super::corpus_web_sources() {
        // ONE PACKAGE PER FRONT END, named after its directory.
        //
        // They were one package called `web`, on the grounds that the real names
        // live in manifests this reader does not open and a wrong package would
        // split one symbol into three. It does the opposite: `app/`, `dojo/` and
        // `website/` are three separate applications that import nothing from
        // each other, so filing them together MERGES two declarations that are
        // genuinely distinct — `dojo/src/app.d.ts` and `website/src/app.d.ts`
        // both minting `typescript·web·app·mod`.
        //
        // The directory is not the manifest name, but it is one package per
        // package, which is the property every identity depends on.
        let package = rel.split('/').next().unwrap_or("web").to_string();
        sources.push((package, rel, text));
    }

    let read_all = |types: &TypeHomes| -> Vec<Read> {
        let mut out = Vec::new();
        for (package, path, text) in &sources {
            let Some(ext) = path.rsplit_once('.').map(|(_, e)| format!(".{e}")) else {
                continue;
            };
            let Some(adapter) = lang::adapter_for_ext(&ext) else { continue };
            let module = adapter.module_path(path, package_root_of(path));
            let source = Source { package, module: &module, path, text };
            // A file the grammar rejects is a FACT this harness reports (A9),
            // not one it hides.
            if let Ok(facts) = adapter.read(&source, types) {
                out.push(Read {
                    package: package.clone(),
                    path: path.clone(),
                    text: text.clone(),
                    facts,
                });
            }
        }
        out
    };

    let first = read_all(&TypeHomes::unknown());
    let homes = TypeHomes::of(
        first.iter().flat_map(|r| r.facts.symbols.iter().map(|s| (r.package.as_str(), s))),
    );
    // The SECOND pass, complete, before anything is placed. Every barrier
    // artifact below is taken off it rather than off `first`, and for
    // `declared_members` that is not a tidiness point: a member's identity
    // carries the module its TYPE lives in, so the pre-barrier pass spells
    // those members differently and a set taken from it would match nothing.
    let anchored = read_all(&homes);

    // AND THE LADDER. A walk states what it SAW; a target is placed by
    // resolution (R7), so a harness that reads without resolving sees almost
    // nothing resolved and would report every pattern as underivable. That is
    // what the first run of this file did.
    let first_party: BTreeSet<String> = sources.iter().map(|(p, _, _)| p.clone()).collect();
    // The BOUNDARY table, from the same completed pass the type table came
    // from: every member name any first-party type declares. A member the scan
    // declares NOWHERE cannot become a first-party edge, so the ladder labels
    // it the boundary rather than counting it as a miss.
    let first_party_members = member_names_of(anchored.iter().map(|r| &r.facts));
    let declared_members = members_declared_by(anchored.iter().map(|r| &r.facts));
    let returns = returns_declared_by(anchored.iter().map(|r| &r.facts));
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
        .map(|read| {
            let grammar = lang::adapter_for(read.facts.language).grammar();
            Read { facts: resolve(read.facts, grammar, &world), ..read }
        })
        .collect()
}

/// THE REPORT. Every reason, every language, one table, the same shape on every
/// run.
///
/// It exists because prose drifts. A number retyped into a summary acquires a
/// different label each time it is written, and a reader cannot tell a real
/// movement from a rephrasing. This is the ONE place the graph's coverage is
/// stated: every `Reason` variant appears as a row whether or not it occurred,
/// so a bucket going from absent to present is visible rather than being a new
/// line that was not there before.
///
///     cargo test -p senseid --bin senseid -- --ignored --nocapture \
///       indexer::acceptance::report
#[test]
#[ignore]
fn report() {
    let corpus = read_the_corpus();

    // Every language the registry reads, in a fixed order, whether or not the
    // corpus happened to contain one.
    let languages: Vec<&'static str> =
        super::facts::Language::all().iter().map(|l| l.as_str()).collect();
    // Every reason, in a fixed order, whether or not it occurred.
    let reasons = [
        "Unplaced",
        "NoImportInScope",
        "ReceiverTypeUnknown",
        "AmbiguousCandidates",
        "DynamicDispatch",
        "Plumbing",
        "ExternalBoundary",
        "NoDeclaredType",
        "MacroExpansion",
        "UnhandledForm",
    ];

    let mut cells: BTreeMap<(&str, String), usize> = BTreeMap::new();
    // The other half of the same question. A bare RESOLVED count says how much
    // of the graph is placed and nothing about how well — `declared_here` and
    // `through_a_glob` are both RESOLVED and are not the same claim.
    let mut rungs: BTreeMap<(&str, &'static str), usize> = BTreeMap::new();
    let mut files: BTreeMap<&str, usize> = BTreeMap::new();
    let mut symbols: BTreeMap<&str, usize> = BTreeMap::new();
    let mut relations: BTreeMap<&str, usize> = BTreeMap::new();
    let mut placed_relations: BTreeMap<&str, usize> = BTreeMap::new();
    let mut imports_local: BTreeMap<&str, usize> = BTreeMap::new();
    let mut imports_external: BTreeMap<&str, usize> = BTreeMap::new();

    for read in &corpus {
        let language = read.facts.language.as_str();
        *files.entry(language).or_default() += 1;
        *symbols.entry(language).or_default() += read.facts.symbols.len();
        *relations.entry(language).or_default() += read.facts.relations.len();
        *placed_relations.entry(language).or_default() += read
            .facts
            .relations
            .iter()
            .filter(|r| matches!(r.parent, Resolution::Resolved { .. }))
            .count();
        for import in &read.facts.imports {
            match import.origin {
                super::facts::ImportOrigin::Local => {
                    *imports_local.entry(language).or_default() += 1
                }
                super::facts::ImportOrigin::External { .. } => {
                    *imports_external.entry(language).or_default() += 1
                }
            }
        }
        for reference in &read.facts.references {
            let row = match &reference.target {
                Resolution::Resolved { via, .. } => {
                    *rungs.entry((language, via.as_label())).or_default() += 1;
                    "RESOLVED".to_string()
                }
                Resolution::Unresolved { reason, .. } => format!("{reason:?}"),
            };
            *cells.entry((language, row)).or_default() += 1;
        }
    }

    let width = 22;
    let row = |label: &str, get: &dyn Fn(&str) -> usize| {
        let mut line = format!("| {label:<width$} |");
        let mut total = 0;
        for language in &languages {
            let n = get(language);
            total += n;
            line.push_str(&format!(" {n:>10} |"));
        }
        line.push_str(&format!(" {total:>10} |"));
        println!("{line}");
    };

    let mut header = format!("| {:<width$} |", "");
    let mut rule = format!("|{}|", "-".repeat(width + 2));
    for language in &languages {
        header.push_str(&format!(" {language:>10} |"));
        rule.push_str(&format!("{}|", "-".repeat(12)));
    }
    header.push_str(&format!(" {:>10} |", "total"));
    rule.push_str(&format!("{}|", "-".repeat(12)));

    println!("\n## Corpus\n");
    println!("{header}");
    println!("{rule}");
    row("files read", &|l| files.get(l).copied().unwrap_or(0));
    row("declarations", &|l| symbols.get(l).copied().unwrap_or(0));
    row("relations", &|l| relations.get(l).copied().unwrap_or(0));
    row("imports, first-party", &|l| imports_local.get(l).copied().unwrap_or(0));
    row("imports, external", &|l| imports_external.get(l).copied().unwrap_or(0));

    // ── how much graph each declaration carries ──────────────────────────────
    //
    // An EDGE is a fact with both ends known: a reference the ladder placed, or
    // a relation whose parent it placed. An unresolved reference is a row with
    // a reason and no target — real, reportable, and not an edge, so counting
    // it here would make a graph look richer the WORSE it resolved.
    //
    // A NODE is a declaration. The ratio says how much structure one
    // declaration carries, which is the number a reader feels: a graph at 1.0
    // is a list, and one at 5.0 is something to traverse.
    println!("\n## Edges per node\n");
    println!("{header}");
    println!("{rule}");
    let edges = |l: &str| -> usize {
        cells.get(&(l, "RESOLVED".to_string())).copied().unwrap_or(0)
            + placed_relations.get(l).copied().unwrap_or(0)
    };
    row("resolved edges", &edges);
    row("nodes", &|l| symbols.get(l).copied().unwrap_or(0));
    print!("| {:<width$} |", "edges per node");
    for language in &languages {
        let n = symbols.get(language).copied().unwrap_or(0);
        let ratio = if n == 0 { 0.0 } else { edges(language) as f64 / n as f64 };
        print!(" {ratio:>10.2} |");
    }
    let all_edges: usize = languages.iter().map(|l| edges(l)).sum();
    let all_nodes: usize = symbols.values().sum();
    println!(" {:>10.2} |", if all_nodes == 0 { 0.0 } else { all_edges as f64 / all_nodes as f64 });

    println!("\n## References, by reason\n");
    println!("{header}");
    println!("{rule}");
    row("RESOLVED", &|l| cells.get(&(l, "RESOLVED".to_string())).copied().unwrap_or(0));
    for reason in reasons {
        row(reason, &|l| cells.get(&(l, reason.to_string())).copied().unwrap_or(0));
    }

    // The partition the prose kept reaching for. Stated in the table so nobody
    // has to invent a word for it — and the words invented so far collided with
    // `Unplaced`, which is a real variant meaning something else entirely.
    let doubt = |l: &str| -> usize {
        super::facts::Reason::ALL
            .iter()
            .filter(|r| r.casts_doubt())
            .map(|r| cells.get(&(l, format!("{r:?}"))).copied().unwrap_or(0))
            .sum()
    };
    let verdicts = |l: &str| -> usize {
        super::facts::Reason::ALL
            .iter()
            .filter(|r| !r.casts_doubt())
            .map(|r| cells.get(&(l, format!("{r:?}"))).copied().unwrap_or(0))
            .sum()
    };
    println!("\n## The same references, in the three groups that mean different things\n");
    println!("{header}");
    println!("{rule}");
    row("placed", &|l| cells.get(&(l, "RESOLVED".to_string())).copied().unwrap_or(0));
    row("outside, by verdict", &verdicts);
    row("doubt — a real gap", &doubt);

    println!("\n## Resolved references, by the rung that placed them\n");
    println!("{header}");
    println!("{rule}");
    for rung in super::facts::Rung::ALL {
        row(rung.as_label(), &|l| rungs.get(&(l, rung.as_label())).copied().unwrap_or(0));
    }

    let total: usize = cells.values().sum();
    assert!(total > 0, "the corpus produced no references at all");

    // ── the classification itself, ratcheted ─────────────────────────────────
    //
    // This report PRINTED for a long time and asserted almost nothing about the
    // histogram, so a shared-ladder change could reclassify half the corpus and
    // the only signal would be a person reading two runs side by side.
    //
    // SHARES, not counts. The Rust corpus is this repository, so it grows every
    // time the indexer does — 124,185 references became 124,684 in one session
    // of adding tests — and a count floor would fail on growth or pass on
    // regression depending on which moved faster.
    for language in &languages {
        let placed = cells.get(&(*language, "RESOLVED".to_string())).copied().unwrap_or(0);
        let seen: usize = std::iter::once("RESOLVED")
            .chain(reasons)
            .map(|r| cells.get(&(*language, r.to_string())).copied().unwrap_or(0))
            .sum();
        if seen == 0 {
            continue;
        }
        let share = 100.0 * placed as f64 / seen as f64;
        // MEASURED: rust 48.7%, typescript 40.4%. A point of slack, because the
        // corpus moves under us; anything larger is a change in behaviour and
        // has to be looked at rather than absorbed.
        let floor = match *language {
            "rust" => 47.7,
            "typescript" => 39.4,
            // NOT a default, and `_ => 0.0` is why: every share is `>= 0.0`, so
            // a language absent from this table got a gate that passed
            // unconditionally — present in the report, measured by nothing. The
            // next language added would have inherited it in silence, which is
            // the one failure mode a ratchet cannot survive.
            other => panic!(
                "no resolution floor is recorded for `{other}`. Run this test, read the share \
                 it reports, and record it here — a language in the corpus with no floor is \
                 not gated at all"
            ),
        };
        assert!(
            share >= floor,
            "{language} resolves {share:.1}% of its references, below the {floor}% measured — \
             a shared-ladder change has reclassified something"
        );
    }

    // A3's core, and it was asserted only for Java. Every reference reaches a
    // verdict: `Unplaced` means the ladder returned without answering, which is
    // the one reason no reader can act on.
    for language in &languages {
        assert_eq!(
            cells.get(&(*language, "Unplaced".to_string())).copied().unwrap_or(0),
            0,
            "{language} left references with no verdict at all"
        );
    }
    // The three groups are a PARTITION. If they stop summing to the whole, a
    // variant has been added that `casts_doubt` has not been taught about, and
    // the group table is quietly reporting a smaller corpus than the one above.
    let grouped: usize = languages
        .iter()
        .map(|l| {
            cells.get(&(*l, "RESOLVED".to_string())).copied().unwrap_or(0) + doubt(l) + verdicts(l)
        })
        .sum();
    assert_eq!(
        grouped, total,
        "placed + verdict + doubt is {grouped} of {total} — the three groups must partition \
         every reference, or the split is hiding some"
    );
    // The rungs must account for every RESOLVED reference. They are the only
    // way `climb` can return one, so a shortfall means a resolution was minted
    // somewhere that does not say which rung minted it.
    let placed: usize = languages
        .iter()
        .map(|l| cells.get(&(*l, "RESOLVED".to_string())).copied().unwrap_or(0))
        .sum();
    let by_rung: usize = rungs.values().sum();
    assert_eq!(
        by_rung, placed,
        "the rungs account for {by_rung} of {placed} placed references — an edge exists that \
         does not say which rung placed it, and a rung is the only thing mapping a wrong edge \
         back to the code that made it"
    );
    // Every reference is in exactly one row: the rows ARE the enum, so a new
    // variant that nothing lists would show up here as a missing total.
    let tabulated: usize = languages
        .iter()
        .map(|l| {
            std::iter::once("RESOLVED")
                .chain(reasons)
                .map(|r| cells.get(&(*l, r.to_string())).copied().unwrap_or(0))
                .sum::<usize>()
        })
        .sum();
    assert_eq!(
        tabulated, total,
        "the table accounts for {tabulated} of {total} references — a `Reason` variant exists \
         that this report has no row for, so the table is quietly incomplete"
    );
}

/// **A1. Import-mediated references resolve at a high rate.**
///
/// §6 puts this first and says why: an import NAMES its target, so a miss here
/// means the grammar is misread rather than that the answer was unknowable. It
/// is the sharpest signal the walk or the ladder is wrong, and nothing measured
/// it.
///
/// "Import-mediated" is computed, not assumed: a reference whose path HEAD is a
/// name an `Import` in the same file binds. That is exactly the population the
/// ladder's `through_an_import` rung claims, so a miss in it is a rung failing
/// on its own input.
///
/// A THRESHOLD per language, ratcheted at what is measured, because §6's target
/// of 99.9% stated as a hard gate today would fail and be waived.
#[test]
#[ignore]
fn an_import_named_target_resolves() {
    let corpus = read_the_corpus();
    let mut hit: BTreeMap<&str, usize> = BTreeMap::new();
    let mut missed: BTreeMap<&str, (usize, BTreeMap<String, usize>)> = BTreeMap::new();

    for read in &corpus {
        let language = read.facts.language.as_str();
        let separator = lang::adapter_for(read.facts.language).grammar().path_separator;
        let bound: BTreeSet<&str> =
            read.facts.imports.iter().filter_map(|i| i.binds.name()).collect();
        if bound.is_empty() {
            continue;
        }
        for reference in &read.facts.references {
            let (name, resolved) = match &reference.target {
                Resolution::Resolved { .. } => {
                    // A resolved target no longer carries the evidence that
                    // says how it was reached, so the population is counted
                    // from the reference's own `from`-side name where it has
                    // one. Resolved references are counted as hits only when
                    // the walk left a name to match, which is why the miss
                    // side below is the one that must be complete.
                    (None, true)
                }
                Resolution::Unresolved { reason, evidence } => {
                    (Some((evidence.name.clone(), format!("{reason:?}"))), false)
                }
            };
            match name {
                None if resolved => {}
                Some((name, reason)) => {
                    let head = name.split(separator).next().unwrap_or(&name);
                    if bound.contains(head) {
                        let entry = missed.entry(language).or_insert((0, BTreeMap::new()));
                        entry.0 += 1;
                        *entry.1.entry(reason).or_default() += 1;
                    }
                }
                None => {}
            }
        }
        // The hit side: every reference the ladder DID place in a file that has
        // imports. Counted per file so the rate has a denominator.
        *hit.entry(language).or_default() += read
            .facts
            .references
            .iter()
            .filter(|r| matches!(r.target, Resolution::Resolved { .. }))
            .count();
    }

    println!("\n## A1: references whose path head an import binds\n");
    for (language, (n, reasons)) in &missed {
        let resolved = hit.get(language).copied().unwrap_or(0);
        let rate =
            if resolved + n == 0 { 100.0 } else { 100.0 * resolved as f64 / (resolved + n) as f64 };
        println!("{language}: {resolved} resolved, {n} import-named MISSES ({rate:.1}%)");
        for (reason, count) in reasons {
            println!("  {count:>6}  {reason}");
        }
    }

    assert!(!missed.is_empty() || !hit.is_empty(), "no file in the corpus had an import");
    // THE RATCHET, per language. Measured, not aspirational.
    for (language, ceiling) in [("rust", 20_000usize), ("typescript", 8_000usize)] {
        let n = missed.get(language).map(|(n, _)| *n).unwrap_or(0);
        assert!(
            n <= ceiling,
            "{language}: {n} references name a head an import binds and did not resolve \
             (ratchet {ceiling}). An import names its target, so this is the grammar or the \
             ladder being wrong, not an unknowable answer"
        );
    }
}

/// A8. Every RESOLVED edge that names one of ours names a declaration this scan
/// actually holds.
///
/// The measurement that was missing, and its absence is why a whole language's
/// import rung could be broken in plain sight. `report` counts a reference as
/// RESOLVED the moment a rung answers; it never asks whether the identity that
/// rung minted exists. So 3,865 TypeScript import edges — every first-party one
/// there was — pointed at nodes no file declares, and each was counted a
/// success by the histogram, by A1, and by the resolve-share ratchet alike.
///
/// A DANGLING edge is not a wrong edge and R4 prefers it to one. But it is not a
/// success either: a reader following "who calls this" gets nothing, which is
/// the same answer they would get if the caller did not exist.
///
/// # What this CANNOT see, and it is the other half of R4
///
/// A WRONG edge. The only question asked here is whether the target exists, and
/// a wrong edge points at a node that does — that is what makes it the worse of
/// the two. So this test running byte-identical across a change is evidence
/// about dangling edges and about nothing else, and reading "A8 unchanged" as
/// "no wrong edge was introduced" is reading a number that was never measured.
///
/// It has already happened: three defects in `Ladder::types_home_of` each
/// placed a member on a type whose home came from another module, another
/// package or another language, every one of those targets was a real
/// declaration, and this table did not move. Nothing structural can separate
/// them — a wrong edge and a right one differ only in what the SOURCE meant —
/// so the cover for that class is a fixture stating the intended target, and
/// `resolve.rs` carries one per clause.
///
/// Split by RUNG, because that is what makes it actionable. A rung with a high
/// dangling count is one function to go and read; a single total is a number
/// nobody can act on. Library targets are excluded by [`fqn::parse`] rather than
/// by a string test — R5 says an external is named and never opened, so nothing
/// this scan declares could confirm one.
#[test]
#[ignore]
fn every_first_party_edge_names_a_declaration_this_scan_holds() {
    let corpus = read_the_corpus();
    let declared: BTreeSet<&str> =
        corpus.iter().flat_map(|r| r.facts.symbols.iter().map(|s| s.fqn.as_str())).collect();

    // (language, fact, rung) -> (landed, dangling), plus the worst offenders.
    //
    // A REFERENCE and a RELATION are split because they are different claims —
    // "this use site reaches that" against "this declaration hangs off that" —
    // and they are minted by different code. Merged, the larger one hides the
    // smaller: rust's 652 dangling relation parents and its 298 dangling
    // reference targets have nothing to do with each other.
    let mut tally: BTreeMap<(&str, &str, &str), (usize, usize)> = BTreeMap::new();
    let mut examples: BTreeMap<&str, BTreeMap<&str, usize>> = BTreeMap::new();
    for read in &corpus {
        let language = read.facts.language.as_str();
        let targets = read
            .facts
            .references
            .iter()
            .map(|r| ("reference", &r.target))
            .chain(read.facts.relations.iter().map(|r| ("relation", &r.parent)));
        for (fact, target) in targets {
            let Resolution::Resolved { fqn, via } = target else { continue };
            // An external names a package we never open, so there is no
            // declaration of ours for it to match and counting it either way
            // would be meaningless.
            match super::fqn::parse(fqn.as_str()) {
                Ok(parsed) if parsed.origin == super::fqn::Origin::Lib => continue,
                // An identity that will not parse is a defect of its own, and
                // it is one A7 and the fqn suite own. Counted as dangling here
                // rather than skipped, because it certainly is not landed.
                Err(_) => {}
                Ok(_) => {}
            }
            let slot = tally.entry((language, fact, via.as_label())).or_default();
            if declared.contains(fqn.as_str()) {
                slot.0 += 1;
            } else {
                slot.1 += 1;
                *examples.entry(language).or_default().entry(fqn.as_str()).or_default() += 1;
            }
        }
    }

    println!("\n## A8: first-party edges, and whether the target is declared\n");
    println!(
        "  {:<12} {:<10} {:<26} {:>8} {:>10}",
        "language", "fact", "rung", "landed", "DANGLING"
    );
    let mut dangling_by_language: BTreeMap<&str, usize> = BTreeMap::new();
    for ((language, fact, rung), (landed, dangling)) in &tally {
        println!("  {language:<12} {fact:<10} {rung:<26} {landed:>8} {dangling:>10}");
        *dangling_by_language.entry(language).or_default() += dangling;
    }
    for (language, worst) in &examples {
        let mut top: Vec<(&&str, &usize)> = worst.iter().collect();
        top.sort_by_key(|(fqn, n)| (std::cmp::Reverse(**n), **fqn));
        println!("\n  {language}: {} distinct dangling targets, worst 10:", worst.len());
        for (fqn, n) in top.into_iter().take(10) {
            println!("    {n:>5}  {fqn}");
        }
    }

    // THE RATCHET, per language. Measured, and it moved a long way to get here:
    // typescript stood at 3,865 dangling with ZERO landed before the import
    // clause was read, which is the whole of its first-party import traffic.
    //
    // What the remainder is, decomposed rather than absorbed:
    //
    // - rust 987 = 652 RELATION parents + 298 imports + 37 rooted paths. The
    //   652 are one shape: an `Owns` relation from an `impl PgStore` block
    //   mints its parent at the module the IMPL sits in, where the struct is
    //   declared one module out. The member's own identity goes through
    //   `TypeHomes` and is right; the relation's parent does not. Pre-existing,
    //   found by this test, and its own slice — see `docs/backlog.md`.
    // - typescript 561 = 523 in one re-export barrel (`e2e/fixtures` passes
    //   Playwright's `test` and `expect` straight through) plus 38 in
    //   `lib/components/kit/index`. Both are `export { x } from './y'`, where
    //   the target IS the module named and the declaration is one module
    //   further on. Following a barrel needs a cross-file re-export table,
    //   which is the lookup deferred to #174.
    for (language, ceiling) in [("rust", 1_200usize), ("typescript", 700)] {
        let n = dangling_by_language.get(language).copied().unwrap_or(0);
        assert!(
            n <= ceiling,
            "{language}: {n} resolved first-party edges name an identity no declaration \
             mints (ratchet {ceiling}). A rung answered and the answer reaches nothing"
        );
    }
}

/// What `NoImportInScope` ACTUALLY is.
///
/// "A bare name with no local declaration and no import that binds it" — now
/// the largest bucket, and the same question applies to it as to the untyped
/// receivers: could the target be OURS? A name no first-party declaration
/// carries anywhere cannot be an edge we lost.
///
/// The split is finer here, because a bare name has more ways to be legitimate:
/// a glob import in scope can bind it without naming it, and a prelude name is
/// in scope with nothing written at all.
#[test]
#[ignore]
fn what_the_unimported_names_are() {
    let corpus = read_the_corpus();

    // EVERY first-party declaration name, not just members: a bare name reaches
    // a free function or a type, which a member-only set would miss.
    let mut declared: BTreeSet<&str> = BTreeSet::new();
    for read in &corpus {
        for symbol in &read.facts.symbols {
            declared.insert(symbol.name.as_str());
        }
    }

    let mut buckets: BTreeMap<(&str, &str), usize> = BTreeMap::new();
    let mut head: BTreeMap<&str, usize> = BTreeMap::new();
    for read in &corpus {
        let language = read.facts.language.as_str();
        let grammar = lang::adapter_for(read.facts.language).grammar();
        let prelude: BTreeSet<&str> = grammar.prelude.iter().map(|(name, _, _)| *name).collect();
        let has_glob = read.facts.imports.iter().any(|i| i.binds == super::facts::Binding::Glob);

        for reference in &read.facts.references {
            let Resolution::Unresolved { reason, evidence } = &reference.target else { continue };
            if format!("{reason:?}") != "NoImportInScope" {
                continue;
            }
            let name = evidence.name.as_str();
            let head_segment = name.split(grammar.path_separator).next().unwrap_or(name);
            let bucket = if prelude.contains(head_segment) {
                "in the language PRELUDE — external, and already nameable"
            } else if declared.contains(name) || declared.contains(head_segment) {
                "a name we DO declare somewhere — a lost first-party edge"
            } else if has_glob {
                "a GLOB is in scope, so the binding is unknowable here"
            } else {
                "declared nowhere first-party — boundary"
            };
            *buckets.entry((language, bucket)).or_default() += 1;
            if bucket.starts_with("declared nowhere") {
                *head.entry(name).or_default() += 1;
            }
        }
    }

    println!("\n## NoImportInScope, by what the name IS\n");
    println!("| {:<48} | {:>8} | {:>10} |", "", "rust", "typescript");
    println!("|{}|{}|{}|", "-".repeat(50), "-".repeat(10), "-".repeat(12));
    let mut rows: BTreeSet<&str> = BTreeSet::new();
    for (_, bucket) in buckets.keys() {
        rows.insert(bucket);
    }
    for bucket in &rows {
        println!(
            "| {:<48} | {:>8} | {:>10} |",
            bucket,
            buckets.get(&("rust", *bucket)).copied().unwrap_or(0),
            buckets.get(&("typescript", *bucket)).copied().unwrap_or(0)
        );
    }

    let mut ranked: Vec<(&&str, &usize)> = head.iter().collect();
    ranked.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
    println!("\nthe boundary's head — names nothing first-party declares:");
    for (name, n) in ranked.iter().take(15) {
        println!("  {n:>6}  {name}");
    }
    assert!(!buckets.is_empty(), "no unimported names, so this proved nothing");
}

/// The residue: names we DO declare somewhere and still could not place.
///
/// The largest bucket left after the cheap rules, and unlike them it has no
/// single mechanism — so it is decomposed by the SHAPE of the path, which is
/// what decides which rung should have fired.
#[test]
#[ignore]
fn what_the_placeable_names_are() {
    let corpus = read_the_corpus();
    let mut declared: BTreeSet<&str> = BTreeSet::new();
    for read in &corpus {
        for symbol in &read.facts.symbols {
            declared.insert(symbol.name.as_str());
        }
    }

    let mut shapes: BTreeMap<(&str, &str), usize> = BTreeMap::new();
    let mut head: BTreeMap<&str, usize> = BTreeMap::new();
    for read in &corpus {
        let language = read.facts.language.as_str();
        let separator = lang::adapter_for(read.facts.language).grammar().path_separator;
        for reference in &read.facts.references {
            let Resolution::Unresolved { reason, evidence } = &reference.target else { continue };
            if format!("{reason:?}") != "NoImportInScope" {
                continue;
            }
            let name = evidence.name.as_str();
            let first = name.split(separator).next().unwrap_or(name);
            if !declared.contains(name) && !declared.contains(first) {
                continue;
            }
            let segments = name.split(separator).count();
            let shape = if segments > 1 {
                "a multi-segment PATH — the rung that should place it is rooted or import"
            } else if evidence.reach == Reach::Field {
                "a FIELD reach — no path rung may serve one, by design"
            } else if name.chars().next().is_some_and(char::is_uppercase) {
                "a bare TYPE name — declared elsewhere, no import placed it"
            } else {
                "a bare VALUE name — a function or const declared elsewhere"
            };
            *shapes.entry((language, shape)).or_default() += 1;
            *head.entry(name).or_default() += 1;
        }
    }

    println!("\n## The placeable residue, by path shape\n");
    println!("| {:<58} | {:>8} | {:>10} |", "", "rust", "typescript");
    println!("|{}|{}|{}|", "-".repeat(60), "-".repeat(10), "-".repeat(12));
    let rows: BTreeSet<&str> = shapes.keys().map(|(_, s)| *s).collect();
    for shape in &rows {
        println!(
            "| {:<58} | {:>8} | {:>10} |",
            shape,
            shapes.get(&("rust", *shape)).copied().unwrap_or(0),
            shapes.get(&("typescript", *shape)).copied().unwrap_or(0)
        );
    }
    let mut ranked: Vec<(&&str, &usize)> = head.iter().collect();
    ranked.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
    println!("\nhead:");
    for (name, n) in ranked.iter().take(12) {
        println!("  {n:>6}  {name}");
    }
    assert!(!shapes.is_empty(), "no placeable residue, so this proved nothing");
}

/// `(language, shape)` and the count plus a few examples, ranked for display.
type Ranked<'a> = (&'a (&'a str, &'a str), &'a (usize, Vec<String>));

/// What `ReceiverTypeUnknown` ACTUALLY is, bucketed by the shape of the
/// receiver the walk could not type.
///
/// It is the largest bucket in the report by a wide margin, and a number that
/// large is useless until it is decomposed: "53,224 receivers untyped" invites
/// the guess that the walk is broken, when most of it may be one shape with one
/// cause. The walk records the receiver verbatim as `Observation::Receiver`, so
/// this reads what it actually saw rather than inferring.
#[test]
#[ignore]
fn what_the_untyped_receivers_are() {
    let corpus = read_the_corpus();
    let mut shapes: BTreeMap<(&str, &str), (usize, Vec<String>)> = BTreeMap::new();

    for read in &corpus {
        let language = read.facts.language.as_str();
        for reference in &read.facts.references {
            let Resolution::Unresolved { reason, evidence } = &reference.target else { continue };
            if format!("{reason:?}") != "ReceiverTypeUnknown" {
                continue;
            }
            let receiver = evidence.saw.iter().find_map(|o| match o {
                super::facts::Observation::Receiver(text) => Some(text.as_str()),
                _ => None,
            });
            let Some(receiver) = receiver else { continue };
            let one_line = receiver.split('\n').next().unwrap_or(receiver).trim();
            // The shape, not the text: what KIND of expression the walk was
            // asked to name a type for.
            let shape = if one_line.ends_with(')') && one_line.contains('(') {
                "a CALL's result — needs the callee's return type (#174)"
            } else if one_line.starts_with("self.") || one_line.starts_with("this.") {
                "a field of self/this whose type is not stated here"
            } else if one_line.contains('.') {
                "a longer CHAIN — a.b.c"
            } else if one_line.contains('[') {
                "an index or slice"
            } else if one_line.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '$') {
                "a BARE NAME the file never typed — usually an import (#174)"
            } else {
                "other"
            };
            let entry = shapes.entry((language, shape)).or_insert((0, Vec::new()));
            entry.0 += 1;
            if entry.1.len() < 3 && !one_line.is_empty() {
                entry.1.push(format!("{}  —  {one_line}", read.path));
            }
        }
    }

    println!("\n## ReceiverTypeUnknown, by receiver shape\n");
    let mut ranked: Vec<Ranked<'_>> = shapes.iter().collect();
    ranked.sort_by_key(|(_, (n, _))| std::cmp::Reverse(*n));
    for ((language, shape), (n, examples)) in ranked {
        println!("{n:>7}  [{language}] {shape}");
        for example in examples {
            println!("           {example}");
        }
    }
    assert!(!shapes.is_empty(), "no untyped receiver carried the receiver it saw");
}

/// WHERE GOING DEEPER WOULD PAY, and where it would not.
///
/// `ReceiverTypeUnknown` is the largest bucket and reads as a gap. Most of it
/// is not one, and the difference decides whether any more receiver-typing work
/// is worth doing.
///
/// The split is on ONE question a reader actually cares about: could the target
/// be OURS? A member name that no first-party declaration anywhere in the
/// corpus carries cannot be a first-party edge, whatever we learn about the
/// receiver — it is `trim`, `ok`, `unwrap`, `toBeVisible`: a transform on a
/// library type, at the boundary R5 says we name and never open.
///
/// The matching side is an UPPER BOUND, not an answer: a name can coincide
/// between a library method and one of ours. It is the ceiling on what more
/// typing could buy, and a ceiling is what a build/do-not-build decision needs.
#[test]
#[ignore]
fn where_going_deeper_would_pay() {
    let corpus = read_the_corpus();

    // Every member name any first-party type declares — the same set the
    // ladder is handed as `World::first_party_members`, so this ceiling is
    // measured against the boundary the resolver actually applied.
    let ours = member_names_of(corpus.iter().map(|r| &r.facts));

    let mut could_be_ours: BTreeMap<&str, usize> = BTreeMap::new();
    let mut boundary: BTreeMap<&str, usize> = BTreeMap::new();
    let mut top_boundary: BTreeMap<&str, usize> = BTreeMap::new();
    for read in &corpus {
        let language = read.facts.language.as_str();
        for reference in &read.facts.references {
            let Resolution::Unresolved { reason, evidence } = &reference.target else { continue };
            if format!("{reason:?}") != "ReceiverTypeUnknown" {
                continue;
            }
            if ours.contains(evidence.name.as_str()) {
                *could_be_ours.entry(language).or_default() += 1;
            } else {
                *boundary.entry(language).or_default() += 1;
                *top_boundary.entry(evidence.name.as_str()).or_default() += 1;
            }
        }
    }

    println!("\n## ReceiverTypeUnknown: could the target be OURS?\n");
    println!("| {:<26} | {:>10} | {:>10} |", "", "rust", "typescript");
    println!("|{}|{}|{}|", "-".repeat(28), "-".repeat(12), "-".repeat(12));
    let cell = |m: &BTreeMap<&str, usize>, l: &str| m.get(l).copied().unwrap_or(0);
    println!(
        "| {:<26} | {:>10} | {:>10} |",
        "could be ours (CEILING)",
        cell(&could_be_ours, "rust"),
        cell(&could_be_ours, "typescript")
    );
    println!(
        "| {:<26} | {:>10} | {:>10} |",
        "boundary — not ours at all",
        cell(&boundary, "rust"),
        cell(&boundary, "typescript")
    );

    let mut ranked: Vec<(&&str, &usize)> = top_boundary.iter().collect();
    ranked.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
    println!("\nthe boundary's head — names no first-party type declares:");
    for (name, n) in ranked.iter().take(15) {
        println!("  {n:>6}  .{name}()");
    }

    assert!(!ours.is_empty(), "no first-party member names, so the split proved nothing");
}

/// Does the graph already carry what an LLM needs per function?
///
/// The shape asked for: `A()` lives at `<path>` lines `n:m`, takes these
/// params, returns this, and calls B, C, D. If every function has that, the
/// remaining receiver-typing work buys refinement rather than capability, and
/// the honest answer to "should we go deeper" is no.
#[test]
#[ignore]
fn every_function_carries_what_a_reader_needs() {
    let corpus = read_the_corpus();
    let mut per_language: BTreeMap<&str, [usize; 5]> = BTreeMap::new();

    for read in &corpus {
        let language = read.facts.language.as_str();
        // Which symbol each reference sits inside — the caller side.
        let mut calls_from: BTreeMap<&str, usize> = BTreeMap::new();
        for reference in &read.facts.references {
            if matches!(reference.kind, RefKind::Calls | RefKind::Constructs) {
                *calls_from.entry(reference.from.as_str()).or_default() += 1;
            }
        }
        for symbol in &read.facts.symbols {
            if !matches!(symbol.kind, SymbolKind::Function | SymbolKind::Method) {
                continue;
            }
            let counts = per_language.entry(language).or_insert([0; 5]);
            counts[0] += 1;
            // A line RANGE, not a point: `n:m` is what makes the body fetchable.
            if symbol.span.end_line > symbol.span.start_line {
                counts[1] += 1;
            }
            if !symbol.params.is_empty() {
                counts[2] += 1;
            }
            if matches!(symbol.declared_type, super::facts::DeclaredType::Stated(_)) {
                counts[3] += 1;
            }
            if calls_from.contains_key(symbol.fqn.as_str()) {
                counts[4] += 1;
            }
        }
    }

    println!("\n## Per function: is the reader's shape already there?\n");
    println!("| {:<26} | {:>10} | {:>10} |", "", "rust", "typescript");
    println!("|{}|{}|{}|", "-".repeat(28), "-".repeat(12), "-".repeat(12));
    let get =
        |m: &BTreeMap<&str, [usize; 5]>, l: &str, i: usize| m.get(l).map(|c| c[i]).unwrap_or(0);
    for (i, label) in [
        "functions and methods",
        "with a line RANGE",
        "with params recorded",
        "with a return type",
        "with >=1 outgoing call",
    ]
    .iter()
    .enumerate()
    {
        println!(
            "| {:<26} | {:>10} | {:>10} |",
            label,
            get(&per_language, "rust", i),
            get(&per_language, "typescript", i)
        );
    }
    assert!(!per_language.is_empty(), "no functions found, so this proved nothing");
}

/// **A7. No two declarations mint one identity.**
///
/// The guard §2.1 leans on when it drops the type/value discriminator, and it
/// had never been run corpus-wide — only per-file fixtures and a per-folder
/// check at write time, neither of which can see two FILES colliding.
///
/// It matters beyond tidiness. The fqn is the merge key, so two declarations
/// under one identity are one node in the graph: "who calls this" answers for
/// both, "where is this defined" picks one, and which one is whatever the scan
/// wrote last. A reader cannot tell that from a correct answer.
///
/// Reported as a RATCHET rather than asserted at zero. §6 records why: a gate
/// stated as "zero" that fails on its first run gets waived, and a waived gate
/// is not a gate. The number may fall; it may not rise.
///
/// # 525 today. The function-body cause is CLOSED for Rust and PARTIAL for
/// TypeScript
///
/// A declaration inside a FUNCTION BODY is minted as if it sat at module scope.
///
/// ```ignore
/// fn dep_re() -> &'static Regex { static RE: OnceLock<Regex> = OnceLock::new(); .. }
/// fn plugin_re() -> &'static Regex { static RE: OnceLock<Regex> = OnceLock::new(); .. }
/// ```
///
/// Both mint one identity, as does a third in the same file. Three unrelated
/// regexes, one node, and "where is `RE` defined" answers with whichever the
/// scan wrote last. `COPY_CAP`, `status_of`, `ARMS` and — in TypeScript —
/// `deadline` are the same shape, so it is not a Rust quirk.
///
/// The repair is the walk giving a function body its own container, so a local
/// declaration is either named under its function or not emitted as a
/// module-level item at all. Which of those is right is a grammar decision and
/// is not made here; what is settled is that the current answer is wrong.
///
/// **A SECOND cause, and it is not function bodies.** A `#[cfg(feature = "x")]`
/// / `#[cfg(not(feature = "x"))]` pair declares one name twice at MODULE scope
/// — `provision_status` in `api/handlers/model_provisioning.rs` is exactly
/// that, and an earlier version of this comment mis-attributed it to the
/// function-body cause. Only one of the two is compiled, but the walk reads
/// text and cannot know which; the identity is the same either way, so this may
/// be a collision the grammar should tolerate rather than repair. Not settled.
/// `const _` twice in one file (`tasks/handlers/embed.rs`) is a third thing
/// again: an anonymous name is not a name.
///
/// # TypeScript's 522 — what the function-body fix does NOT yet reach
///
/// A declaration is now named under its enclosing function for a `function`
/// declaration and for a function or arrow bound to a `const`. Two shapes are
/// still un-scoped, and they are the common ones in this corpus:
///
/// - an ANONYMOUS arrow in argument position — `items.map(x => { const y = 1 })`
///   — walked by the `ArrowFunctionExpression` arm of `Walk::expression`, which
///   has no name to push;
/// - a METHOD body, walked from `Walk::class_element`, where the container is
///   `Type` so a local is minted as a MEMBER of the class rather than a local
///   of the method.
///
/// Both need a name for the enclosing scope that a use site could also compose.
/// An anonymous arrow has none, so this is the same grammar question the Rust
/// side answered differently: what identity does an unreachable declaration
/// get? Not settled for the anonymous case.
///
/// # What this number was before, and why it moved
///
/// 708 -> 525 is the function-body fix: Rust 13 -> 3, TypeScript 695 -> 522.
///
/// 1,114 was reported before that, and 375 of those were ONE declaration emitted TWICE by
/// the JavaScript walk walking a function initialiser on two paths — a
/// declaration colliding with itself, not two declarations sharing a scope. A
/// further 31 were files the shipped scan never reads, which the corpus was
/// picking up because it restated the ignore policy instead of sharing it.
/// Both are fixed. The remaining 708 are real.
#[test]
#[ignore]
fn no_two_declarations_mint_one_identity() {
    let corpus = read_the_corpus();
    let mut minted: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for read in &corpus {
        for symbol in &read.facts.symbols {
            minted
                .entry(symbol.fqn.as_str().to_string())
                .or_default()
                .push(format!("{}:{}", read.path, symbol.span.start_line));
        }
    }
    let collisions: Vec<(&String, &Vec<String>)> =
        minted.iter().filter(|(_, sites)| sites.len() > 1).collect();

    println!("\n── A7: one declaration, one identity ──");
    println!("declarations {}", minted.values().map(Vec::len).sum::<usize>());
    println!("identities   {}", minted.len());
    println!("COLLIDING    {}", collisions.len());
    for (fqn, sites) in collisions.iter().take(15) {
        println!("  {fqn}");
        for site in sites.iter().take(4) {
            println!("      {site}");
        }
    }

    assert!(!minted.is_empty(), "the corpus minted no identities, so this passed vacuously");
    // PER LANGUAGE, because one total lets a regression in the smaller language
    // hide under the larger one's headroom: Rust's 13 could triple and the
    // grand total would still sit under any single number.
    let mut per_language: BTreeMap<&str, usize> = BTreeMap::new();
    for (fqn, _) in &collisions {
        if let Ok(parsed) = super::fqn::parse(fqn)
            && let super::fqn::Origin::Local { lang, .. } = parsed.origin
        {
            *per_language.entry(lang.as_str()).or_default() += 1;
        }
    }
    for (language, n) in &per_language {
        println!("  {n:>5}  {language}");
    }

    // THE RATCHET. Lower it when a collision is repaired; never raise it to
    // make a run pass.
    // LOWERED, not raised. Giving each front end its own package repaired 11
    // cross-app false merges — two apps' `src/app.d.ts` were one identity —
    // and the file-module emission added none that survive it.
    //
    // **RAISED ONCE, BY ONE, AND THIS IS THE JUSTIFICATION** — the exception the
    // line above otherwise forbids. Stage 11's S8 keys a method on its type and
    // its name and demotes the trait to a `TraitImpl` edge, so an inherent
    // method and a same-named one a trait impl supplies become ONE node.
    //
    // That is not an accident of the implementation, it is the trade spec §7
    // makes explicitly: a merge key must be what BOTH sides can produce, and a
    // caller writing `p.status_all()` cannot spell which of the two it meant —
    // that is what dispatch decides. The segment that told them apart could
    // only ever be minted by one side, which is why a repo-wide translation
    // table existed to bridge it.
    //
    // What S8 changes is WHERE the cost is visible. Before, the mismatch showed
    // up as an edge silently refused — 22 references over 3 identities, which no
    // ratchet counts and no reader of this table could see. Now it shows up
    // here, as a collision, on every run.
    //
    // MEASURED over this whole repository, not predicted: rust 3 -> 4. Exactly
    // ONE pair merges — `ModelProvisioning`'s inherent `status_all` and the one
    // its `impl ReadinessProbe` supplies, which delegates to it. The shape
    // permits more (any type with two traits supplying one name); this corpus
    // contains no other instance.
    const KNOWN: usize = 515;
    const KNOWN_RUST: usize = 4;
    const KNOWN_TYPESCRIPT: usize = 511;
    assert!(
        collisions.len() <= KNOWN,
        "identity collisions rose to {} (ratchet {KNOWN}) — two declarations under one \
         identity are ONE node, and every query about either answers for both",
        collisions.len()
    );
    assert!(
        per_language.get("rust").copied().unwrap_or(0) <= KNOWN_RUST,
        "rust collisions rose to {:?} (ratchet {KNOWN_RUST})",
        per_language.get("rust")
    );
    assert!(
        per_language.get("typescript").copied().unwrap_or(0) <= KNOWN_TYPESCRIPT,
        "typescript collisions rose to {:?} (ratchet {KNOWN_TYPESCRIPT})",
        per_language.get("typescript")
    );
}

/// **A3. Every unresolved reference carries a reason, and the histogram
/// accounts for 100% of them.**
///
/// Run for every language rather than for Rust alone, because a reason
/// histogram is only a measurement while every language fills the same buckets
/// (R7). A language whose misses landed in a bucket of its own would make the
/// total unreadable.
#[test]
#[ignore]
fn every_miss_is_accounted_for_in_every_language() {
    let corpus = read_the_corpus();
    let mut by_language: BTreeMap<&str, (usize, usize, BTreeMap<String, usize>)> = BTreeMap::new();
    for read in &corpus {
        let entry =
            by_language.entry(read.facts.language.as_str()).or_insert((0, 0, BTreeMap::new()));
        for reference in &read.facts.references {
            match &reference.target {
                Resolution::Resolved { .. } => entry.0 += 1,
                Resolution::Unresolved { reason, evidence } => {
                    entry.1 += 1;
                    *entry.2.entry(format!("{reason:?}")).or_default() += 1;
                    assert!(
                        !evidence.name.is_empty() && !evidence.node_kind.is_empty(),
                        "{}: a miss with nothing in it is one nobody can act on",
                        read.path
                    );
                }
            }
        }
    }

    println!("\n── A3: every miss named, per language ──");
    for (language, (resolved, unresolved, histogram)) in &by_language {
        let counted: usize = histogram.values().sum();
        println!("\n{language}: resolved {resolved} | unresolved {unresolved}");
        for (reason, n) in histogram {
            println!("  {n:>7}  {reason}");
        }
        assert_eq!(
            counted, *unresolved,
            "{language}: the histogram accounts for {counted} of {unresolved} misses"
        );
    }
    assert!(by_language.len() > 1, "only one language read, so this proved nothing shared");
}

/// **R8 / spec 04 S10. The seven patterns are derivable from nodes and edges
/// alone, with no return to source.**
///
/// The requirement 04 named as "the one most likely to be skipped", and it was
/// skipped. It is the one that makes "what approach is already in use here"
/// answerable — which is the difference between a reader following the pattern
/// in front of them and inventing a fourth way to do the same thing.
///
/// Two things are checked, and they are different:
///
/// 1. every fact in R8's UNION is actually emitted, in non-trivial quantity;
/// 2. a pattern can be DERIVED from those facts — demonstrated by deriving
///    Facade and Adapter, the two whose fact requirements are the widest.
///
/// A pattern with zero instances is reported and does not fail: this repository
/// not using the Observer pattern is a fact about the repository, while a
/// MISSING FACT is a defect in the walk. The two are told apart by checking the
/// facts and the derivations separately.
#[test]
#[ignore]
fn the_facts_the_seven_patterns_need_are_all_emitted_and_two_are_derived() {
    let corpus = read_the_corpus();

    let mut field_types = 0usize;
    let mut param_types = 0usize;
    let mut return_types = 0usize;
    let mut inheritance = 0usize;
    let mut construction = 0usize;
    let mut ownership = 0usize;
    let mut statics = 0usize;
    let mut member_calls = 0usize;

    for read in &corpus {
        for symbol in &read.facts.symbols {
            let stated = matches!(symbol.declared_type, super::facts::DeclaredType::Stated(_));
            match symbol.kind {
                SymbolKind::Field | SymbolKind::Property if stated => field_types += 1,
                SymbolKind::Function | SymbolKind::Method if stated => return_types += 1,
                SymbolKind::Static => statics += 1,
                _ => {}
            }
            param_types += symbol
                .params
                .iter()
                .filter(|p| matches!(p.declared_type, super::facts::DeclaredType::Stated(_)))
                .count();
        }
        for relation in &read.facts.relations {
            match relation.kind {
                RelationKind::Extends | RelationKind::Implements | RelationKind::TraitImpl => {
                    inheritance += 1
                }
                RelationKind::Owns => ownership += 1,
                _ => {}
            }
        }
        for reference in &read.facts.references {
            match reference.kind {
                RefKind::Constructs => construction += 1,
                RefKind::Calls => member_calls += 1,
                _ => {}
            }
        }
    }

    println!("\n── R8: the union of facts the seven patterns need ──");
    let required = [
        ("field types", field_types),
        ("parameter types", param_types),
        ("return types", return_types),
        ("implements / extends", inheritance),
        ("construction edges", construction),
        ("member ownership", ownership),
        ("statics (Singleton)", statics),
        ("call edges", member_calls),
    ];
    for (fact, n) in required {
        println!("  {n:>7}  {fact}");
    }
    for (fact, n) in required {
        assert!(
            n > 0,
            "R8 needs {fact} and the walk emitted none — every pattern that reads it is \
             underivable, and pattern detection would report absence as a fact about the code"
        );
    }

    // ── the derivations, from nodes and edges only ───────────────────────────
    //
    // Nothing below opens a source file. If a derivation needs something the
    // facts do not carry, it cannot be written — which is exactly what S10 is
    // asking.

    // FACADE: a type whose members call members of OTHER types. Needs call
    // edges and member ownership, and nothing else.
    let mut owner_of: BTreeMap<String, String> = BTreeMap::new();
    for read in &corpus {
        for relation in &read.facts.relations {
            if relation.kind == RelationKind::Owns
                && let Resolution::Resolved { fqn: owner, .. } = &relation.parent
            {
                owner_of.insert(relation.child.as_str().to_string(), owner.as_str().to_string());
            }
        }
    }
    let mut facade_reach: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for read in &corpus {
        for reference in &read.facts.references {
            if reference.kind != RefKind::Calls {
                continue;
            }
            let Resolution::Resolved { fqn: target, .. } = &reference.target else { continue };
            let (Some(caller), Some(callee)) =
                (owner_of.get(reference.from.as_str()), owner_of.get(target.as_str()))
            else {
                continue;
            };
            if caller != callee {
                facade_reach.entry(caller.clone()).or_default().insert(callee.clone());
            }
        }
    }
    let facades: Vec<(&String, usize)> = {
        let mut v: Vec<(&String, usize)> =
            facade_reach.iter().map(|(ty, reached)| (ty, reached.len())).collect();
        v.sort_by_key(|(_, reached)| std::cmp::Reverse(*reached));
        v
    };

    // ADAPTER: a type that IMPLEMENTS something and HOLDS a field, which is
    // the shape R8 names. Needs `implements` and field types.
    let mut implements: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for read in &corpus {
        for relation in &read.facts.relations {
            if matches!(
                relation.kind,
                RelationKind::Implements | RelationKind::TraitImpl | RelationKind::Extends
            ) && let Resolution::Resolved { fqn: parent, .. } = &relation.parent
            {
                implements
                    .entry(relation.child.as_str().to_string())
                    .or_default()
                    .insert(parent.as_str().to_string());
            }
        }
    }
    let mut holds_a_field: BTreeSet<String> = BTreeSet::new();
    for read in &corpus {
        for symbol in &read.facts.symbols {
            if symbol.kind == SymbolKind::Field
                && let Some(owner) = owner_of.get(symbol.fqn.as_str())
            {
                holds_a_field.insert(owner.clone());
            }
        }
    }
    let adapters: Vec<&String> =
        implements.keys().filter(|ty| holds_a_field.contains(*ty)).collect();

    println!("\n── R8: derived from nodes and edges alone ──");
    println!("FACADE  types calling into other types: {}", facades.len());
    for (ty, reached) in facades.iter().take(5) {
        println!("  reaches {reached:>3} other types  {ty}");
    }
    println!("ADAPTER implements-and-holds: {}", adapters.len());
    for ty in adapters.iter().take(5) {
        println!("  {ty}");
    }

    assert!(
        !facades.is_empty(),
        "no Facade could be derived: call edges and member ownership are both emitted, so \
         either no type in this corpus calls another's members, or the two facts do not JOIN — \
         and the second would make every pattern that needs both underivable"
    );
    assert!(
        !adapters.is_empty(),
        "no Adapter could be derived: `implements` and field types are both emitted, so either \
         nothing here implements-and-holds, or the two facts do not join"
    );
}

/// **R5 / A4. A dependency is ours or somebody else's, and the graph says
/// which.**
///
/// Externality comes from the IMPORT and never from absence (§2, R6) — a symbol
/// missing from what has been scanned may simply not have been scanned yet, and
/// deciding on absence makes the answer depend on file order.
///
/// This reports the split a reader actually asks for: how much of what this
/// code reaches is first-party, and how much is a library it should not be
/// reading the internals of.
#[test]
#[ignore]
fn every_import_is_classified_as_ours_or_a_librarys() {
    let corpus = read_the_corpus();
    let mut local = 0usize;
    let mut external: BTreeMap<String, usize> = BTreeMap::new();
    let mut globs = 0usize;

    for read in &corpus {
        for import in &read.facts.imports {
            match &import.origin {
                super::facts::ImportOrigin::Local => local += 1,
                super::facts::ImportOrigin::External { package } => {
                    *external.entry(package.clone()).or_default() += 1;
                }
            }
            if import.binds == super::facts::Binding::Glob {
                globs += 1;
            }
        }
    }

    println!("\n── R5: internal vs external ──");
    println!("local imports    {local}");
    println!("external imports {}", external.values().sum::<usize>());
    println!("distinct external packages {}", external.len());
    let mut ranked: Vec<(&String, &usize)> = external.iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(a.1));
    for (package, n) in ranked.iter().take(12) {
        println!("  {n:>5}  {package}");
    }
    println!("globs (bind an unknown set) {globs}");

    assert!(
        local > 0 && !external.is_empty(),
        "a corpus with no imports of either kind is not one"
    );
    assert!(
        !external.contains_key(""),
        "an import was classified external with no package name, which would mint a library \
         node out of nothing"
    );
}

/// **Every member says which type owns it.** One language emitted none and
/// every test passed.
///
/// `RelationKind::Owns` is the vocabulary for "this type declares this member"
/// — the fact a member rung needs, and the one thing no `SymbolKind` filter can
/// stand in for. Rust and JavaScript emit it; Java shipped with ZERO and
/// nothing noticed, because no check compares the languages on it.
///
/// Counted per language for exactly that reason: a total would have read as
/// healthy while one column sat at nought.
#[test]
#[ignore]
fn every_type_owned_declaration_says_which_type_owns_it() {
    let corpus = read_the_corpus();
    let mut members: BTreeMap<&str, usize> = BTreeMap::new();
    let mut owns: BTreeMap<&str, usize> = BTreeMap::new();

    for read in &corpus {
        let language = read.facts.language.as_str();
        // A member is a declaration whose identity carries a TYPE segment —
        // read off the fqn rather than guessed from `SymbolKind`, because which
        // kinds are type-owned differs per language and that guess is what made
        // `first_party_members` unusable as an inventory.
        *members.entry(language).or_default() += read
            .facts
            .symbols
            .iter()
            .filter(|s| super::fqn::parse(s.fqn.as_str()).is_ok_and(|p| p.tail.len() > 1))
            .count();
        *owns.entry(language).or_default() +=
            read.facts.relations.iter().filter(|r| r.kind == RelationKind::Owns).count();
    }

    println!("\n## Owns, per language\n");
    for language in super::facts::Language::all() {
        let l = language.as_str();
        println!(
            "  {l:<12} members {:>7} | owns {:>7}",
            members.get(l).copied().unwrap_or(0),
            owns.get(l).copied().unwrap_or(0)
        );
    }

    for language in super::facts::Language::all() {
        let l = language.as_str();
        let declared = members.get(l).copied().unwrap_or(0);
        if declared == 0 {
            continue;
        }
        assert!(
            owns.get(l).copied().unwrap_or(0) > 0,
            "{l} declares {declared} type-owned members and emits NO Owns relation, so the \
             graph cannot say which type owns any of them"
        );
    }
}

/// **Two barriers, and every miss has to be explainable** — over THIS
/// repository's Rust and TypeScript.
///
/// The measurement itself lives in [`super::barrier`], because Java runs the
/// same two barriers over a corpus nobody here wrote and a second copy of a
/// measurement is not a second measurement. What stays here is the corpus: this
/// repository's own source, read and placed by [`read_the_corpus`].
#[test]
#[ignore]
fn every_source_node_is_reached_by_a_test_and_then_by_other_source() {
    let corpus = read_the_corpus();
    let units: Vec<barrier::Unit<'_>> = corpus
        .iter()
        .map(|read| barrier::Unit {
            path: read.path.as_str(),
            text: read.text.as_str(),
            facts: &read.facts,
        })
        .collect();
    let per = barrier::two_barriers(&units);
    assert!(per.values().map(|t| t.nodes).sum::<usize>() > 0, "no source nodes at all");
}
