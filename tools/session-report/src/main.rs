//! session-report — a retrospective from Copilot CLI transcripts.
//!
//! Reads a folder of `<session-id>/events.jsonl` directories and writes a
//! markdown retrospective: what the sessions show about pace, where friction
//! showed up, and what is working. Every observation carries a reference — a
//! session id and a timestamp — so it can be checked against the source.
//!
//! Deliberately isolated from the daemon: no database, no network, no writes
//! anywhere except the output file. These are other people's transcripts.

mod metrics;
mod model;
mod parse;
mod render;

use std::path::{Path, PathBuf};

fn usage() -> ! {
    eprintln!(
        "usage: session-report --input <folder> [--name <label>] [--out <file.md>]\n\
         \n\
         <folder> holds one directory per session, each with events.jsonl.\n\
         Writes markdown to --out, or stdout when omitted."
    );
    std::process::exit(2)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut input: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut name: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--input" | "-i" => {
                input = args.get(i + 1).map(PathBuf::from);
                i += 2;
            }
            "--out" | "-o" => {
                out = args.get(i + 1).map(PathBuf::from);
                i += 2;
            }
            "--name" | "-n" => {
                name = args.get(i + 1).cloned();
                i += 2;
            }
            _ => usage(),
        }
    }
    let Some(input) = input else { usage() };
    if !input.is_dir() {
        eprintln!("error: {} is not a directory", input.display());
        std::process::exit(1);
    }

    let label = name.unwrap_or_else(|| {
        input.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()
    });

    let (sessions, skipped, scanned) = collect(&input);
    if sessions.is_empty() {
        // Honest failure: say what was looked for and where, rather than emit an
        // empty report that reads as "this person did nothing".
        eprintln!(
            "error: no Copilot CLI sessions under {} ({scanned} directories scanned, none had events.jsonl)",
            input.display()
        );
        std::process::exit(1);
    }

    let analysis = metrics::analyse(&sessions, skipped);
    let doc = render::report(&label, &sessions, &analysis);

    match out {
        Some(p) => {
            if let Err(e) = std::fs::write(&p, &doc) {
                eprintln!("error: could not write {}: {e}", p.display());
                std::process::exit(1);
            }
            eprintln!(
                "wrote {} — {} session(s), {} events",
                p.display(),
                analysis.sessions,
                analysis.events
            );
        }
        None => print!("{doc}"),
    }
}

/// Find session directories one level down, and also accept being pointed
/// straight at a single session.
fn collect(root: &Path) -> (Vec<model::Session>, usize, usize) {
    let mut sessions = Vec::new();
    let mut skipped = 0usize;
    let mut scanned = 0usize;

    if root.join("events.jsonl").is_file()
        && let Some(o) = parse::parse_session(root)
    {
        skipped += o.skipped_lines;
        sessions.push(o.session);
        return (sessions, skipped, 1);
    }

    let Ok(rd) = std::fs::read_dir(root) else {
        return (sessions, skipped, scanned);
    };
    let mut dirs: Vec<PathBuf> =
        rd.filter_map(Result::ok).map(|e| e.path()).filter(|p| p.is_dir()).collect();
    dirs.sort();
    for d in dirs {
        scanned += 1;
        if let Some(o) = parse::parse_session(&d) {
            skipped += o.skipped_lines;
            sessions.push(o.session);
        }
    }
    sessions.sort_by_key(|s| s.first_ms);
    (sessions, skipped, scanned)
}
