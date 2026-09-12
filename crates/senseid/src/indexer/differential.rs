//! Stage 10 S1/S2 — the cutover gate: what does v2 see that v1 did not, and
//! what did v1 see that v2 lost?
//!
//! Spec: `docs/spec/indexer/10-cutover.md`. This is a TOOL whose output a
//! person reads before deciding to cut over, not a test that passes or fails on
//! its own. What it must never do is let a difference go unclassified.
//!
//! # The comparison is on RESOLVED targets, not on reference counts (S2)
//!
//! v1 DROPS a reference it cannot resolve; v2 emits an `Unresolved` for every
//! one. So v2 will show far more references, and that increase is the fix
//! rather than a regression. Counting raw references would report the defect
//! being repaired as damage — which is why S2 names the comparison explicitly
//! and why this module never exposes a raw count as a verdict.
//!
//! # The grammars differ, and a naive set-diff reports everything as lost
//!
//! v2 added a trailing REACH segment that v1's fqns do not carry — measured
//! before the wipe, 194,355 of 194,376 stored fqns had none. So v1's
//! `rust·senseid·api·handle` and v2's `rust·senseid·api·handle·item` name the
//! same symbol in two spellings, and a straight set difference would call every
//! v1 symbol a regression and every v2 symbol an improvement at the same time:
//! a report that is 100% wrong while looking complete.
//!
//! [`identity_key`] is the one normalisation, and it is the ONLY one. Anything
//! else that differs is a real difference and is classified as such.

use std::collections::BTreeSet;

use crate::indexer::facts::{FileFacts, Resolution};
use crate::languages::fqn::FqnFileOutput;

/// What one difference means for the cutover (S1).
///
/// There is deliberately no `Unknown` that reads as harmless. A difference this
/// module cannot account for is [`Verdict::Unclassified`], and
/// [`DiffReport::blocks_cutover`] treats it exactly like a regression — because
/// "we do not know what this is" and "this is fine" are different states, and a
/// gate that conflates them is not a gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verdict {
    /// v2 resolves a target v1 did not. The point of the rewrite.
    Improvement,
    /// v1 resolved a target v2 does not. BLOCKS cutover (S1).
    Regression,
    /// A difference with a named, written reason.
    Explained,
    /// A difference no rule accounts for. Blocks, same as a regression.
    Unclassified,
}

/// One difference, with what it was and what it means.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Difference {
    pub verdict: Verdict,
    /// The normalised identity the difference is about.
    pub key: String,
    /// Why it is classified that way. Empty is not allowed for `Explained` —
    /// see [`DiffReport::blocks_cutover`].
    pub reason: String,
}

/// The gate's output.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiffReport {
    pub differences: Vec<Difference>,
    /// Resolved targets v1 produced, normalised. Reported as a COUNT of the
    /// compared set rather than of raw references, per S2.
    pub v1_resolved: usize,
    pub v2_resolved: usize,
}

impl DiffReport {
    pub fn count(&self, verdict: Verdict) -> usize {
        self.differences.iter().filter(|d| d.verdict == verdict).count()
    }

    /// Is v2's resolved set a SUPERSET of v1's (S2)?
    ///
    /// Stated as a set property and not as `v2_resolved >= v1_resolved`: two
    /// sets of equal size can each hold something the other does not, and a
    /// count comparison would call that a pass.
    pub fn v2_resolves_everything_v1_did(&self) -> bool {
        self.count(Verdict::Regression) == 0
    }

    /// The cutover gate. A regression blocks; so does anything unclassified,
    /// and so does an `Explained` with no written reason — an empty explanation
    /// is an unclassified difference wearing a label.
    pub fn blocks_cutover(&self) -> bool {
        self.differences.iter().any(|d| match d.verdict {
            Verdict::Regression | Verdict::Unclassified => true,
            Verdict::Explained => d.reason.trim().is_empty(),
            Verdict::Improvement => false,
        })
    }
}

/// An fqn reduced to what the two grammars AGREE on.
///
/// v2's trailing reach segment is dropped, because v1 has none and keeping it
/// would make every symbol differ. Nothing else is normalised: the language,
/// the package and the whole module/name path must match exactly, since those
/// are what identify the symbol in both grammars.
///
/// **The reach is recognised by its LABEL, not by position.** `fqn::parse`
/// cannot be used here: it implements the v2 grammar, in which the last segment
/// IS the reach, so handing it a v1 fqn strips the symbol's NAME instead and
/// two unrelated symbols in one module collapse onto one key. That is a silent
/// wrong answer of the worst kind — the harness would under-report both
/// regressions and improvements, and the totals would still look sensible.
///
/// Recognising the label is safe in the other direction too: `item`, `field`,
/// `macro` and `mod` are reserved trailing segments in v2's grammar, and a v1
/// fqn whose last segment happens to be one of those words would have to be a
/// symbol literally named `item` — for which the v2 spelling is
/// `…·item·item`, so the normalised keys still agree.
///
/// A string that parses as neither grammar is returned VERBATIM rather than
/// skipped: it is a real difference between the producers and must reach the
/// report. Dropping it would hide exactly the case where one side emits
/// something malformed.
pub fn identity_key(fqn: &str) -> String {
    match fqn.rsplit_once(crate::languages::fqn::SEP) {
        Some((head, last)) if crate::indexer::fqn::Reach::from_label(last).is_some() => {
            head.to_string()
        }
        _ => fqn.to_string(),
    }
}

/// Every resolved target v1 produced for one file, normalised.
fn v1_resolved_targets(v1: &FqnFileOutput) -> BTreeSet<String> {
    v1.refs.iter().filter_map(|r| r.target_fqn.as_deref()).map(identity_key).collect()
}

/// Every resolved target v2 produced for one file, normalised.
///
/// `Unresolved` is deliberately not counted. It is not a missing target — it is
/// v2 saying so, which is the behaviour v1 lacked, and folding it in here would
/// compare the two on a dimension only one of them has.
fn v2_resolved_targets(v2: &FileFacts) -> BTreeSet<String> {
    v2.references
        .iter()
        .filter_map(|r| match &r.target {
            Resolution::Resolved(fqn) => Some(identity_key(fqn.as_str())),
            Resolution::Unresolved { .. } => None,
        })
        .collect()
}

/// Compare one file's facts from both producers (S1/S2).
///
/// Every difference is classified. The classification rules are:
///
/// - resolved by v2 and not v1 -> IMPROVEMENT. This is the rewrite working.
/// - resolved by v1 and not v2 -> REGRESSION, unless a rule below explains it.
/// - an external (`lib·`) target v1 resolved -> EXPLAINED. v1 minted a `lib·`
///   node for any import it could not place first-party; v2 reaches externals
///   through the same prefix but only from a use site that names a package, so
///   the two sets legitimately differ in shape rather than in reach.
///
/// Anything else that differs is UNCLASSIFIED and blocks, which is what stops
/// this from becoming a rubber stamp.
pub fn differential(v1: &FqnFileOutput, v2: &FileFacts) -> DiffReport {
    let a = v1_resolved_targets(v1);
    let b = v2_resolved_targets(v2);

    let mut differences = Vec::new();
    for key in b.difference(&a) {
        differences.push(Difference {
            verdict: Verdict::Improvement,
            key: key.clone(),
            reason: "v2 resolves a target v1 did not".into(),
        });
    }
    for key in a.difference(&b) {
        let (verdict, reason) = if key.starts_with(crate::languages::fqn::LIB_PREFIX) {
            (
                Verdict::Explained,
                "an external target: v1 minted one for any unplaceable import, v2 mints one \
                 only where the use site names a package (D12)"
                    .to_string(),
            )
        } else {
            (Verdict::Regression, "v1 resolved this target and v2 does not".to_string())
        };
        differences.push(Difference { verdict, key: key.clone(), reason });
    }

    differences.sort();
    DiffReport { differences, v1_resolved: a.len(), v2_resolved: b.len() }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The normalisation, on the case that makes it necessary.
    ///
    /// Without it the two spellings of one symbol look like a regression AND an
    /// improvement simultaneously — a report that is entirely wrong while
    /// appearing complete.
    #[test]
    fn the_reach_segment_is_the_one_thing_normalised_away() {
        assert_eq!(
            identity_key("rust·senseid·api::handlers·handle·item"),
            identity_key("rust·senseid·api::handlers·handle"),
            "v1 and v2 spell one symbol two ways; the reach is the only difference"
        );
        // The language segment SURVIVES. Using `fqn::parse` here dropped it —
        // and worse, on a v1 fqn it stripped the NAME, collapsing every symbol
        // in a module onto one key while the totals still looked sensible.
        assert_eq!(identity_key("rust·senseid·api·handle·item"), "rust·senseid·api·handle");
        // Two symbols in one module stay two keys. This is the assertion the
        // `parse`-based version failed.
        assert_ne!(identity_key("rust·senseid·api·handle"), identity_key("rust·senseid·api·other"),);
        // And nothing else collapses: a different module is a different symbol.
        assert_ne!(
            identity_key("rust·senseid·api·handle·item"),
            identity_key("rust·senseid·db·handle·item"),
        );
        // The language is not part of the key because both producers agree on
        // it per file, but the PACKAGE is, and it separates.
        assert_ne!(
            identity_key("rust·senseid·api·handle·item"),
            identity_key("rust·cli·api·handle·item"),
        );
    }

    /// An unparseable fqn reaches the report rather than being skipped.
    #[test]
    fn an_fqn_the_parser_rejects_is_kept_verbatim_not_dropped() {
        assert_eq!(identity_key("not an fqn at all"), "not an fqn at all");
    }

    /// S1. Nothing may default to harmless.
    #[test]
    fn an_unclassified_difference_blocks_the_cutover() {
        let report = DiffReport {
            differences: vec![Difference {
                verdict: Verdict::Unclassified,
                key: "x".into(),
                reason: "no rule matched".into(),
            }],
            ..Default::default()
        };
        assert!(
            report.blocks_cutover(),
            "`we do not know what this is` is not `this is fine` — a gate that conflates them \
             is not a gate"
        );
    }

    /// And neither may an EXPLAINED with nothing written in it.
    #[test]
    fn an_explanation_that_explains_nothing_blocks_too() {
        let report = DiffReport {
            differences: vec![Difference {
                verdict: Verdict::Explained,
                key: "x".into(),
                reason: "   ".into(),
            }],
            ..Default::default()
        };
        assert!(report.blocks_cutover(), "an empty explanation is unclassified wearing a label");
    }

    /// Improvements alone never block — that is the whole point of cutting over.
    #[test]
    fn improvements_do_not_block() {
        let report = DiffReport {
            differences: vec![Difference {
                verdict: Verdict::Improvement,
                key: "x".into(),
                reason: "v2 resolves it".into(),
            }],
            ..Default::default()
        };
        assert!(!report.blocks_cutover());
        assert!(report.v2_resolves_everything_v1_did());
    }

    /// S2, stated as a SET property.
    ///
    /// Two sets of equal size can each hold something the other does not, so a
    /// count comparison would pass this. The superset question is about
    /// membership and is answered by the regression count.
    #[test]
    fn equal_counts_do_not_make_v2_a_superset() {
        let report = DiffReport {
            differences: vec![
                Difference { verdict: Verdict::Improvement, key: "a".into(), reason: "new".into() },
                Difference { verdict: Verdict::Regression, key: "b".into(), reason: "lost".into() },
            ],
            v1_resolved: 10,
            v2_resolved: 10,
        };
        assert!(
            !report.v2_resolves_everything_v1_did(),
            "same size, different members — the counts agreeing proves nothing"
        );
        assert!(report.blocks_cutover());
    }
}

/// The gate, run over this repository's own Rust.
///
/// `#[ignore]`: it parses the whole corpus twice, which is minutes rather than
/// milliseconds. It is a TOOL a person runs before deciding to cut over (S1),
/// not a test that gates every commit — and it prints its report rather than
/// asserting a number, because the number is what the person is reading.
///
/// Run it with:
///
/// ```text
/// cargo test -p senseid --bin senseid -- --ignored --nocapture \
///   indexer::differential::corpus::v1_and_v2_over_this_repos_rust
/// ```
#[cfg(test)]
mod corpus {
    use super::*;
    use crate::indexer::lang::rust::{self, Source};
    use crate::indexer::resolve::{World, resolve};
    use crate::languages::LanguageAdapter;

    #[tokio::test]
    #[ignore]
    async fn v1_and_v2_over_this_repos_rust() {
        let sources = crate::indexer::corpus_rust_sources();
        let first_party: std::collections::BTreeSet<String> =
            sources.iter().map(|(path, _)| crate::indexer::package_of(path)).collect();
        let scanned = std::collections::BTreeSet::new();
        let world = World { first_party: &first_party, scanned: &scanned };

        let mut totals = DiffReport::default();
        let mut files_compared = 0usize;
        let mut v1_produced_nothing = 0usize;
        let mut v2_failed_to_read = 0usize;
        let mut v2_declined: std::collections::BTreeMap<String, usize> = Default::default();
        let mut v2_unresolved_names: Vec<(String, String)> = Vec::new();

        for (abs, text) in &sources {
            // v1: the shipped producer, from the absolute path (it walks up to
            // the manifest itself).
            let Some(v1) = crate::languages::rust_lang::RustAdapter.fqn_output(abs, "", text)
            else {
                // v1 declining a file is itself a fact — COUNTED, not skipped,
                // because "v1 produced nothing here" is precisely the case where
                // v2 looking better means nothing.
                v1_produced_nothing += 1;
                continue;
            };

            // v2: the same file, at the grain a real walk produces.
            let package = crate::indexer::package_of(abs);
            let rel = crate::indexer::workspace_relative(abs);
            let module = crate::indexer::module_of(&rel);
            let Ok(facts) =
                rust::read(&Source { package: &package, module: &module, path: &rel, text })
            else {
                v2_failed_to_read += 1;
                continue;
            };
            let v2 = resolve(facts, &rust::GRAMMAR, &world);

            // Every NAME v2 saw at a use site but declined to place. If a v1
            // "resolution" turns up here, v2 did not lose the reference — it
            // saw it and refused to guess, which R4 says is the right answer
            // and is the opposite of a regression.
            for r in &v2.references {
                if let crate::indexer::facts::Resolution::Unresolved { reason, .. } = &r.target {
                    *v2_declined.entry(format!("{reason:?}")).or_insert(0usize) += 1;
                }
            }
            v2_unresolved_names.extend(v2.references.iter().filter_map(|r| match &r.target {
                crate::indexer::facts::Resolution::Unresolved { reason, evidence } => {
                    Some((evidence.name.clone(), format!("{reason:?}")))
                }
                _ => None,
            }));

            let report = differential(&v1, &v2);
            totals.v1_resolved += report.v1_resolved;
            totals.v2_resolved += report.v2_resolved;
            totals.differences.extend(report.differences);
            files_compared += 1;
        }

        let regressions = totals.count(Verdict::Regression);
        let improvements = totals.count(Verdict::Improvement);
        let explained = totals.count(Verdict::Explained);
        let unclassified = totals.count(Verdict::Unclassified);

        println!("\n── v1 vs v2 over {files_compared} rust files ──");
        println!("v1 produced no output   {v1_produced_nothing}");
        println!("v2 could not read       {v2_failed_to_read}");
        println!("resolved targets  v1    {}", totals.v1_resolved);
        println!("resolved targets  v2    {}", totals.v2_resolved);
        println!("IMPROVEMENT             {improvements}");
        println!("REGRESSION              {regressions}");
        println!("EXPLAINED               {explained}");
        println!("UNCLASSIFIED            {unclassified}");
        println!("blocks cutover          {}", totals.blocks_cutover());
        println!("\nv2 UNRESOLVED by reason (references v1 would simply have dropped):");
        for (reason, n) in &v2_declined {
            println!("  {n:>6}  {reason}");
        }
        println!("  (total unresolved: {})", v2_unresolved_names.len());

        // THE DECIDING MEASUREMENT for the 777. For each regression, did v2 SEE
        // that name at a use site and decline to place it? If so v2 did not lose
        // the reference — it refused to guess, which R4 says is correct and is
        // the opposite of a regression.
        let declined_by_name: std::collections::BTreeMap<&str, &str> =
            v2_unresolved_names.iter().map(|(n, r)| (n.as_str(), r.as_str())).collect();
        let mut seen_and_declined: std::collections::BTreeMap<&str, usize> = Default::default();
        let mut never_seen = 0usize;
        for d in totals.differences.iter().filter(|d| d.verdict == Verdict::Regression) {
            let leaf = d.key.rsplit('\u{00B7}').next().unwrap_or("");
            match declined_by_name.get(leaf) {
                Some(reason) => *seen_and_declined.entry(reason).or_default() += 1,
                None => never_seen += 1,
            }
        }
        println!("\nof {regressions} regressions, v2 SAW the name and declined:");
        for (reason, n) in &seen_and_declined {
            println!("  {n:>6}  {reason}");
        }
        println!("  {never_seen:>6}  name never appears at any v2 use site");

        // A sample of each blocking class, so the number has something
        // inspectable behind it. "N regressions" and "these N regressions" are
        // different claims, and only the second can be acted on.
        // Is a "regression" actually the same symbol under a different MODULE
        // segment? Measured rather than assumed: take each regression's package
        // and its last two segments, and look for an improvement that matches.
        // If most do, the two producers disagree about the module path and not
        // about whether the target resolves — a different finding entirely.
        let tail_key = |k: &str| -> String {
            let segs: Vec<&str> = k.split('\u{00B7}').collect();
            match segs.len() {
                0 | 1 => k.to_string(),
                n => format!(
                    "{}|{}",
                    segs.get(1).copied().unwrap_or(""),
                    segs[n - 2..].join("\u{00B7}")
                ),
            }
        };
        let improved_tails: std::collections::BTreeSet<String> = totals
            .differences
            .iter()
            .filter(|d| d.verdict == Verdict::Improvement)
            .map(|d| tail_key(&d.key))
            .collect();
        let same_symbol_different_module = totals
            .differences
            .iter()
            .filter(|d| d.verdict == Verdict::Regression)
            .filter(|d| improved_tails.contains(&tail_key(&d.key)))
            .count();
        println!(
            "of {regressions} regressions, {same_symbol_different_module} match an IMPROVEMENT on \
             (package, last two segments) — i.e. the same symbol under a different module path"
        );

        for (label, verdict) in
            [("regression", Verdict::Regression), ("unclassified", Verdict::Unclassified)]
        {
            let sample: Vec<&str> = totals
                .differences
                .iter()
                .filter(|d| d.verdict == verdict)
                .map(|d| d.key.as_str())
                .take(15)
                .collect();
            if !sample.is_empty() {
                println!("\nfirst {label}s: {sample:#?}");
            }
        }

        assert!(
            files_compared > 100,
            "the gate compared {files_compared} files, which is not a corpus"
        );
    }
}

/// WHY the gate blocks — the investigation behind the 777, with source.
///
/// The gate says "v1 resolved this and v2 does not". That is a true statement
/// and an insufficient one, because it does not say whether v1 was RIGHT. This
/// answers that, by asking of every disputed target the one question that
/// settles it:
///
/// **Does the thing v1 pointed at exist?**
///
/// Three outcomes, and they mean opposite things:
///
/// - the target exists in NEITHER producer's definitions -> v1 resolved to a
///   GHOST. Nothing in the corpus declares it. v1's edge pointed at a node
///   minted only because a reference asked for it, and R4 ranks that below no
///   edge at all. Not a regression.
/// - it exists in v1's definitions but not v2's -> the two DISAGREE ON IDENTITY,
///   not on reach. A minting difference, which the normalisation should have
///   covered and did not.
/// - it exists in BOTH -> v2 HAS the definition and still did not connect the
///   reference to it. The only class that is a real loss of reach.
///
/// `#[ignore]` for the same reason as the gate: it parses the corpus twice.
#[cfg(test)]
mod why {
    use super::*;
    use crate::indexer::lang::rust::{self, Source};
    use crate::indexer::resolve::{World, resolve};
    use crate::languages::LanguageAdapter;

    /// One disputed target, with everything needed to judge it by hand.
    struct Disputed {
        key: String,
        file: String,
        reason: String,
        /// What v1 and v2 each DECLARE under this member name, so the two
        /// spellings can be read side by side instead of inferred.
        v1_declares: Vec<String>,
        v2_declares: Vec<String>,
        /// The use site's line, and the source text of it.
        line: u32,
        source_line: String,
        /// The bare name the use site saw.
        saw: String,
    }

    #[tokio::test]
    #[ignore]
    async fn what_the_disputed_targets_actually_are() {
        let sources = crate::indexer::corpus_rust_sources();
        let first_party: std::collections::BTreeSet<String> =
            sources.iter().map(|(path, _)| crate::indexer::package_of(path)).collect();
        let scanned = std::collections::BTreeSet::new();
        let world = World { first_party: &first_party, scanned: &scanned };

        // Pass 1: every DEFINITION both producers mint, normalised. This is what
        // "does the target exist?" is asked against.
        let mut v1_defs: std::collections::BTreeSet<String> = Default::default();
        let mut v2_defs: std::collections::BTreeSet<String> = Default::default();
        let mut parsed: Vec<(String, String, crate::indexer::facts::FileFacts)> = Vec::new();
        let mut v1_by_file: std::collections::BTreeMap<
            String,
            crate::languages::fqn::FqnFileOutput,
        > = Default::default();

        for (abs, text) in &sources {
            let Some(v1) = crate::languages::rust_lang::RustAdapter.fqn_output(abs, "", text)
            else {
                continue;
            };
            v1_defs.extend(v1.defs.iter().map(|d| identity_key(&d.fqn)));

            let package = crate::indexer::package_of(abs);
            let rel = crate::indexer::workspace_relative(abs);
            let module = crate::indexer::module_of(&rel);
            let Ok(facts) =
                rust::read(&Source { package: &package, module: &module, path: &rel, text })
            else {
                continue;
            };
            let facts = resolve(facts, &rust::GRAMMAR, &world);
            v2_defs.extend(facts.symbols.iter().map(|s| identity_key(s.fqn.as_str())));
            v1_by_file.insert(rel.clone(), v1);
            parsed.push((rel, text.clone(), facts));
        }

        // Pass 2: for every disputed target, classify it by whether it EXISTS.
        let mut ghost = Vec::new();
        let mut trait_qualified = Vec::new();
        let mut identity_disagreement = Vec::new();
        let mut real_loss = Vec::new();

        for (rel, text, v2) in &parsed {
            let Some(v1) = v1_by_file.get(rel) else { continue };
            let report = differential(v1, v2);

            for d in report.differences.iter().filter(|d| d.verdict == Verdict::Regression) {
                // The use site v2 declined, matched by the bare name.
                let leaf = d.key.rsplit('\u{00B7}').next().unwrap_or("").to_string();
                let declined = v2.references.iter().find_map(|r| match &r.target {
                    crate::indexer::facts::Resolution::Unresolved { reason, evidence }
                        if evidence.name == leaf =>
                    {
                        Some((format!("{reason:?}"), evidence.name.clone(), r.at.start_line))
                    }
                    _ => None,
                });
                let (reason, saw, line) = declined.unwrap_or_else(|| {
                    ("(v2 emitted no reference for this name)".into(), leaf.clone(), 0)
                });
                // Line 0 means NO use site was matched, so there is no source
                // to show. Printing line 1 there — which an earlier version of
                // this did — puts a file's opening comment under a finding it
                // has nothing to do with, and a reader would reasonably believe
                // the two were related.
                let source_line = if line == 0 {
                    "(no use site matched this name in v2's references)".to_string()
                } else {
                    text.lines()
                        .nth(line as usize - 1)
                        .unwrap_or("")
                        .trim()
                        .chars()
                        .take(110)
                        .collect::<String>()
                };

                // Match on the LAST TWO segments (`Type·member`), not the
                // member alone. `·failed` alone matches every `failed` in the
                // workspace, and a `.take(3)` over an alphabetical set then
                // shows three unrelated ones while hiding the relevant
                // declaration — a truncated search presented as an answer.
                let tail = {
                    let segs: Vec<&str> = d.key.split('\u{00B7}').collect();
                    if segs.len() >= 2 {
                        format!("\u{00B7}{}", segs[segs.len() - 2..].join("\u{00B7}"))
                    } else {
                        format!("\u{00B7}{leaf}")
                    }
                };
                let declares = |defs: &std::collections::BTreeSet<String>| -> Vec<String> {
                    let hits: Vec<String> =
                        defs.iter().filter(|k| k.ends_with(&tail)).cloned().collect();
                    if hits.is_empty() {
                        // Fall back to the member alone, and SAY that is what
                        // happened — "no declaration of Type::member" and "no
                        // declaration of member anywhere" are different facts.
                        let member = format!("\u{00B7}{leaf}");
                        let loose: Vec<String> =
                            defs.iter().filter(|k| k.ends_with(&member)).take(3).cloned().collect();
                        if loose.is_empty() {
                            vec![format!("(nothing declares ·{leaf})")]
                        } else {
                            std::iter::once(format!("(no ...{tail}; other ·{leaf}:)"))
                                .chain(loose)
                                .collect()
                        }
                    } else {
                        hits.into_iter().take(3).collect()
                    }
                };
                let item = Disputed {
                    key: d.key.clone(),
                    file: rel.clone(),
                    reason,
                    v1_declares: declares(&v1_defs),
                    v2_declares: declares(&v2_defs),
                    line,
                    source_line,
                    saw,
                };
                match (v1_defs.contains(&d.key), v2_defs.contains(&d.key)) {
                    (_, true) => real_loss.push(item),
                    (true, false) => identity_disagreement.push(item),
                    (false, false) => {
                        // Is the SAME member declared on the SAME type under a
                        // LONGER identity? v1's grammar qualifies a trait-impl
                        // method with its trait (`…·Type·Trait·member`) while a
                        // call site cannot know which trait, so it mints
                        // `…·Type·member`. If a longer key exists, v1's own two
                        // sides disagree, and the "ghost" is that disagreement
                        // rather than a missing declaration.
                        let sep = '\u{00B7}';
                        let qualified = d.key.rsplit_once(sep).is_some_and(|(head, member)| {
                            let head = format!("{head}{sep}");
                            let member = format!("{sep}{member}");
                            v1_defs.iter().any(|k| k.starts_with(&head) && k.ends_with(&member))
                        });
                        if qualified {
                            trait_qualified.push(item);
                        } else {
                            ghost.push(item);
                        }
                    }
                }
            }
        }

        let show = |label: &str, items: &[Disputed], n: usize| {
            println!("\n\n══ {label}: {} ══", items.len());
            for d in items.iter().take(n) {
                println!("\n  target v1 claimed : {}", d.key);
                println!("  use site          : {}:{}", d.file, d.line);
                println!("  source            : {}", d.source_line);
                println!("  v2 saw the name   : {}", d.saw);
                println!("  v2 declined because: {}", d.reason);
                println!("  v1 DECLARES       : {:?}", d.v1_declares);
                println!("  v2 DECLARES       : {:?}", d.v2_declares);
            }
        };

        println!("\n\n════════ WHY THE GATE BLOCKS ════════");
        println!("v1 definitions {} | v2 definitions {}", v1_defs.len(), v2_defs.len());
        show(
            "V1 DISAGREES WITH ITSELF — its definition side spells this member with a trait \
             qualifier and its reference side without one, so the edge points at an identity \
             v1's own parser never declares",
            &trait_qualified,
            5,
        );
        show("GHOST — nothing declares this under ANY identity", &ghost, 5);
        show(
            "IDENTITY DISAGREEMENT — v1 declares it, v2 mints the declaration differently",
            &identity_disagreement,
            6,
        );
        show(
            "REAL LOSS — v2 HAS the definition and still did not connect the reference",
            &real_loss,
            8,
        );

        // What v2 SAID, per class. This is the number that says what one fix
        // would buy: a class dominated by a single reason has a single cause.
        fn by_reason(items: &[Disputed]) -> std::collections::BTreeMap<&str, usize> {
            let mut m: std::collections::BTreeMap<&str, usize> = Default::default();
            for d in items {
                *m.entry(d.reason.as_str()).or_default() += 1;
            }
            m
        }
        for (label, items) in [
            ("v1 self-disagreement", &trait_qualified),
            ("ghost", &ghost),
            ("identity disagreement", &identity_disagreement),
            ("REAL LOSS", &real_loss),
        ] {
            println!("\n{label} — v2's reason:");
            for (reason, n) in by_reason(items) {
                println!("  {n:>5}  {reason}");
            }
        }

        println!("\n\n──── verdict ────");
        println!("v1 self-disagreement   {}", trait_qualified.len());
        println!("ghost, no declaration  {}", ghost.len());
        println!("identity disagreement  {}", identity_disagreement.len());
        println!("REAL LOSS OF REACH     {}", real_loss.len());
    }
}
