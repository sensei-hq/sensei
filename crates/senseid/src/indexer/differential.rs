//! Stage 10 S1/S2 — the cutover gate: what does this indexer see that the
//! legacy one did not, and what did the legacy one see that this one lost?
//!
//! `legacy` throughout is `crate::languages`, the shipped producer; `current`
//! is this indexer. Naming them by role rather than by a version number is
//! deliberate — only one of them survives cutover.
//!
//! Spec: `docs/spec/indexer/10-cutover.md`. This is a TOOL whose output a
//! person reads before deciding to cut over, not a test that passes or fails on
//! its own. What it must never do is let a difference go unclassified.
//!
//! # The comparison is on RESOLVED targets, not on reference counts (S2)
//!
//! `legacy` DROPS a reference it cannot resolve; `current` emits an `Unresolved`
//! for every one. So `current` shows far more references, and that increase is the fix
//! rather than a regression. Counting raw references would report the defect
//! being repaired as damage — which is why S2 names the comparison explicitly
//! and why this module never exposes a raw count as a verdict.
//!
//! # The grammars differ, and a naive set-diff reports everything as lost
//!
//! `current` added a trailing REACH segment that `legacy`'s fqns do not carry —
//! measured before the wipe, 194,355 of 194,376 stored fqns had none. So
//! `legacy`'s `rust·senseid·api·handle` and `current`'s
//! `rust·senseid·api·handle·item` name the same symbol in two spellings, and a
//! straight set difference would call every `legacy` symbol a regression and
//! every `current` symbol an improvement at the same time:
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
    /// current resolves a target legacy did not. The point of the rewrite.
    Improvement,
    /// legacy resolved a target current does not. BLOCKS cutover (S1).
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
    /// Resolved targets legacy produced, normalised. Reported as a COUNT of the
    /// compared set rather than of raw references, per S2.
    pub legacy_resolved: usize,
    pub current_resolved: usize,
}

impl DiffReport {
    pub fn count(&self, verdict: Verdict) -> usize {
        self.differences.iter().filter(|d| d.verdict == verdict).count()
    }

    /// Is current's resolved set a SUPERSET of legacy's (S2)?
    ///
    /// Stated as a set property and not as `current_resolved >= legacy_resolved`: two
    /// sets of equal size can each hold something the other does not, and a
    /// count comparison would call that a pass.
    pub fn current_resolves_everything_legacy_did(&self) -> bool {
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
/// current's trailing reach segment is dropped, because legacy has none and keeping it
/// would make every symbol differ. Nothing else is normalised: the language,
/// the package and the whole module/name path must match exactly, since those
/// are what identify the symbol in both grammars.
///
/// **The reach is recognised by its LABEL, not by position.** `fqn::parse`
/// cannot be used here: it implements the current grammar, in which the last segment
/// IS the reach, so handing it a legacy fqn strips the symbol's NAME instead and
/// two unrelated symbols in one module collapse onto one key. That is a silent
/// wrong answer of the worst kind — the harness would under-report both
/// regressions and improvements, and the totals would still look sensible.
///
/// Recognising the label is safe in the other direction too: `item`, `field`,
/// `macro` and `mod` are reserved trailing segments in current's grammar, and a legacy
/// fqn whose last segment happens to be one of those words would have to be a
/// symbol literally named `item` — for which the current spelling is
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

/// Every resolved target legacy produced for one file, normalised.
fn legacy_resolved_targets(legacy: &FqnFileOutput) -> BTreeSet<String> {
    legacy.refs.iter().filter_map(|r| r.target_fqn.as_deref()).map(identity_key).collect()
}

/// Every resolved target current produced for one file, normalised.
///
/// `Unresolved` is deliberately not counted. It is not a missing target — it is
/// current saying so, which is the behaviour legacy lacked, and folding it in here would
/// compare the two on a dimension only one of them has.
fn current_resolved_targets(current: &FileFacts) -> BTreeSet<String> {
    current
        .references
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
/// - resolved by current and not legacy -> IMPROVEMENT. This is the rewrite working.
/// - resolved by legacy and not current -> REGRESSION, unless a rule below explains it.
/// - an external (`lib·`) target legacy resolved -> EXPLAINED. legacy minted a `lib·`
///   node for any import it could not place first-party; current reaches externals
///   through the same prefix but only from a use site that names a package, so
///   the two sets legitimately differ in shape rather than in reach.
///
/// Anything else that differs is UNCLASSIFIED and blocks, which is what stops
/// this from becoming a rubber stamp.
pub fn differential(legacy: &FqnFileOutput, current: &FileFacts) -> DiffReport {
    let a = legacy_resolved_targets(legacy);
    let b = current_resolved_targets(current);

    let mut differences = Vec::new();
    for key in b.difference(&a) {
        differences.push(Difference {
            verdict: Verdict::Improvement,
            key: key.clone(),
            reason: "current resolves a target legacy did not".into(),
        });
    }
    for key in a.difference(&b) {
        let (verdict, reason) = if key.starts_with(crate::languages::fqn::LIB_PREFIX) {
            (
                Verdict::Explained,
                "an external target: legacy minted one for any unplaceable import, current mints one \
                 only where the use site names a package (D12)"
                    .to_string(),
            )
        } else {
            (Verdict::Regression, "legacy resolved this target and current does not".to_string())
        };
        differences.push(Difference { verdict, key: key.clone(), reason });
    }

    differences.sort();
    DiffReport { differences, legacy_resolved: a.len(), current_resolved: b.len() }
}

/// What a disputed target IS, once BOTH producers' declarations are in hand.
///
/// The gate cannot answer this per file. "Does anything in the corpus declare
/// the thing legacy pointed at" is a question about the whole scan, so it is a
/// second stage over the collected differences rather than another arm inside
/// [`differential`] — which stays pure and per-file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disputed {
    /// Both producers declare it and current still did not connect the
    /// reference. The ONLY class that is a loss of reach.
    RealLoss,
    /// legacy declares it, current does not. The two grammars MINT it
    /// differently — this indexer's problem, and it must keep blocking.
    IdentityDisagreement,
    /// Neither declares this spelling, but legacy declares a LONGER one for the
    /// same member: its grammar qualifies a trait-impl method with the trait
    /// while a call site cannot know which trait. legacy's own two sides
    /// disagree.
    LegacySelfDisagreement,
    /// Nothing anywhere declares it. legacy resolved to a node that exists only
    /// because a reference asked for it.
    Ghost,
}

/// Classify one disputed target. ONE owner, because the gate and the `why`
/// report both ask it and two implementations would drift into two answers to
/// the question the cutover decision rests on.
pub fn classify(
    key: &str,
    legacy_defs: &BTreeSet<String>,
    current_defs: &BTreeSet<String>,
) -> Disputed {
    match (legacy_defs.contains(key), current_defs.contains(key)) {
        (_, true) => Disputed::RealLoss,
        (true, false) => Disputed::IdentityDisagreement,
        (false, false) => {
            // Is the SAME member declared on the SAME type under a LONGER
            // identity? If a longer key exists, the "ghost" is legacy
            // disagreeing with itself rather than a missing declaration.
            let sep = '\u{00B7}';
            let qualified = key.rsplit_once(sep).is_some_and(|(head, member)| {
                let head = format!("{head}{sep}");
                let member = format!("{sep}{member}");
                legacy_defs.iter().any(|k| k.starts_with(&head) && k.ends_with(&member))
            });
            if qualified { Disputed::LegacySelfDisagreement } else { Disputed::Ghost }
        }
    }
}

/// Reclassify the regressions that are legacy being WRONG rather than this
/// indexer losing reach (S1).
///
/// The gate said "legacy resolved this and current does not", which is true and
/// insufficient — it never said whether legacy was RIGHT. Measured over this
/// repo, most of the time it was not: legacy's reference side derives a
/// target's module from the CALL SITE, so `CheckOutcome::ready` became
/// `…·checker·CheckOutcome·ready` and a type used inside `mod tests` gained the
/// test module. Those targets name nothing that exists.
///
/// R4 already settles what such an edge is worth — a wrong edge is worse than a
/// missing one — so declining to mint it is the rewrite working. Two classes
/// flip, and the two that do NOT are the point:
///
/// - [`Disputed::IdentityDisagreement`] stays a regression. legacy declares it
///   and this indexer mints it differently; folding that into `Explained` would
///   hide a grammar difference behind a rule about legacy's mistakes.
/// - [`Disputed::RealLoss`] stays a regression, obviously.
///
/// Every flip carries the reason that fired, because `blocks_cutover` treats an
/// `Explained` with no written reason as unclassified.
pub fn explain_dangling(
    report: &mut DiffReport,
    legacy_defs: &BTreeSet<String>,
    current_defs: &BTreeSet<String>,
) {
    for difference in &mut report.differences {
        if difference.verdict != Verdict::Regression {
            continue;
        }
        let (verdict, reason) = match classify(&difference.key, legacy_defs, current_defs) {
            Disputed::Ghost => (
                Verdict::Explained,
                "NOTHING declares this: legacy resolved to a node that exists only because a \
                 reference asked for it, and R4 ranks that below no edge at all",
            ),
            Disputed::LegacySelfDisagreement => (
                Verdict::Explained,
                "legacy DECLARES a longer identity for this member and its reference side mints \
                 a shorter one — its own two passes disagreeing, not reach lost here",
            ),
            Disputed::IdentityDisagreement | Disputed::RealLoss => continue,
        };
        difference.verdict = verdict;
        difference.reason = reason.to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn declared(keys: &[&str]) -> BTreeSet<String> {
        keys.iter().map(|k| (*k).to_string()).collect()
    }

    fn regression(key: &str) -> Difference {
        Difference {
            verdict: Verdict::Regression,
            key: key.to_string(),
            reason: "legacy resolved this target and current does not".to_string(),
        }
    }

    /// The rule the gate was missing, and the reason it blocked on a number
    /// that was mostly not about this indexer.
    ///
    /// A target NO producer declares is one legacy resolved to a node that
    /// exists only because a reference asked for it. R4 already settles what
    /// that is worth: a wrong edge is worse than a missing one, so declining to
    /// mint it is the rewrite working, not reach lost.
    ///
    /// MUTATION that must break this: explain every regression. The
    /// identity-disagreement case below then goes quiet, and a real minting
    /// difference between the two grammars stops blocking.
    #[test]
    fn a_target_no_producer_declares_is_explained_and_a_minting_difference_is_not() {
        let legacy = declared(&[
            "rust·p·m·Widget·draw",
            // The trait-qualified form legacy's DEFINITION side mints while its
            // reference side mints the unqualified one.
            "rust·p·m·Widget·Draw·render",
            "rust·p·m·OnlyLegacy·spelled",
        ]);
        let current = declared(&["rust·p·m·Widget·draw"]);

        let mut report = DiffReport {
            differences: vec![
                regression("rust·p·m·Widget·draw"),
                regression("rust·p·m·OnlyLegacy·spelled"),
                regression("rust·p·m·Widget·render"),
                regression("rust·p·m·Ghost·nothing_declares_this"),
            ],
            legacy_resolved: 4,
            current_resolved: 1,
        };
        explain_dangling(&mut report, &legacy, &current);

        let verdict = |key: &str| {
            report.differences.iter().find(|d| d.key == key).expect("the difference is there")
        };

        assert_eq!(
            verdict("rust·p·m·Ghost·nothing_declares_this").verdict,
            Verdict::Explained,
            "nothing declares it, so legacy resolved to a node a reference invented"
        );
        assert_eq!(
            verdict("rust·p·m·Widget·render").verdict,
            Verdict::Explained,
            "legacy DECLARES the trait-qualified form, so its own two sides disagree"
        );
        assert_eq!(
            verdict("rust·p·m·OnlyLegacy·spelled").verdict,
            Verdict::Regression,
            "legacy declares it and this indexer does not: a MINTING difference, which is \
             this indexer's bug and must keep blocking"
        );
        assert_eq!(
            verdict("rust·p·m·Widget·draw").verdict,
            Verdict::Regression,
            "both declare it and the reference was not connected: a real loss of reach"
        );

        for difference in &report.differences {
            if difference.verdict == Verdict::Explained {
                assert!(
                    !difference.reason.trim().is_empty(),
                    "{} was explained with no reason, which `blocks_cutover` treats as \
                     unclassified anyway",
                    difference.key
                );
            }
        }
    }

    /// The rule is applied to REGRESSIONS only. An improvement that happens to
    /// name something nothing declares is a different fact and this must not
    /// touch it.
    #[test]
    fn explaining_a_dangling_target_leaves_every_other_verdict_alone() {
        let mut report = DiffReport {
            differences: vec![
                Difference {
                    verdict: Verdict::Improvement,
                    key: "rust·p·m·Ghost·nothing_declares_this".to_string(),
                    reason: "current resolves a target legacy did not".to_string(),
                },
                Difference {
                    verdict: Verdict::Explained,
                    key: "lib·serde·Serialize".to_string(),
                    reason: "an external target".to_string(),
                },
            ],
            legacy_resolved: 0,
            current_resolved: 1,
        };
        let before = report.clone();
        explain_dangling(&mut report, &declared(&[]), &declared(&[]));
        assert_eq!(report, before, "only a regression is reclassified");
    }

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
            "legacy and current spell one symbol two ways; the reach is the only difference"
        );
        // The language segment SURVIVES. Using `fqn::parse` here dropped it —
        // and worse, on a legacy fqn it stripped the NAME, collapsing every symbol
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
                reason: "current resolves it".into(),
            }],
            ..Default::default()
        };
        assert!(!report.blocks_cutover());
        assert!(report.current_resolves_everything_legacy_did());
    }

    /// S2, stated as a SET property.
    ///
    /// Two sets of equal size can each hold something the other does not, so a
    /// count comparison would pass this. The superset question is about
    /// membership and is answered by the regression count.
    #[test]
    fn equal_counts_do_not_make_current_a_superset() {
        let report = DiffReport {
            differences: vec![
                Difference { verdict: Verdict::Improvement, key: "a".into(), reason: "new".into() },
                Difference { verdict: Verdict::Regression, key: "b".into(), reason: "lost".into() },
            ],
            legacy_resolved: 10,
            current_resolved: 10,
        };
        assert!(
            !report.current_resolves_everything_legacy_did(),
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
///   indexer::differential::corpus::legacy_and_current_over_this_repos_rust
/// ```
#[cfg(test)]
mod corpus {
    use super::*;
    use crate::indexer::lang::Source;
    use crate::indexer::lang::rust;
    use crate::indexer::resolve::{World, resolve};
    use crate::languages::LanguageAdapter;

    #[tokio::test]
    #[ignore]
    async fn legacy_and_current_over_this_repos_rust() {
        let sources = crate::indexer::corpus_rust_sources();
        let first_party: std::collections::BTreeSet<String> =
            sources.iter().map(|(path, _)| crate::indexer::package_of(path)).collect();
        let scanned = std::collections::BTreeSet::new();
        let world = World { first_party: &first_party, scanned: &scanned };

        let mut totals = DiffReport::default();
        let mut files_compared = 0usize;
        let mut legacy_produced_nothing = 0usize;
        let mut current_failed_to_read = 0usize;
        let mut current_declined: std::collections::BTreeMap<String, usize> = Default::default();
        let mut current_unresolved_names: Vec<(String, String)> = Vec::new();
        // Every DECLARATION each producer makes, normalised the same way a
        // difference's key is. Collected here because "does anything in the
        // corpus declare the thing legacy pointed at" cannot be answered from
        // one file, and it is the question that separates a regression from
        // legacy having been wrong.
        let mut legacy_defs: BTreeSet<String> = BTreeSet::new();
        let mut current_defs: BTreeSet<String> = BTreeSet::new();

        for (abs, text) in &sources {
            // legacy: the shipped producer, from the absolute path (it walks up to
            // the manifest itself).
            let Some(legacy) = crate::languages::rust_lang::RustAdapter.fqn_output(abs, "", text)
            else {
                // legacy declining a file is itself a fact — COUNTED, not skipped,
                // because "legacy produced nothing here" is precisely the case where
                // current looking better means nothing.
                legacy_produced_nothing += 1;
                continue;
            };

            // current: the same file, at the grain a real walk produces.
            let package = crate::indexer::package_of(abs);
            let rel = crate::indexer::workspace_relative(abs);
            let module = crate::indexer::module_of(&rel);
            let Ok(facts) =
                rust::read(&Source { package: &package, module: &module, path: &rel, text })
            else {
                current_failed_to_read += 1;
                continue;
            };
            let current = resolve(facts, &rust::GRAMMAR, &world);

            // Every NAME current saw at a use site but declined to place. If a legacy
            // "resolution" turns up here, current did not lose the reference — it
            // saw it and refused to guess, which R4 says is the right answer
            // and is the opposite of a regression.
            for r in &current.references {
                if let crate::indexer::facts::Resolution::Unresolved { reason, .. } = &r.target {
                    *current_declined.entry(format!("{reason:?}")).or_insert(0usize) += 1;
                }
            }
            current_unresolved_names.extend(current.references.iter().filter_map(
                |r| match &r.target {
                    crate::indexer::facts::Resolution::Unresolved { reason, evidence } => {
                        Some((evidence.name.clone(), format!("{reason:?}")))
                    }
                    _ => None,
                },
            ));

            legacy_defs.extend(legacy.defs.iter().map(|d| identity_key(&d.fqn)));
            current_defs.extend(current.symbols.iter().map(|s| identity_key(s.fqn.as_str())));

            let report = differential(&legacy, &current);
            totals.legacy_resolved += report.legacy_resolved;
            totals.current_resolved += report.current_resolved;
            totals.differences.extend(report.differences);
            files_compared += 1;
        }

        // The second stage: what the per-file gate could not know. See
        // `explain_dangling` for which classes flip and which deliberately do
        // not.
        let before = totals.count(Verdict::Regression);
        explain_dangling(&mut totals, &legacy_defs, &current_defs);
        let explained_away = before - totals.count(Verdict::Regression);

        let regressions = totals.count(Verdict::Regression);
        let improvements = totals.count(Verdict::Improvement);
        let explained = totals.count(Verdict::Explained);
        let unclassified = totals.count(Verdict::Unclassified);

        println!("\n── legacy vs current over {files_compared} rust files ──");
        println!("legacy produced no output   {legacy_produced_nothing}");
        println!("current could not read       {current_failed_to_read}");
        println!("resolved targets  legacy    {}", totals.legacy_resolved);
        println!("resolved targets  current    {}", totals.current_resolved);
        println!("IMPROVEMENT             {improvements}");
        println!("REGRESSION              {regressions}  (was {before} before the dangling rule)");
        println!("  of which legacy was WRONG about {explained_away}");
        println!("EXPLAINED               {explained}");
        println!("UNCLASSIFIED            {unclassified}");
        println!("blocks cutover          {}", totals.blocks_cutover());
        println!("\nUNRESOLVED by reason (references legacy would simply have dropped):");
        for (reason, n) in &current_declined {
            println!("  {n:>6}  {reason}");
        }
        println!("  (total unresolved: {})", current_unresolved_names.len());

        // THE DECIDING MEASUREMENT for the 777. For each regression, did current SEE
        // that name at a use site and decline to place it? If so current did not lose
        // the reference — it refused to guess, which R4 says is correct and is
        // the opposite of a regression.
        let declined_by_name: std::collections::BTreeMap<&str, &str> =
            current_unresolved_names.iter().map(|(n, r)| (n.as_str(), r.as_str())).collect();
        let mut seen_and_declined: std::collections::BTreeMap<&str, usize> = Default::default();
        let mut never_seen = 0usize;
        for d in totals.differences.iter().filter(|d| d.verdict == Verdict::Regression) {
            let leaf = d.key.rsplit('\u{00B7}').next().unwrap_or("");
            match declined_by_name.get(leaf) {
                Some(reason) => *seen_and_declined.entry(reason).or_default() += 1,
                None => never_seen += 1,
            }
        }
        println!("\nof {regressions} regressions, current SAW the name and declined:");
        for (reason, n) in &seen_and_declined {
            println!("  {n:>6}  {reason}");
        }
        println!("  {never_seen:>6}  name never appears at any current use site");

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

/// How much would anchoring a member to its TYPE's module move? (Sizing.)
///
/// A member's identity is `…·<module>·<Type>·<member>`, and this indexer fills
/// `<module>` with the module the `impl` BLOCK sits in. Rust's own path does
/// not: `PgStore::forge_token_rows` is reached through the type, whose path is
/// `db::pg_store::PgStore` however many files carry an `impl PgStore`.
///
/// The reference side has the same rule, so a call resolves only when the
/// caller happens to sit in the same module as the impl block. `PgStore` has 24
/// such files.
///
/// This measures the move BEFORE anything is built, because the lesson this
/// design has paid for three times is that a route sized after the fact reaches
/// almost nothing. It answers three things: how many member identities would
/// change, how many are AMBIGUOUS (two modules declare the type, so nothing may
/// move — R4), and how many currently-unresolved references would newly match.
///
/// `#[ignore]`: it parses the whole corpus.
#[cfg(test)]
mod anchoring {
    use crate::indexer::facts::{Reason, SymbolKind};
    use crate::indexer::lang::Source;
    use crate::indexer::lang::rust;
    use std::collections::BTreeMap;

    /// Split an ITEM identity — `lang·pkg·[module]·name·reach` — into
    /// `(module, name)`.
    ///
    /// The member forms are deliberately NOT decomposed here, and that is a
    /// correction rather than a simplification. `Form::Member` and
    /// `Form::TraitMember` differ by one segment and an empty module is
    /// DROPPED, so a six-segment string is a member-with-module or a
    /// trait-member-without-one and the string cannot say which (`fqn::Parsed`
    /// records exactly this). A first version of this measurement guessed, read
    /// the TRAIT as the type on every trait impl, and reported that every
    /// implementation of a trait should collapse onto one identity — the
    /// wrong-merge the trait qualifier exists to prevent, presented as a fix.
    ///
    /// The `Owns` relation carries the type's identity as an unambiguous item,
    /// so it is read from there instead.
    fn item_parts(fqn: &str) -> Option<(String, String)> {
        let sep = '\u{00B7}';
        let mut segments: Vec<&str> = fqn.split(sep).collect();
        segments.pop()?;
        let name = segments.pop()?.to_string();
        if segments.len() < 2 {
            return None;
        }
        Some((segments[2..].join(&sep.to_string()), name))
    }

    /// Split a MEMBER reference candidate, which the walk always mints as
    /// `Form::Member` — never the trait form, because a call site cannot know
    /// which trait. Knowing `member` makes the split checkable rather than
    /// guessed: if the segment where `member` should be is not `member`, this
    /// is a shape the caller does not understand and yields nothing.
    fn member_parts(fqn: &str, member: &str) -> Option<(String, String)> {
        let sep = '\u{00B7}';
        let mut segments: Vec<&str> = fqn.split(sep).collect();
        segments.pop()?;
        if segments.pop()? != member {
            return None;
        }
        let ty = segments.pop()?.to_string();
        if segments.len() < 2 {
            return None;
        }
        Some((segments[2..].join(&sep.to_string()), ty))
    }

    #[tokio::test]
    #[ignore]
    async fn how_far_a_member_is_from_its_types_module() {
        let sources = crate::indexer::corpus_rust_sources();
        let mut all: Vec<crate::indexer::facts::FileFacts> = Vec::new();
        for (abs, text) in &sources {
            let package = crate::indexer::package_of(abs);
            let rel = crate::indexer::workspace_relative(abs);
            let module = crate::indexer::module_of(&rel);
            if let Ok(facts) =
                rust::read(&Source { package: &package, module: &module, path: &rel, text })
            {
                all.push(facts);
            }
        }

        // Where each type NAME is declared, per package. A name declared in two
        // modules of one package is AMBIGUOUS and nothing may move for it.
        let mut homes: BTreeMap<(String, String), std::collections::BTreeSet<String>> =
            BTreeMap::new();
        for facts in &all {
            for symbol in &facts.symbols {
                let names_a_type = matches!(
                    symbol.kind,
                    SymbolKind::Struct
                        | SymbolKind::Enum
                        | SymbolKind::Trait
                        | SymbolKind::Class
                        | SymbolKind::Interface
                );
                if !names_a_type {
                    continue;
                }
                if let Some((module, _)) = item_parts(symbol.fqn.as_str()) {
                    homes
                        .entry((facts.package.clone(), symbol.name.clone()))
                        .or_default()
                        .insert(module);
                }
            }
        }

        let mut members = 0usize;
        let mut already_right = 0usize;
        let mut would_move = 0usize;
        let mut ambiguous = 0usize;
        let mut type_not_found = 0usize;
        let mut moved_types: std::collections::BTreeSet<String> = Default::default();

        for facts in &all {
            for relation in &facts.relations {
                if relation.kind != crate::indexer::facts::RelationKind::Owns {
                    continue;
                }
                // The OWNER, which is the type this member belongs to, minted
                // as an item — so it decomposes without guessing.
                let crate::indexer::facts::Resolution::Resolved(owner) = &relation.parent else {
                    continue;
                };
                let Some((module, ty)) = item_parts(owner.as_str()) else { continue };
                members += 1;
                match homes.get(&(facts.package.clone(), ty.clone())) {
                    None => type_not_found += 1,
                    Some(where_declared) if where_declared.len() > 1 => ambiguous += 1,
                    Some(where_declared) => {
                        let home = where_declared.iter().next().expect("one entry");
                        if *home == module {
                            already_right += 1;
                        } else {
                            would_move += 1;
                            moved_types.insert(format!("{module}·{ty} -> {home}·{ty}"));
                        }
                    }
                }
            }
        }

        println!("\n── members, by the type that OWNS them, {} files ──", all.len());
        println!("owned members          {members}");
        println!("  module already right {already_right}");
        println!("  WOULD MOVE           {would_move}");
        println!("  ambiguous type name  {ambiguous}  (two modules declare it — nothing may move)");
        println!("  type not in corpus   {type_not_found}  (an impl on an external type)");
        println!("\ndistinct types whose members would move: {}", moved_types.len());
        for key in moved_types.iter().take(10) {
            println!("  {key}");
        }

        // And the other half of the question: how many MISSES would a matching
        // reference-side change newly place? Counted as unresolved member
        // references whose candidate names a type that has ONE home elsewhere.
        let mut recoverable = 0usize;
        for facts in &all {
            for reference in &facts.references {
                let crate::indexer::facts::Resolution::Unresolved { reason, evidence } =
                    &reference.target
                else {
                    continue;
                };
                if !matches!(reason, Reason::Unplaced | Reason::NoImportInScope) {
                    continue;
                }
                for observation in &evidence.saw {
                    let crate::indexer::facts::Observation::Candidate(candidate) = observation
                    else {
                        continue;
                    };
                    let Some((module, ty)) = member_parts(candidate.as_str(), &evidence.name)
                    else {
                        continue;
                    };
                    if let Some(where_declared) = homes.get(&(facts.package.clone(), ty))
                        && where_declared.len() == 1
                        && where_declared.iter().next() != Some(&module)
                    {
                        recoverable += 1;
                    }
                }
            }
        }
        println!("\nunresolved member references whose type has ONE home elsewhere: {recoverable}");
        assert!(members > 100, "only {members} owned members in this corpus?");
    }
}

/// WHY the gate blocks — the investigation behind the 777, with source.
///
/// The gate says "legacy resolved this and current does not". That is a true statement
/// and an insufficient one, because it does not say whether legacy was RIGHT. This
/// answers that, by asking of every disputed target the one question that
/// settles it:
///
/// **Does the thing legacy pointed at exist?**
///
/// Three outcomes, and they mean opposite things:
///
/// - the target exists in NEITHER producer's definitions -> legacy resolved to a
///   GHOST. Nothing in the corpus declares it. legacy's edge pointed at a node
///   minted only because a reference asked for it, and R4 ranks that below no
///   edge at all. Not a regression.
/// - it exists in legacy's definitions but not current's -> the two DISAGREE ON IDENTITY,
///   not on reach. A minting difference, which the normalisation should have
///   covered and did not.
/// - it exists in BOTH -> current HAS the definition and still did not connect the
///   reference to it. The only class that is a real loss of reach.
///
/// `#[ignore]` for the same reason as the gate: it parses the corpus twice.
#[cfg(test)]
mod why {
    use super::*;
    use crate::indexer::lang::Source;
    use crate::indexer::lang::rust;
    use crate::indexer::resolve::{World, resolve};
    use crate::languages::LanguageAdapter;

    /// One disputed target, laid out with everything needed to judge it by hand.
    /// Its CLASSIFICATION is [`super::Disputed`]; this is the evidence beside it.
    struct Case {
        key: String,
        file: String,
        reason: String,
        /// What legacy and current each DECLARE under this member name, so the two
        /// spellings can be read side by side instead of inferred.
        legacy_declares: Vec<String>,
        current_declares: Vec<String>,
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
        let mut legacy_defs: std::collections::BTreeSet<String> = Default::default();
        let mut current_defs: std::collections::BTreeSet<String> = Default::default();
        let mut parsed: Vec<(String, String, crate::indexer::facts::FileFacts)> = Vec::new();
        let mut legacy_by_file: std::collections::BTreeMap<
            String,
            crate::languages::fqn::FqnFileOutput,
        > = Default::default();

        for (abs, text) in &sources {
            let Some(legacy) = crate::languages::rust_lang::RustAdapter.fqn_output(abs, "", text)
            else {
                continue;
            };
            legacy_defs.extend(legacy.defs.iter().map(|d| identity_key(&d.fqn)));

            let package = crate::indexer::package_of(abs);
            let rel = crate::indexer::workspace_relative(abs);
            let module = crate::indexer::module_of(&rel);
            let Ok(facts) =
                rust::read(&Source { package: &package, module: &module, path: &rel, text })
            else {
                continue;
            };
            let facts = resolve(facts, &rust::GRAMMAR, &world);
            current_defs.extend(facts.symbols.iter().map(|s| identity_key(s.fqn.as_str())));
            legacy_by_file.insert(rel.clone(), legacy);
            parsed.push((rel, text.clone(), facts));
        }

        // Pass 2: for every disputed target, classify it by whether it EXISTS.
        let mut ghost = Vec::new();
        let mut trait_qualified = Vec::new();
        let mut identity_disagreement = Vec::new();
        let mut real_loss = Vec::new();

        for (rel, text, current) in &parsed {
            let Some(legacy) = legacy_by_file.get(rel) else { continue };
            let report = differential(legacy, current);

            for d in report.differences.iter().filter(|d| d.verdict == Verdict::Regression) {
                // The use site current declined, matched by the bare name.
                let leaf = d.key.rsplit('\u{00B7}').next().unwrap_or("").to_string();
                let declined = current.references.iter().find_map(|r| match &r.target {
                    crate::indexer::facts::Resolution::Unresolved { reason, evidence }
                        if evidence.name == leaf =>
                    {
                        Some((format!("{reason:?}"), evidence.name.clone(), r.at.start_line))
                    }
                    _ => None,
                });
                let (reason, saw, line) = declined.unwrap_or_else(|| {
                    ("(current emitted no reference for this name)".into(), leaf.clone(), 0)
                });
                // Line 0 means NO use site was matched, so there is no source
                // to show. Printing line 1 there — which an earlier version of
                // this did — puts a file's opening comment under a finding it
                // has nothing to do with, and a reader would reasonably believe
                // the two were related.
                let source_line = if line == 0 {
                    "(no use site matched this name in current's references)".to_string()
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
                let item = Case {
                    key: d.key.clone(),
                    file: rel.clone(),
                    reason,
                    legacy_declares: declares(&legacy_defs),
                    current_declares: declares(&current_defs),
                    line,
                    source_line,
                    saw,
                };
                // The SAME predicate the gate reclassifies with. Two copies
                // would be two answers to the question the cutover rests on,
                // and the one printed here is the one a person reads.
                match classify(&d.key, &legacy_defs, &current_defs) {
                    Disputed::RealLoss => real_loss.push(item),
                    Disputed::IdentityDisagreement => identity_disagreement.push(item),
                    Disputed::LegacySelfDisagreement => trait_qualified.push(item),
                    Disputed::Ghost => ghost.push(item),
                }
            }
        }

        let show = |label: &str, items: &[Case], n: usize| {
            println!("\n\n══ {label}: {} ══", items.len());
            for d in items.iter().take(n) {
                println!("\n  target legacy claimed : {}", d.key);
                println!("  use site          : {}:{}", d.file, d.line);
                println!("  source            : {}", d.source_line);
                println!("  current saw the name   : {}", d.saw);
                println!("  current declined because: {}", d.reason);
                println!("  legacy DECLARES       : {:?}", d.legacy_declares);
                println!("  current DECLARES       : {:?}", d.current_declares);
            }
        };

        println!("\n\n════════ WHY THE GATE BLOCKS ════════");
        println!(
            "legacy definitions {} | current definitions {}",
            legacy_defs.len(),
            current_defs.len()
        );
        show(
            "LEGACY DISAGREES WITH ITSELF — its definition side spells this member with a trait \
             qualifier and its reference side without one, so the edge points at an identity \
             legacy's own parser never declares",
            &trait_qualified,
            5,
        );
        show("GHOST — nothing declares this under ANY identity", &ghost, 5);
        show(
            "IDENTITY DISAGREEMENT — legacy declares it, current mints the declaration differently",
            &identity_disagreement,
            6,
        );
        show(
            "REAL LOSS — current HAS the definition and still did not connect the reference",
            &real_loss,
            8,
        );

        // The IDENTITY DISAGREEMENT class, every one of them, bucketed by the
        // SHAPE of the disagreement.
        //
        // Six samples cannot tell one cause from three, and this class is the
        // one where the two producers both DECLARE the symbol — so each bucket
        // is a decision about which of them is right, not a gap to close.
        {
            let sep = '\u{00B7}';
            let mut shapes: std::collections::BTreeMap<&str, Vec<(String, String)>> =
                Default::default();
            for d in &identity_disagreement {
                let ours = d
                    .current_declares
                    .iter()
                    .find(|k| !k.starts_with('('))
                    .cloned()
                    .unwrap_or_else(|| "(none)".to_string());
                let shape = if ours == "(none)" {
                    "current declares NOTHING under any spelling of this name"
                } else if ours.rsplit_once(sep).map(|(_, m)| m)
                    == d.key.rsplit_once(sep).map(|(_, m)| m)
                    && ours.matches(sep).count() == d.key.matches(sep).count()
                {
                    // Same member, same arity, different module segment.
                    "same member, DIFFERENT module segment"
                } else {
                    "different segmentation"
                };
                shapes.entry(shape).or_default().push((d.key.clone(), ours));
            }
            println!(
                "\n\n══ IDENTITY DISAGREEMENT, all {} by shape ══",
                identity_disagreement.len()
            );
            for (shape, items) in &shapes {
                println!("\n  {} — {}", items.len(), shape);
                for (legacy_key, ours) in items {
                    println!("      legacy  {legacy_key}");
                    println!("      current {ours}");
                }
            }
        }

        // What current SAID, per class. This is the number that says what one fix
        // would buy: a class dominated by a single reason has a single cause.
        fn by_reason(items: &[Case]) -> std::collections::BTreeMap<&str, usize> {
            let mut m: std::collections::BTreeMap<&str, usize> = Default::default();
            for d in items {
                *m.entry(d.reason.as_str()).or_default() += 1;
            }
            m
        }
        for (label, items) in [
            ("legacy self-disagreement", &trait_qualified),
            ("ghost", &ghost),
            ("identity disagreement", &identity_disagreement),
            ("REAL LOSS", &real_loss),
        ] {
            println!("\n{label} — current's reason:");
            for (reason, n) in by_reason(items) {
                println!("  {n:>5}  {reason}");
            }
        }

        println!("\n\n──── verdict ────");
        println!("legacy self-disagreement   {}", trait_qualified.len());
        println!("ghost, no declaration  {}", ghost.len());
        println!("identity disagreement  {}", identity_disagreement.len());
        println!("REAL LOSS OF REACH     {}", real_loss.len());
    }
}

/// WHAT WOULD FIX IT — how far a binding map gets, measured (not estimated).
///
/// current's walk resolves a receiver's type in exactly one case: `self`/`Self`
/// inside a type's own body. It never visits a `let` declaration, so
/// `let cfg = SenseiConfig::from_env(); cfg.method()` is a
/// `ReceiverTypeUnknown` even though the type is stated one line above.
///
/// This counts, over the real corpus, how each unresolved receiver COULD be
/// typed — so the decision about what to build is made against numbers rather
/// than an impression of which shapes are common.
#[cfg(test)]
mod reach {
    use crate::indexer::facts::{Observation, Resolution};
    use crate::indexer::lang::Source;
    use crate::indexer::lang::rust;
    use crate::indexer::resolve::{World, resolve};

    /// How a receiver's type could be learned, cheapest first.
    #[derive(PartialEq, Eq, PartialOrd, Ord)]
    enum Route {
        /// `for x in coll` — a loop binding, typed by the collection's ELEMENT
        /// type. Structurally a declaration; it was being counted as "other"
        /// only because no `let` or `:` names it.
        ForBinding,
        /// The stated type is `dyn Trait` (or `Box<dyn Trait>`). The concrete
        /// impl is unknowable, but the TRAIT METHOD is a real declaration and is
        /// the correct target — measured: `Checker::check` is minted as
        /// `…·Checker·check·item`. Refusing these conflated "which impl runs"
        /// with "which method is called"; only the first is unanswerable.
        DynTrait,
        /// `let x: T = …` — the type is written at the binding.
        LetAnnotation,
        /// `let x = T::assoc(…)` — the initialiser's path names the type.
        LetInitialiserPath,
        /// `x: T` in the enclosing fn signature.
        Parameter,
        /// `self.m().c()` — the inner call resolves already; only `m`'s
        /// RETURN type is missing, and `m` is declared on this same type.
        ChainedOnSelf,
        /// `a.b().c()` — needs the inner call's RETURN type, and the inner
        /// receiver is not `self`, so the declaration may be anywhere.
        ChainedCall,
        /// A receiver that is not a plain identifier and not a chain.
        Other,
    }

    impl Route {
        fn label(&self) -> &'static str {
            match self {
                Self::ForBinding => "for x in coll     (element type of the collection)",
                Self::DynTrait => "dyn Trait         (the TRAIT METHOD is the target)",
                Self::LetAnnotation => "let x: T          (annotation at the binding)",
                Self::LetInitialiserPath => "let x = T::f()    (initialiser names the type)",
                Self::Parameter => "fn(x: T)          (enclosing signature)",
                Self::ChainedOnSelf => "self.m().c()      (inner resolves; needs m's RETURN type)",
                Self::ChainedCall => "a.b().c()         (needs the inner RETURN type)",
                Self::Other => "other             (not a plain identifier)",
            }
        }
    }

    /// The HEAD of whatever type the source states for `name`, if it states one.
    ///
    /// Deliberately looser than `simple_type_name`: that one decides what is
    /// safe to RECORD as an identity, this one only asks whose type it is. So
    /// `Vec<Config>` yields `Vec` and `&Path` yields `Path` — neither is
    /// mintable, both are answerable.
    fn stated_type_head(name: &str, text: &str) -> Option<String> {
        if !name.chars().all(|c| c.is_alphanumeric() || c == '_') || name.is_empty() {
            return None;
        }
        let after = |prefix: String| -> Option<String> {
            let i = text.find(&prefix)?;
            let rest = &text[i + prefix.len()..];
            let head: String = rest
                .trim_start()
                .trim_start_matches('&')
                .trim_start()
                .trim_start_matches("mut ")
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            (!head.is_empty()).then_some(head)
        };
        after(format!("let {name}: "))
            .or_else(|| after(format!("let mut {name}: ")))
            .or_else(|| after(format!("{name}: ")))
            .or_else(|| {
                // `let x = T::f()` / `let x = T { … }`
                let i = text.find(&format!("let {name} = "))?;
                let init = text[i..].lines().next()?;
                let (_, rhs) = init.split_once(" = ")?;
                let head: String =
                    rhs.trim().chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                (!head.is_empty() && head.chars().next().is_some_and(char::is_uppercase))
                    .then_some(head)
            })
    }

    /// The stated type VERBATIM (not just its head), so `dyn` stays visible.
    fn stated_type_head_raw(name: &str, text: &str) -> Option<String> {
        for prefix in [format!("let {name}: "), format!("let mut {name}: "), format!("{name}: ")] {
            if let Some(i) = text.find(&prefix) {
                let rest = &text[i + prefix.len()..];
                let upto: String = rest
                    .chars()
                    .take_while(|c| !matches!(c, ',' | ';' | ')' | '=' | '\n'))
                    .collect();
                if !upto.trim().is_empty() {
                    return Some(upto.trim().to_string());
                }
            }
        }
        None
    }

    fn route_for(receiver: &str, text: &str) -> Route {
        // A chain is decided by shape alone.
        if receiver.contains('.') || receiver.contains('(') {
            // Split out the case where the chain's INNER receiver is `self`.
            // That inner call already resolves — the walk knows `self`'s type
            // from the container — so the only missing fact is the inner
            // method's RETURN type, and a method of `self` is declared on the
            // same type, overwhelmingly in the same file the walk is holding.
            // No graph needed for those; a same-file declaration lookup does it.
            return if receiver.starts_with("self.") {
                Route::ChainedOnSelf
            } else {
                Route::ChainedCall
            };
        }
        if !receiver.chars().all(|c| c.is_alphanumeric() || c == '_') || receiver.is_empty() {
            return Route::Other;
        }
        // Deliberately TEXTUAL and file-wide, not scope-aware. This is an upper
        // bound on what each route reaches, and it is labelled as one — a
        // scope-aware count needs the walk itself, which is the thing being
        // sized. Over-counting here is visible; under-counting would hide reach.
        // `dyn Trait` first: the stated type says so outright, whatever bound it.
        // The concrete impl is unknowable, but the TRAIT METHOD is a real
        // declaration and is the correct target — `Checker::check` exists.
        if stated_type_head_raw(receiver, text).is_some_and(|t| t.contains("dyn ")) {
            return Route::DynTrait;
        }
        // A loop binding is structurally a declaration — the collection states
        // the element type — and was being lumped into "other" because no `let`
        // or `:` names it.
        if text.contains(&format!("for {receiver} in "))
            || text.contains(&format!("for &{receiver} in "))
        {
            return Route::ForBinding;
        }
        let annotated = format!("let {receiver}: ");
        let annotated_mut = format!("let mut {receiver}: ");
        if text.contains(&annotated) || text.contains(&annotated_mut) {
            return Route::LetAnnotation;
        }
        for prefix in [format!("let {receiver} = "), format!("let mut {receiver} = ")] {
            if let Some(i) = text.find(&prefix) {
                let rest = &text[i + prefix.len()..];
                let init = rest.lines().next().unwrap_or("");
                // `T::f(` with T capitalised — an associated function or
                // constructor, whose first segment names the type.
                if let Some((head, _)) = init.split_once("::")
                    && head.chars().next().is_some_and(char::is_uppercase)
                {
                    return Route::LetInitialiserPath;
                }
                return Route::Other;
            }
        }
        if text.contains(&format!("{receiver}: ")) {
            return Route::Parameter;
        }
        Route::Other
    }

    #[tokio::test]
    #[ignore]
    async fn how_each_unknown_receiver_could_be_typed() {
        let sources = crate::indexer::corpus_rust_sources();
        let first_party: std::collections::BTreeSet<String> =
            sources.iter().map(|(path, _)| crate::indexer::package_of(path)).collect();
        let scanned = std::collections::BTreeSet::new();
        let world = World { first_party: &first_party, scanned: &scanned };

        // Every TYPE the corpus declares. The question below is whether an
        // unresolved receiver's type is one of ours at all — because a call on
        // a std type is not a first-party edge we are failing to make, it is an
        // external call, and the two are different findings.
        let mut first_party_types: std::collections::BTreeSet<String> = Default::default();
        for (abs, text) in &sources {
            let package = crate::indexer::package_of(abs);
            let rel = crate::indexer::workspace_relative(abs);
            let module = crate::indexer::module_of(&rel);
            if let Ok(f) =
                rust::read(&Source { package: &package, module: &module, path: &rel, text })
            {
                first_party_types.extend(
                    f.symbols
                        .iter()
                        .filter(|s| {
                            matches!(
                                s.kind,
                                crate::indexer::facts::SymbolKind::Struct
                                    | crate::indexer::facts::SymbolKind::Enum
                                    | crate::indexer::facts::SymbolKind::Trait
                                    | crate::indexer::facts::SymbolKind::TypeAlias
                            )
                        })
                        .map(|s| s.name.clone()),
                );
            }
        }
        let mut ours = 0usize;
        let mut theirs = 0usize;
        let mut untypable = 0usize;
        let mut ours_sample: Vec<String> = Vec::new();

        let mut by_route: std::collections::BTreeMap<&str, usize> = Default::default();
        let mut total = 0usize;
        let mut examples: std::collections::BTreeMap<&str, Vec<String>> = Default::default();

        for (abs, text) in &sources {
            let package = crate::indexer::package_of(abs);
            let rel = crate::indexer::workspace_relative(abs);
            let module = crate::indexer::module_of(&rel);
            let Ok(facts) =
                rust::read(&Source { package: &package, module: &module, path: &rel, text })
            else {
                continue;
            };
            let facts = resolve(facts, &rust::GRAMMAR, &world);

            for r in &facts.references {
                let Resolution::Unresolved { reason, evidence } = &r.target else { continue };
                if !matches!(reason, crate::indexer::facts::Reason::ReceiverTypeUnknown) {
                    continue;
                }
                let receiver = evidence.saw.iter().find_map(|o| match o {
                    Observation::Receiver(s) => Some(s.as_str()),
                    _ => None,
                });
                let Some(receiver) = receiver else { continue };
                total += 1;
                // Whose type is it? Take the head of whatever the source
                // states for this name — `Vec<Config>` -> `Vec`, `&Path` ->
                // `Path` — and ask whether the corpus declares it.
                match stated_type_head(receiver, text) {
                    Some(head) if first_party_types.contains(&head) => {
                        ours += 1;
                        if ours_sample.len() < 10 {
                            ours_sample.push(format!(
                                "{rel}:{} — {receiver}.{}   [{receiver}: {head}]",
                                r.at.start_line, evidence.name
                            ));
                        }
                    }
                    Some(_) => theirs += 1,
                    None => untypable += 1,
                }

                let route = route_for(receiver, text);
                *by_route.entry(route.label()).or_default() += 1;
                let ex = examples.entry(route.label()).or_default();
                if ex.len() < 3 {
                    ex.push(format!("{rel}:{} — {receiver}.{}", r.at.start_line, evidence.name));
                }
            }
        }

        let typed = ours + theirs;
        println!("\n\n════ whose type is the receiver? ════");
        println!("  {ours:>6}  declared by THIS CORPUS — a first-party edge we are missing");
        for e in &ours_sample {
            println!("             {e}");
        }
        println!("  {theirs:>6}  NOT ours (std, a crate) — an EXTERNAL call, not a missing edge");
        println!("  {untypable:>6}  the source states no type for this name at all");
        if typed > 0 {
            println!(
                "  -> of the {typed} we can attribute, {:.0}% are external",
                (theirs as f64) * 100.0 / (typed as f64)
            );
        }

        println!("\n\n════ how {total} ReceiverTypeUnknown receivers COULD be typed ════");
        println!("(upper bound — the match is textual and file-wide, not scope-aware)\n");
        let mut rows: Vec<_> = by_route.iter().collect();
        rows.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
        for (label, n) in rows {
            let pct = (*n as f64) * 100.0 / (total.max(1) as f64);
            println!("  {n:>6}  {pct:>5.1}%  {label}");
            for e in examples.get(label).into_iter().flatten() {
                println!("                     e.g. {e}");
            }
        }
    }
}
