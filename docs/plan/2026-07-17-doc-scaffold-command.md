# Doc-Scaffold Command Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `sensei scaffold` CLI command that creates the canonical "spine of record" doc structure into a project, idempotently.

**Architecture:** A new `scaffold` module in the `sensei-cli` crate, mirroring the existing `doctor.rs` module pattern. Logic is split into a **pure** `canonical_layout()` (returns the folder/file spec — trivially unit-testable) and an **IO** `materialize()` (writes the spec under a caller-supplied base path — testable against a tempdir). `main.rs` wires a `Scaffold` subcommand that calls `scaffold::run(cwd)` and prints a `[created]/[exists]` report, exactly like `init_project_scope`.

**Tech Stack:** Rust (edition 2024), `clap` 4 derive, `std::fs`, `tempfile` (dev-dep, already in the workspace lock at 3.27), `chrono` (already a dep).

**Scope note:** This plan builds the **project-level** structure only. The per-feature dossier scaffolder (`sensei scaffold feature <name>`) and memory-anchoring are deliberate follow-on plans. The `--kind code|content` flag is intentionally omitted — the doc structure is *invariant* across project types (only the later baseline tooling differs).

**Spec:** `docs/plan/operating-model.md` §5 (canonical doc structure) + §3.2 (metrics/governance are live surfaces, not folders).

---

## File Structure

- **Create** `crates/cli/src/scaffold.rs` — the whole feature: `Entry`/`Layout` types, pure `canonical_layout()`, IO `materialize()`, `run()`, template functions, unit tests.
- **Modify** `crates/cli/src/main.rs` — add `mod scaffold;`, a `Scaffold` subcommand variant, its dispatch arm, and a `scaffold_cmd()` printer + one parse test.
- **Modify** `crates/cli/Cargo.toml` — add `tempfile` under `[dev-dependencies]`.

Canonical layout produced under `<target>/docs/`:

```
docs/
  README.md            # structure guide (incl. "metrics/governance are live, not folders")
  vision.md            # Intent
  personas/            # Audience
  journeys/            # Outcomes
  roadmap/             # Planner output (phases + value releases)
  design/              # Structure (architecture + design-system reference)
  mockups/             # system-wide mockup (one cohesive artifact)
  features/            # FR/NFR registry
    README.md          # per-feature dossier shape
  decisions.md         # append-only log
```

---

## Task 1: Layout model + pure `canonical_layout()`

**Files:**
- Create: `crates/cli/src/scaffold.rs`
- Modify: `crates/cli/src/main.rs` (add `mod scaffold;`)
- Modify: `crates/cli/Cargo.toml` (dev-dep)

- [ ] **Step 1: Add the `tempfile` dev-dependency**

In `crates/cli/Cargo.toml`, append after the `[dependencies]` block:

```toml
[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 2: Create `scaffold.rs` with types, a stubbed `canonical_layout`, and failing tests**

Create `crates/cli/src/scaffold.rs`:

```rust
//! `sensei scaffold` — create the canonical "spine of record" doc structure.
//!
//! Split into a pure `canonical_layout` (the spec) and an IO `materialize`
//! (writes it), so the structure is unit-testable without touching disk.

use std::fs;
use std::path::Path;

/// One entry in the canonical layout: a directory or a file with contents.
pub enum Entry {
    Dir(String),
    File { path: String, contents: String },
}

impl Entry {
    /// The entry's path relative to the scaffold base (handy for tests + logging).
    pub fn path(&self) -> &str {
        match self {
            Entry::Dir(p) => p,
            Entry::File { path, .. } => path,
        }
    }
}

/// The full set of directories + files the scaffold creates.
pub struct Layout {
    pub entries: Vec<Entry>,
}

/// The canonical doc structure (spec §5). Pure — takes the project name + date
/// and returns the layout; no IO, no clock. Identical for code and non-code
/// projects (the structure is the invariant).
pub fn canonical_layout(_project: &str, _date: &str) -> Layout {
    Layout { entries: vec![] } // stub — implemented in Step 4
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(l: &Layout) -> Vec<&str> {
        l.entries.iter().map(|e| e.path()).collect()
    }

    #[test]
    fn layout_has_every_canonical_slot() {
        let l = canonical_layout("demo", "2026-07-17");
        let p = paths(&l);
        for expected in [
            "docs/README.md",
            "docs/vision.md",
            "docs/personas",
            "docs/journeys",
            "docs/roadmap",
            "docs/design",
            "docs/mockups",
            "docs/features",
            "docs/features/README.md",
            "docs/decisions.md",
        ] {
            assert!(p.contains(&expected), "layout missing {expected}");
        }
    }

    #[test]
    fn metrics_and_governance_are_not_folders() {
        // Spec §3.2: these are live surfaces (Sensei/Dōjō), never doc folders.
        let l = canonical_layout("demo", "2026-07-17");
        for banned in ["docs/metrics", "docs/governance"] {
            assert!(
                !paths(&l).iter().any(|p| *p == banned || p.starts_with(&format!("{banned}/"))),
                "{banned} must not be scaffolded as a folder"
            );
        }
    }

    #[test]
    fn vision_interpolates_project_and_date() {
        let l = canonical_layout("acme", "2026-07-17");
        let vision = l
            .entries
            .iter()
            .find_map(|e| match e {
                Entry::File { path, contents } if path == "docs/vision.md" => Some(contents),
                _ => None,
            })
            .expect("vision.md present");
        assert!(vision.contains("acme"), "vision names the project");
        assert!(vision.contains("2026-07-17"), "vision carries the date");
    }
}
```

Then add the module declaration to `crates/cli/src/main.rs` next to the existing `mod doctor;` (line 12):

```rust
mod doctor;
mod scaffold;
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p sensei-cli scaffold:: -- --nocapture`
Expected: FAIL — `layout_has_every_canonical_slot` and `vision_interpolates_project_and_date` fail (stub returns an empty layout); `metrics_and_governance_are_not_folders` passes vacuously.

- [ ] **Step 4: Implement `canonical_layout` + template helpers**

Replace the stubbed `canonical_layout` in `crates/cli/src/scaffold.rs` with:

```rust
pub fn canonical_layout(project: &str, date: &str) -> Layout {
    let dir = |p: &str| Entry::Dir(p.to_string());
    let file = |p: &str, c: String| Entry::File { path: p.to_string(), contents: c };
    Layout {
        entries: vec![
            file("docs/README.md", readme_md(project, date)),
            file("docs/vision.md", vision_md(project, date)),
            dir("docs/personas"),
            dir("docs/journeys"),
            dir("docs/roadmap"),
            dir("docs/design"),
            dir("docs/mockups"),
            dir("docs/features"),
            file("docs/features/README.md", features_md(project, date)),
            file("docs/decisions.md", decisions_md(project, date)),
        ],
    }
}

fn vision_md(project: &str, date: &str) -> String {
    format!(
        "---\nname: Vision — {project}\nupdated: {date}\n---\n\n# Vision\n\n\
         > The why. What this project is for, who it serves, and what \"good\" looks like.\n\
         > Keep it living — sharpen it as the objective clarifies.\n\n\
         ## Objective\n\n<!-- One paragraph: the problem and the outcome. -->\n\n\
         ## Audience\n\n<!-- Who it's for. See personas/. -->\n\n\
         ## Outcomes\n\n<!-- Experiences/outcomes it must deliver. See journeys/. -->\n"
    )
}

fn decisions_md(project: &str, date: &str) -> String {
    format!(
        "---\nname: Decisions log — {project}\nupdated: {date}\n---\n\n# Decisions\n\n\
         > Append-only. One entry per decision: date · decision · why · alternatives.\n\
         > The anti-rework memory — never re-derive a settled choice.\n"
    )
}

fn readme_md(project: &str, date: &str) -> String {
    format!(
        "---\nname: Docs structure — {project}\nupdated: {date}\n---\n\n\
         # Project docs — the spine of record\n\n\
         Canonical structure (scaffolded by `sensei scaffold`). The *shape* never\n\
         changes; the method that fills it does.\n\n\
         | Path | Slot | Holds |\n|---|---|---|\n\
         | vision.md | Intent | the why / objective |\n\
         | personas/ | Audience | who it's for |\n\
         | journeys/ | Outcomes | user flows |\n\
         | roadmap/ | — | phases + value releases (Planner output) |\n\
         | design/ | Structure | architecture + design-system reference |\n\
         | mockups/ | — | system-wide mockup (one cohesive artifact) |\n\
         | features/ | — | the FR/NFR registry; one dossier per feature |\n\
         | decisions.md | Decisions | append-only log |\n\n\
         **Not folders — live surfaces:** metrics (quality / coverage / churn /\n\
         velocity trends) and governance (rules) are dynamic surfaces in\n\
         Sensei / Dōjō, not docs here.\n"
    )
}

fn features_md(project: &str, date: &str) -> String {
    format!(
        "---\nname: Features — {project}\nupdated: {date}\n---\n\n# Features\n\n\
         The FR/NFR registry — the \"what\". Each feature is a dossier that mirrors\n\
         the project shape:\n\n\
         ```\n\
         <feature>/\n\
         \x20 brief.md        # intent-level: user objective + data (not layout)\n\
         \x20 design.md       # depth dialed by risk; cross-layer contract\n\
         \x20 mockup-ref.md   # link to a section of the system mockup (optional; added later)\n\
         \x20 plan.md         # tasks\n\
         \x20 tests/          # acceptance\n\
         \x20 decisions.md    # learnings\n\
         ```\n"
    )
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p sensei-cli scaffold:: -- --nocapture`
Expected: PASS — all three `scaffold::tests` pass.

- [ ] **Step 6: Commit**

```bash
git add crates/cli/src/scaffold.rs crates/cli/src/main.rs crates/cli/Cargo.toml
git commit -m "feat(cli): canonical doc-scaffold layout (pure)

Spec plan/operating-model.md §5. Pure canonical_layout() returns the spine-of-record
structure; metrics/governance are asserted NOT to be folders (§3.2 — live surfaces).

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: `materialize()` + `ScaffoldReport` (IO, idempotent)

**Files:**
- Modify: `crates/cli/src/scaffold.rs`

- [ ] **Step 1: Add `ScaffoldReport`, a stubbed `materialize`, and failing tempdir tests**

In `crates/cli/src/scaffold.rs`, add after `canonical_layout` (before the `#[cfg(test)]` module):

```rust
/// What a scaffold run did — created vs already-present, in layout order.
#[derive(Default)]
pub struct ScaffoldReport {
    pub created: Vec<String>,
    pub skipped: Vec<String>,
}

/// Write `layout` under `base`, idempotently: existing paths are skipped, never
/// overwritten. Returns the per-path report.
pub fn materialize(_base: &Path, _layout: &Layout) -> ScaffoldReport {
    ScaffoldReport::default() // stub — implemented in Step 3
}
```

Add these tests inside the existing `#[cfg(test)] mod tests` block (after the current tests):

```rust
    #[test]
    fn materialize_creates_every_entry_once() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = canonical_layout("demo", "2026-07-17");
        let report = materialize(tmp.path(), &layout);

        assert_eq!(report.created.len(), layout.entries.len(), "all created");
        assert!(report.skipped.is_empty(), "nothing skipped on a clean dir");
        assert!(tmp.path().join("docs/vision.md").is_file());
        assert!(tmp.path().join("docs/features").is_dir());
        assert!(tmp.path().join("docs/features/README.md").is_file());
        assert!(tmp.path().join("docs/decisions.md").is_file());
    }

    #[test]
    fn materialize_is_idempotent_and_never_overwrites() {
        let tmp = tempfile::tempdir().unwrap();
        let layout = canonical_layout("demo", "2026-07-17");
        materialize(tmp.path(), &layout);

        // Hand-edit a file, then re-run: it must be skipped, not clobbered.
        let vision = tmp.path().join("docs/vision.md");
        fs::write(&vision, "EDITED").unwrap();

        let second = materialize(tmp.path(), &layout);
        assert!(second.created.is_empty(), "second run creates nothing");
        assert_eq!(second.skipped.len(), layout.entries.len(), "all skipped");
        assert_eq!(fs::read_to_string(&vision).unwrap(), "EDITED", "not overwritten");
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p sensei-cli scaffold::tests::materialize -- --nocapture`
Expected: FAIL — stub creates nothing, so `materialize_creates_every_entry_once` fails on the `created.len()` assert.

- [ ] **Step 3: Implement `materialize`**

Replace the stubbed `materialize` in `crates/cli/src/scaffold.rs` with:

```rust
pub fn materialize(base: &Path, layout: &Layout) -> ScaffoldReport {
    let mut report = ScaffoldReport::default();
    for entry in &layout.entries {
        let rel = entry.path().to_string();
        let target = base.join(&rel);
        match entry {
            Entry::Dir(_) => {
                if target.exists() {
                    report.skipped.push(rel);
                } else {
                    fs::create_dir_all(&target).ok();
                    report.created.push(rel);
                }
            }
            Entry::File { contents, .. } => {
                if target.exists() {
                    report.skipped.push(rel);
                } else {
                    if let Some(parent) = target.parent() {
                        fs::create_dir_all(parent).ok();
                    }
                    fs::write(&target, contents).ok();
                    report.created.push(rel);
                }
            }
        }
    }
    report
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p sensei-cli scaffold:: -- --nocapture`
Expected: PASS — all five `scaffold::tests` pass.

- [ ] **Step 5: Commit**

```bash
git add crates/cli/src/scaffold.rs
git commit -m "feat(cli): idempotent materialize() for the doc scaffold

Writes the canonical layout under a base path; existing files are skipped, never
overwritten. Tested against a tempdir (create + idempotent re-run).

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: `run()` + wire the `Scaffold` subcommand

**Files:**
- Modify: `crates/cli/src/scaffold.rs` (add `run`)
- Modify: `crates/cli/src/main.rs` (subcommand + dispatch + printer + parse test)

- [ ] **Step 1: Add `scaffold::run`**

In `crates/cli/src/scaffold.rs`, add after `materialize` (before the test module):

```rust
/// Scaffold the canonical structure into `target`. Derives the project name from
/// the directory name and stamps today's date (via the crate's `format_date`).
pub fn run(target: &Path) -> ScaffoldReport {
    let name = target
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project");
    let date = crate::format_date();
    let layout = canonical_layout(name, &date);
    materialize(target, &layout)
}
```

- [ ] **Step 2: Write the failing parse test in `main.rs`**

In `crates/cli/src/main.rs`, add to the `#[cfg(test)] mod tests` block:

```rust
    #[test]
    fn scaffold_subcommand_parses_with_and_without_path() {
        let bare = Cli::parse_from(["sensei", "scaffold"]);
        match bare.command {
            Some(Commands::Scaffold { path }) => assert!(path.is_none()),
            _ => panic!("expected Scaffold command"),
        }
        let with = Cli::parse_from(["sensei", "scaffold", "--path", "/tmp/x"]);
        match with.command {
            Some(Commands::Scaffold { path }) => assert_eq!(path.as_deref(), Some("/tmp/x")),
            _ => panic!("expected Scaffold command"),
        }
    }
```

- [ ] **Step 3: Run the parse test to verify it fails**

Run: `cargo test -p sensei-cli scaffold_subcommand_parses -- --nocapture`
Expected: FAIL — does not compile: `Commands::Scaffold` variant does not exist yet.

- [ ] **Step 4: Add the subcommand variant, dispatch arm, and printer**

In `crates/cli/src/main.rs`, add a variant to the `Commands` enum (after the `Scan` variant, before `Index`):

```rust
    /// Scaffold the canonical Sensei doc structure into a project
    Scaffold {
        /// Target directory (default: current directory)
        #[arg(long)]
        path: Option<String>,
    },
```

Add a dispatch arm in `main()` (after the `Scan` arm):

```rust
        Some(Commands::Scaffold { path }) => scaffold_cmd(path.as_deref()),
```

Add the printer function (place it near `scan`, in the "Daemon / Scan / AddLib" section):

```rust
fn scaffold_cmd(path: Option<&str>) {
    let target = match path {
        Some(p) => PathBuf::from(p),
        None => match std::env::current_dir() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Error: cannot determine current directory: {e}");
                std::process::exit(1);
            }
        },
    };
    println!("=== sensei scaffold ===\n{}\n", target.display());
    let report = scaffold::run(&target);
    for p in &report.created {
        println!("  [created] {p}");
    }
    for p in &report.skipped {
        println!("  [exists]  {p}");
    }
    println!(
        "\n{} created, {} already present.",
        report.created.len(),
        report.skipped.len()
    );
}
```

- [ ] **Step 5: Run the full CLI test suite to verify everything passes**

Run: `cargo test -p sensei-cli`
Expected: PASS — the new parse test plus all `scaffold::tests` and the pre-existing tests pass.

- [ ] **Step 6: Manual smoke test**

Run:
```bash
cargo run -p sensei-cli -- scaffold --path /tmp/scaffold-smoke
find /tmp/scaffold-smoke -type f -o -type d | sort
```
Expected: prints `[created]` lines; `find` shows `docs/vision.md`, `docs/features/README.md`, `docs/decisions.md`, and the `personas/journeys/roadmap/design/mockups/features` dirs — and NO `docs/metrics` or `docs/governance`. Re-run the same command: every line now reads `[exists]`.

Then clean up: `rm -rf /tmp/scaffold-smoke`

- [ ] **Step 7: Commit**

```bash
git add crates/cli/src/scaffold.rs crates/cli/src/main.rs
git commit -m "feat(cli): sensei scaffold command — canonical doc structure

Wires scaffold::run into a Scaffold subcommand (default cwd, --path override) with a
[created]/[exists] report, matching init_project_scope's idempotent style.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Self-Review

- **Spec coverage:** §5 canonical structure → Tasks 1–3 (all slots scaffolded); §3.2 metrics/governance-are-live → asserted absent in Task 1 Step 2 + README template. Deferred (own plans, noted in Scope): per-feature dossier scaffolder, memory-anchoring, baseline `--kind`.
- **Type consistency:** `Entry`, `Layout`, `ScaffoldReport`, `canonical_layout(&str,&str)`, `materialize(&Path,&Layout)`, `run(&Path)` — names/signatures identical across Tasks 1–3. `Entry::path()` used by both `materialize` and the tests.
- **Placeholders:** none — every step has real code and exact commands.
- **Reuse (CLAUDE.md DRY):** `run()` reuses the existing `crate::format_date()`; `scaffold_cmd` mirrors `init_project_scope`'s cwd + `[created]/[exists]` idiom rather than inventing a new one.

---

## Post-implementation notes (shipped 2026-07-17)

Built via subagent-driven TDD; per-task spec + code-quality reviews + a final whole-feature review (verdict: **Ship**). Commits on `develop`: `b546db74` (pure layout), `0b8c91f3` (materialize), `5bf594a1` (CLI wiring).

**Approved deviations from the plan text (both toward the "no silent errors" house rule):**
1. `ScaffoldReport` gained a `failed: Vec<(String, String)>` field, and `materialize` **records** IO errors (no `.ok()` discards; parent-dir creation chained into the write) instead of the plan's original `.ok()` swallow. Added test `materialize_records_failures_instead_of_swallowing` (read-only base).
2. `scaffold_cmd` prints `[failed]` to stderr and **exits non-zero** when any entry fails.

**Follow-ups (minor, non-blocking — from the final review):**
- Doc-comment the `Entry`/`Layout` fields.
- Add a `--help`/README note that `scaffold` targets **new / pre-structure** projects — on an already-restructured repo it adds the canonical slots *additively* (idempotent skip-if-exists), yielding a hybrid layout.
- If a 4th command needs "resolve target dir or exit 1", extract `resolve_target_dir(Option<&str>) -> PathBuf` (currently duplicated with `init_project_scope`).

**Deferred to later Phase-1 plans:** per-feature dossier scaffolder (`sensei scaffold feature <name>`), memory-anchoring to spine slots, baseline `--kind`.
