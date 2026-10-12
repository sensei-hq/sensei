//! Reading the corpus — the one reader every measurement in this layer shares.
//!
//! Not a threshold and not a gate: it opens files and walks them, and
//! `acceptance`, `reachability`, `impact`, `persist` and `resolve` all measure
//! over what it returns. It lived inside `acceptance` because that is where the
//! first measurement was written, which is filing by history rather than by
//! what the code does.
//!
//! ONE READER IS THE POINT. A second copy would not be a second measurement; it
//! would be two numbers that drift apart and then disagree about which language
//! regressed. The same argument the walk makes about having one producer.
//!
//! THE BARRIER IS HERE. Every file is walked once with no type table so the
//! type DECLARATIONS can be collected, then walked again with that table — which
//! is what makes a member's identity independent of which file came first (R6).

use std::collections::BTreeSet;

use crate::indexer::facts::FileFacts;
use crate::indexer::lang::{self, Source, TypeHomes};
use crate::indexer::resolve::{
    World, member_names_of, members_declared_by, resolve, returns_declared_by,
};

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
/// `pub(crate)`, not `pub(super)`: `impact` and `persist` measure over this
/// same corpus, and neither is a descendant of `quality`. The narrower
/// visibility worked only while this module sat directly under `indexer` —
/// and a second reader would be a second measurement that drifts.
pub(crate) struct Read {
    pub(crate) package: String,
    pub(crate) path: String,
    pub(crate) text: String,
    pub(crate) facts: FileFacts,
}

/// Read the whole corpus — every language — through the adapter each extension
/// dispatches to.
///
/// The barrier is here: every file is walked once with no type table so the
/// type DECLARATIONS can be collected, then walked again with the table, which
/// is what makes a member's identity independent of which file came first (R6).
pub(crate) fn read_the_corpus() -> Vec<Read> {
    let mut sources: Vec<(String, String, String)> = Vec::new();
    for (abs, text) in crate::indexer::corpus_rust_sources() {
        let package = crate::indexer::package_of(&abs);
        let rel = crate::indexer::workspace_relative(&abs);
        sources.push((package, rel, text));
    }
    for (rel, text) in crate::indexer::corpus_web_sources() {
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
