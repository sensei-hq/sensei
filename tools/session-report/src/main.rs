//! session-report — a retrospective from Copilot CLI transcripts.
//!
//! Reads a folder of `<session-id>/events.jsonl` directories and writes a
//! markdown retrospective: what the sessions show about pace, where friction
//! showed up, and what is working. Every observation carries a reference — a
//! session id and a timestamp — so it can be checked against the source.
//!
//! Deliberately isolated from the daemon: no database, no network, no writes
//! anywhere except the output file. These are other people's transcripts.

mod advice;
mod claude;
mod compare;
mod facets;
mod metrics;
mod model;
mod parse;
mod render;
mod retro;
mod signals;
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
         --facets [endpoint] derives one LLM record per session (default: a LOCAL\n\
         ollama at 127.0.0.1:11434) and writes them to facets/<name>/ beside the\n\
         report. --facet-model picks the model (default gemma4:latest).\n\
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
    // Facets are OPT-IN and default to a LOCAL model: these are other people's
    // transcripts, and the prompt text must not leave the machine unless the
    // person running this says so explicitly.
    let mut facets: Option<String> = None;
    let mut facet_model = "gemma4:latest".to_string();
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
            "--facets" => {
                facets = Some(
                    args.get(i + 1)
                        .cloned()
                        .unwrap_or_else(|| "http://127.0.0.1:11434/api/generate".to_string()),
                );
                // Allow a bare `--facets` before another flag.
                if args.get(i + 1).is_some_and(|a| a.starts_with('-')) {
                    facets = Some("http://127.0.0.1:11434/api/generate".to_string());
                    i += 1;
                } else {
                    i += 2;
                }
            }
            "--facet-model" => {
                facet_model = args.get(i + 1).cloned().unwrap_or(facet_model);
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

    if let Some(endpoint) = &facets {
        run_facets(&sessions, endpoint, &facet_model, &facet_dir(out.as_deref(), &label));
    }

    let analysis = metrics::analyse(&sessions, skipped);

    // Person-level synthesis, once, cached beside the facets. Derived here
    // rather than in run_facets because it needs the finished Analysis.
    if let Some(endpoint) = &facets {
        let dir = facet_dir(out.as_deref(), &label);
        let target = dir.join("_insights.json");
        if !target.exists() {
            let found = load_facets(&dir);
            match advice::derive(&label, &found, &analysis, endpoint, &facet_model) {
                Ok(ins) => {
                    let _ = std::fs::write(
                        &target,
                        serde_json::to_string_pretty(&ins).unwrap_or_default(),
                    );
                    eprintln!("  insights: {} recommendation(s) kept", ins.recommendations.len());
                }
                // Reported, not swallowed: an empty recommendations section
                // should be visibly empty, not silently absent.
                Err(e) => eprintln!("  insights: none — {e}"),
            }
        }
    }

    let insights = load_insights(&facet_dir(out.as_deref(), &label));
    let mut doc = render::report(&label, tool, &sessions, &analysis);
    // Facets are read from disk rather than from the run above, so a report can
    // be regenerated without re-deriving them.
    let facet_dir = facet_dir(out.as_deref(), &label);
    let found = load_facets(&facet_dir);
    if !found.is_empty() {
        doc.push_str(&retro::report(&label, &found, &analysis, insights.as_ref()));
    }

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

/// Where this person's derived records live, beside their report.
fn facet_dir(out: Option<&Path>, label: &str) -> PathBuf {
    out.and_then(|p| p.parent()).unwrap_or(Path::new(".")).join("facets").join(label)
}

/// The person-level synthesis, if it has been derived.
fn load_insights(dir: &Path) -> Option<advice::Insights> {
    let text = std::fs::read_to_string(dir.join("_insights.json")).ok()?;
    serde_json::from_str(&text).ok()
}

/// Read the facet records already derived for this person.
fn load_facets(dir: &Path) -> Vec<facets::Facet> {
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<facets::Facet> = rd
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .filter_map(|t| serde_json::from_str(&t).ok())
        .collect();
    out.sort_by(|a: &facets::Facet, b| a.session_id.cmp(&b.session_id));
    out
}

/// Derive one facet per session and write it beside the report.
///
/// Failures are REPORTED, not silently skipped: a facet dropped for want of
/// grounding is a real gap in the retrospective, and pretending the run was
/// clean would overstate how much of the person's work the report covers.
fn run_facets(sessions: &[model::Session], endpoint: &str, facet_model: &str, dir: &Path) {
    if let Err(e) = std::fs::create_dir_all(dir) {
        eprintln!("error: cannot create {}: {e}", dir.display());
        std::process::exit(1);
    }
    let (mut ok, mut failed) = (0usize, 0usize);
    let mut reasons: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut dropped: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for (i, s) in sessions.iter().enumerate() {
        let target = dir.join(format!("{}.json", s.id));
        if target.exists() {
            ok += 1;
            continue;
        }
        eprint!("  facet {}/{} {}\r", i + 1, sessions.len(), &s.id[..8.min(s.id.len())]);
        match facets::derive(s, endpoint, facet_model) {
            Ok(f) => match serde_json::to_string_pretty(&f) {
                Ok(j) => {
                    let _ = std::fs::write(&target, j);
                    ok += 1;
                }
                Err(e) => {
                    failed += 1;
                    *reasons.entry(e.to_string()).or_default() += 1;
                }
            },
            Err(e) => {
                failed += 1;
                // Group by the kind of failure, not the instance — but name the
                // sessions, so a gap in coverage can be traced to real files
                // rather than reported as an anonymous count.
                let key = e.split(':').next().unwrap_or(&e).to_string();
                *reasons.entry(key.clone()).or_default() += 1;
                dropped.entry(key).or_default().push(s.id.clone());
            }
        }
    }
    eprintln!("  facets: {ok} derived, {failed} dropped                    ");
    let mut rs: Vec<(&String, &usize)> = reasons.iter().collect();
    rs.sort_by_key(|r| std::cmp::Reverse(*r.1));
    for (why, n) in rs {
        eprintln!("    {n} × {why}");
        if let Some(ids) = dropped.get(why) {
            for id in ids.iter().take(3) {
                eprintln!("        {id}");
            }
        }
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
        if let Some(mut o) = parse::parse_session(&d) {
            skipped += o.skipped_lines;
            o.session.file = Some(d.join("events.jsonl"));
            sessions.push(o.session);
        }
    }
    sessions.sort_by_key(|s| s.first_ms);
    (sessions, skipped, scanned)
}
