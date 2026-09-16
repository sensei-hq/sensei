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

use super::facts::{FileFacts, Language, Resolution, SymbolKind};

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
/// separate files under `src/test/java` and TypeScript's are `*.spec.ts`
/// siblings — both are answered by the path, and neither has an in-file
/// marker to look for.
fn inline_tests_begin(text: &str, language: Language) -> Option<u32> {
    let marker = match language {
        Language::Rust => "#[cfg(test)]",
        Language::TypeScript | Language::Java => return None,
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
    use crate::indexer::lang::{self, Source, TypeHomes};
    use crate::indexer::resolve::{World, members_declared_by, resolve};

    /// Walk and place a handful of in-memory files, the same two-pass way a
    /// corpus is read: every file walked once with no type table so the
    /// declarations can be collected, then again with it, then the ladder (R6).
    fn placed(files: &[(&str, &str)]) -> Vec<FileFacts> {
        let read_all = |types: &TypeHomes| -> Vec<FileFacts> {
            files
                .iter()
                .filter_map(|(path, text)| {
                    let ext = format!(".{}", path.rsplit('.').next().unwrap_or(""));
                    let adapter = lang::adapter_for_ext(&ext)?;
                    let source = Source { package: "unnamed", module: "", path, text };
                    adapter.read(&source, types).ok()
                })
                .collect()
        };
        let first = read_all(&TypeHomes::unknown());
        let homes = TypeHomes::of(
            first.iter().flat_map(|f| f.symbols.iter().map(|s| (f.package.as_str(), s))),
        );
        let anchored = read_all(&homes);
        let first_party: BTreeSet<String> = anchored.iter().map(|f| f.package.clone()).collect();
        let first_party_members: BTreeSet<String> = anchored
            .iter()
            .flat_map(|f| f.symbols.iter())
            .filter(|s| matches!(s.kind, SymbolKind::Method | SymbolKind::Field))
            .map(|s| s.name.clone())
            .collect();
        let declared_members = members_declared_by(anchored.iter());
        let scanned = BTreeSet::new();
        let world = World {
            first_party: &first_party,
            first_party_members: &first_party_members,
            declared_members: &declared_members,
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
