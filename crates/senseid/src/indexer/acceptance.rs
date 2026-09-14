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

use super::facts::{FileFacts, RefKind, RelationKind, Resolution, SymbolKind};
use super::lang::{self, Source, TypeHomes};
use super::resolve::{World, resolve};

/// The directory a file's package is rooted at, from the file's own path.
///
/// Everything before `/src/`, or the first segment when there is none. Getting
/// this wrong is not a small error: `module_path` strips the root, so a root of
/// `.` left `crates/senseid/src/db/pg_store` standing as the module segment of
/// every identity that file declares — which collapsed four different `Row`
/// types onto one fqn and reported 1,111 identity collisions that were the
/// harness's own doing.
fn package_root_of(path: &str) -> &str {
    match path.find("/src/") {
        Some(at) => &path[..at],
        None => path.split('/').next().unwrap_or("."),
    }
}

/// One file's facts, with the package that owns it.
struct Read {
    package: String,
    path: String,
    facts: FileFacts,
}

/// Read the whole corpus — every language — through the adapter each extension
/// dispatches to.
///
/// The barrier is here: every file is walked once with no type table so the
/// type DECLARATIONS can be collected, then walked again with the table, which
/// is what makes a member's identity independent of which file came first (R6).
fn read_the_corpus() -> Vec<Read> {
    let mut sources: Vec<(String, String, String)> = Vec::new();
    for (abs, text) in super::corpus_rust_sources() {
        let package = super::package_of(&abs);
        let rel = super::workspace_relative(&abs);
        sources.push((package, rel, text));
    }
    for (rel, text) in super::corpus_web_sources() {
        // The three front ends are one package for this purpose. Their real
        // package names come from a manifest the corpus reader does not open,
        // and a wrong package would split one symbol into three.
        sources.push(("web".to_string(), rel, text));
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
                out.push(Read { package: package.clone(), path: path.clone(), facts });
            }
        }
        out
    };

    let first = read_all(&TypeHomes::unknown());
    let homes = TypeHomes::of(
        first.iter().flat_map(|r| r.facts.symbols.iter().map(|s| (r.package.as_str(), s))),
    );

    // AND THE LADDER. A walk states what it SAW; a target is placed by
    // resolution (R7), so a harness that reads without resolving sees almost
    // nothing resolved and would report every pattern as underivable. That is
    // what the first run of this file did.
    let first_party: BTreeSet<String> = sources.iter().map(|(p, _, _)| p.clone()).collect();
    let scanned = BTreeSet::new();
    let world = World { first_party: &first_party, scanned: &scanned };
    read_all(&homes)
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
        "Denylisted",
        "NoDeclaredType",
        "MacroExpansion",
        "UnhandledForm",
    ];

    let mut cells: BTreeMap<(&str, String), usize> = BTreeMap::new();
    let mut files: BTreeMap<&str, usize> = BTreeMap::new();
    let mut symbols: BTreeMap<&str, usize> = BTreeMap::new();
    let mut relations: BTreeMap<&str, usize> = BTreeMap::new();
    let mut imports_local: BTreeMap<&str, usize> = BTreeMap::new();
    let mut imports_external: BTreeMap<&str, usize> = BTreeMap::new();

    for read in &corpus {
        let language = read.facts.language.as_str();
        *files.entry(language).or_default() += 1;
        *symbols.entry(language).or_default() += read.facts.symbols.len();
        *relations.entry(language).or_default() += read.facts.relations.len();
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
                Resolution::Resolved(_) => "RESOLVED".to_string(),
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

    println!("\n## References, by reason\n");
    println!("{header}");
    println!("{rule}");
    row("RESOLVED", &|l| cells.get(&(l, "RESOLVED".to_string())).copied().unwrap_or(0));
    for reason in reasons {
        row(reason, &|l| cells.get(&(l, reason.to_string())).copied().unwrap_or(0));
    }

    let total: usize = cells.values().sum();
    assert!(total > 0, "the corpus produced no references at all");
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
        let bound: BTreeSet<&str> = read
            .facts
            .imports
            .iter()
            .filter_map(|i| match &i.binds {
                super::facts::Binding::Name(name) => Some(name.as_str()),
                super::facts::Binding::Glob => None,
            })
            .collect();
        if bound.is_empty() {
            continue;
        }
        for reference in &read.facts.references {
            let (name, resolved) = match &reference.target {
                Resolution::Resolved(_) => {
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
            .filter(|r| matches!(r.target, Resolution::Resolved(_)))
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
/// # 708 today, and they are TWO causes, not one
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
/// # What this number was before, and why it moved
///
/// 1,114 was reported, and 375 of those were ONE declaration emitted TWICE by
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
    const KNOWN: usize = 708;
    const KNOWN_RUST: usize = 13;
    const KNOWN_TYPESCRIPT: usize = 695;
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
                Resolution::Resolved(_) => entry.0 += 1,
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
                && let Resolution::Resolved(owner) = &relation.parent
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
            let Resolution::Resolved(target) = &reference.target else { continue };
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
            ) && let Resolution::Resolved(parent) = &relation.parent
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
