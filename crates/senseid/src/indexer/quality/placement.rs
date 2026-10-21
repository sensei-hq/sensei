//! **THE PLACEMENT FLOOR** — how many of this repository's calls and references
//! the production path places, by language, package and edge kind, and the
//! floor none of them may fall below
//! (docs/history/plan/2026-10-13-indexer-full-call-placement.md).
//!
//! Measured the way production places them, not a model of it: the files the
//! repository walk finds and its linguist marks keep, each package's project
//! compiled by its language's engine (`CompilerSessions::compile_packages`),
//! each file placed by `index_file` against that compile. A second harness that
//! re-implemented any of that would be a second measurement, and two numbers
//! that drift.
//!
//! A cell is (language, package, edge kind); `calls` are calls and macro
//! invocations, `references` reads, writes, constructions and type uses. A use
//! that is no edge (a local, a parameter, an anonymous function) is neither
//! placed nor missed — it was never an edge.

use std::collections::BTreeMap;
use std::path::Path;

use crate::indexer::compiler_session::{CompilerSessions, canonical};
use crate::indexer::facts::{RefKind, Resolution};
use crate::indexer::index::{FileInput, Indexed, Mode, index_file};
use crate::indexer::lang::compiler::Compiled;
use crate::indexer::repo::EntryClass;

/// Nothing placed by a compiler engine may fall below this share.
const FLOOR: f64 = 0.90;

#[derive(Default)]
struct Cell {
    placed: u64,
    missed: BTreeMap<String, u64>,
}

impl Cell {
    fn missed(&self) -> u64 {
        self.missed.values().sum()
    }
    fn share(&self) -> f64 {
        let all = self.placed + self.missed();
        if all == 0 { 1.0 } else { self.placed as f64 / all as f64 }
    }
}

fn language_of(path: &str) -> &'static str {
    match path.rsplit_once('.').map(|(_, e)| e) {
        Some("rs") => "rust",
        Some("svelte") => "svelte",
        _ => "ts/js",
    }
}

fn edge_kind_of(kind: RefKind) -> Option<&'static str> {
    match kind {
        RefKind::Calls | RefKind::MacroInvokes => Some("calls"),
        RefKind::Reads | RefKind::Writes | RefKind::Constructs | RefKind::TypeUse => {
            Some("references")
        }
        RefKind::Imports => None,
    }
}

#[tokio::test]
#[ignore = "compiles this repository with its real engines (rust-analyzer, the TypeScript \
            checker) — minutes; run nightly and before a release"]
async fn placement_never_falls_below_the_floor() {
    let repo = canonical(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."));
    let contents = crate::indexer::repo::scan(&repo, &[]).expect("the repository walks");
    let dirs = contents.entries.iter().filter_map(|e| e.rel_path.parent().map(Path::to_path_buf));
    let marks = crate::tasks::handlers::helpers::LinguistMarks::load(&repo, dirs)
        .expect("the linguist marks read");

    // The files production indexes through a compiler: owned source, with an engine.
    let mut files = Vec::new();
    for entry in &contents.entries {
        let EntryClass::Source { language } = entry.class else { continue };
        let abs = repo.join(&entry.rel_path);
        let ext = abs.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_default();
        if marks.classify(&abs).is_some()
            || crate::tasks::handlers::helpers::classify_unscannable(&abs, &ext).is_some()
        {
            continue;
        }
        let Some(engine) =
            crate::indexer::lang::adapter_for_ext(&format!(".{ext}")).and_then(|a| a.compiler())
        else {
            continue;
        };
        let Some((package_dir, _)) =
            crate::indexer::pipeline::owning_manifest_on_disk(&abs, &repo, language)
        else {
            continue;
        };
        files.push((
            entry.rel_path.to_string_lossy().into_owned(),
            abs,
            language,
            engine,
            package_dir,
        ));
    }

    let sessions = CompilerSessions::new(&repo);
    let packages = files.iter().map(|(_, _, _, engine, dir)| (*engine, dir.clone())).collect();
    for (project, answer) in sessions.compile_packages(packages).await {
        if let Err(e) = answer {
            eprintln!("NOT COMPILED {}: {e}", project.display());
        }
    }

    let mut cells: BTreeMap<(&'static str, String, &'static str), Cell> = BTreeMap::new();
    for (rel, abs, language, engine, package_dir) in &files {
        let Some(placement) = crate::indexer::pipeline::placement_on_disk(abs, &repo, *language)
        else {
            continue;
        };
        let Ok(text) = std::fs::read_to_string(abs) else { continue };
        let project = sessions.compiled_for(*engine, package_dir).await.ok();
        let path_in_project = project.as_ref().and_then(|p| {
            canonical(abs).strip_prefix(&p.root).ok().map(|p| p.to_string_lossy().into_owned())
        });
        let declarations = project.as_ref().map(|p| sessions.declarations(p));
        let compiled = match (&project, &path_in_project, &declarations) {
            (Some(p), Some(path), Some(d)) => {
                Some(Compiled { index: &p.index, path, declarations: d })
            }
            _ => None,
        };
        let told = crate::indexer::pipeline::TellFile::about(&placement.package);
        let indexed = index_file(FileInput {
            repo: &repo.to_string_lossy(),
            path: rel,
            mode: Mode::New,
            package: &placement.package,
            module: &placement.module,
            text: &text,
            world: &told.world(),
            compiled: compiled.as_ref(),
        });
        let Indexed::Read(facts) = indexed.indexed else { continue };
        for reference in &facts.references {
            let Some(kind) = edge_kind_of(reference.kind) else { continue };
            let cell =
                cells.entry((language_of(rel), placement.package.clone(), kind)).or_default();
            match &reference.target {
                Resolution::Resolved { .. } => cell.placed += 1,
                Resolution::Unresolved { reason, .. } => {
                    *cell.missed.entry(reason.as_label().to_string()).or_default() += 1
                }
            }
        }
    }

    // The decomposition, every cell: the report the floor is read from.
    let mut by_language: BTreeMap<(&str, &str), Cell> = BTreeMap::new();
    println!(
        "{:<8} {:<30} {:<11} {:>8} {:>7} {:>8}  misses",
        "lang", "package", "kind", "placed", "missed", "share"
    );
    for ((language, package, kind), cell) in &cells {
        println!(
            "{language:<8} {package:<30} {kind:<11} {:>8} {:>7} {:>7.2}%  {:?}",
            cell.placed,
            cell.missed(),
            100.0 * cell.share(),
            cell.missed
        );
        let total = by_language.entry((language, kind)).or_default();
        total.placed += cell.placed;
        for (reason, n) in &cell.missed {
            *total.missed.entry(reason.clone()).or_default() += n;
        }
    }
    println!();
    for ((language, kind), cell) in &by_language {
        println!(
            "{language:<8} {kind:<11} {:>8} {:>7} {:>7.2}%  {:?}",
            cell.placed,
            cell.missed(),
            100.0 * cell.share(),
            cell.missed
        );
    }

    assert!(!cells.is_empty(), "the floor measured nothing — no file was placed");
    let below: Vec<_> = cells
        .iter()
        .filter(|(_, cell)| cell.share() < FLOOR)
        .map(|((language, package, kind), cell)| {
            format!("{language} {package} {kind}: {:.2}%", 100.0 * cell.share())
        })
        .collect();
    assert!(below.is_empty(), "placement fell below {:.0}%: {below:#?}", 100.0 * FLOOR);
}
