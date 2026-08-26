//! session-report — a retrospective from Copilot CLI transcripts.
//!
//! Reads a folder of `<session-id>/events.jsonl` directories and writes a
//! markdown retrospective: what the sessions show about pace, where friction
//! showed up, and what is working. Every observation carries a reference — a
//! session id and a timestamp — so it can be checked against the source.
//!
//! Deliberately isolated from the daemon: no database, no network, no writes
//! anywhere except the output file. These are other people's transcripts.

mod claude;
mod compare;
mod metrics;
mod model;
mod parse;
mod render;
mod vscode;

use std::path::{Path, PathBuf};

fn usage() -> ! {
    eprintln!(
        "usage: session-report --input <folder> [--name <label>] [--out <file.md>]\n\
         \n\
         <folder> holds one directory per session, each with events.jsonl.\n\
         --compare treats <folder> as a folder OF people, one subfolder each, and\n\
         writes a single side-by-side instead.\n\
         \n\
         Writes markdown to --out, or stdout when omitted."
    );
    std::process::exit(2)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut input: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut name: Option<String> = None;
    let mut compare = false;
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
            "--compare" | "-c" => {
                compare = true;
                i += 1;
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

    if compare {
        return run_compare(&input, out.as_deref());
    }

    let tool = detect(&input);
    let (sessions, skipped, scanned) = match tool {
        Some(Tool::ClaudeCode) => {
            let (s, sk) = claude::collect(&input);
            let n = s.len();
            (s, sk, n)
        }
        Some(Tool::VsCode) => {
            let (s, sk) = vscode::collect(&input);
            let n = s.len();
            (s, sk, n)
        }
        _ => collect(&input),
    };
    if sessions.is_empty() {
        // Honest failure: say what was looked for and where, rather than emit an
        // empty report that reads as "this person did nothing".
        eprintln!(
            "error: no sessions under {} ({scanned} scanned). Expected either a \
             Copilot CLI folder (one directory per session, each with events.jsonl) \
             or a Claude Code folder (a `projects/` tree).",
            input.display()
        );
        std::process::exit(1);
    }

    let analysis = metrics::analyse(&sessions, skipped);
    let doc = render::report(&label, tool, &sessions, &analysis);

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

/// One report covering everyone in a folder of people.
fn run_compare(root: &Path, out: Option<&Path>) {
    let mut people = Vec::new();
    let Ok(rd) = std::fs::read_dir(root) else {
        eprintln!("error: cannot read {}", root.display());
        std::process::exit(1);
    };
    let mut dirs: Vec<PathBuf> =
        rd.filter_map(Result::ok).map(|e| e.path()).filter(|p| p.is_dir()).collect();
    dirs.sort();
    for dir in dirs {
        let tool = detect(&dir);
        let (sessions, skipped, _) = match tool {
            Some(Tool::ClaudeCode) => {
                let (s, sk) = claude::collect(&dir);
                let n = s.len();
                (s, sk, n)
            }
            Some(Tool::VsCode) => {
                let (s, sk) = vscode::collect(&dir);
                let n = s.len();
                (s, sk, n)
            }
            Some(Tool::CopilotCli) => collect(&dir),
            // Say which folder was skipped and why, rather than quietly
            // producing a comparison that is missing someone.
            None => {
                eprintln!("skipping {} — no recognised transcript format", dir.display());
                continue;
            }
        };
        if sessions.is_empty() {
            eprintln!("skipping {} — no sessions parsed", dir.display());
            continue;
        }
        let name = dir.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        eprintln!("  {name}: {} session(s)", sessions.len());
        people.push(compare::Person { name, tool, analysis: metrics::analyse(&sessions, skipped) });
    }
    if people.is_empty() {
        eprintln!("error: nobody to compare under {}", root.display());
        std::process::exit(1);
    }
    let doc = compare::report(&people);
    match out {
        Some(p) => {
            if let Err(e) = std::fs::write(p, &doc) {
                eprintln!("error: could not write {}: {e}", p.display());
                std::process::exit(1);
            }
            eprintln!("wrote {} — {} people", p.display(), people.len());
        }
        None => print!("{doc}"),
    }
}

/// Which transcript format a folder holds.
///
/// Detected, not configured: people send the folder their tool made and should
/// not have to know what it is called.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tool {
    CopilotCli,
    ClaudeCode,
    VsCode,
}

impl Tool {
    pub fn label(self) -> &'static str {
        match self {
            Tool::CopilotCli => "GitHub Copilot CLI",
            Tool::ClaudeCode => "Claude Code",
            Tool::VsCode => "VS Code (Copilot Chat)",
        }
    }
}

pub fn detect(root: &Path) -> Option<Tool> {
    if root.join("events.jsonl").is_file() {
        return Some(Tool::CopilotCli);
    }
    // Copilot is checked BEFORE Claude, because a Copilot folder can also
    // contain a `projects/` directory — manoj's does. Matching on `projects/`
    // first silently classified him as Claude, found nothing, and dropped him
    // from the comparison entirely.
    if let Ok(rd) = std::fs::read_dir(root)
        && rd.filter_map(Result::ok).any(|e| e.path().join("events.jsonl").is_file())
    {
        return Some(Tool::CopilotCli);
    }
    if root.join("projects").is_dir() {
        return Some(Tool::ClaudeCode);
    }
    // A workspaceStorage tree: one directory per workspace, each with
    // chatSessions/. Checked last because it is the least distinctive shape.
    let ws = if root.join("workspaceStorage").is_dir() {
        root.join("workspaceStorage")
    } else {
        root.to_path_buf()
    };
    if let Ok(rd) = std::fs::read_dir(&ws)
        && rd.filter_map(Result::ok).any(|e| e.path().join("chatSessions").is_dir())
    {
        return Some(Tool::VsCode);
    }
    None
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
