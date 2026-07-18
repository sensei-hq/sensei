# Feature-Dossier Scaffolder Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `sensei scaffold feature <name>` — scaffold a per-feature dossier under `docs/features/<name>/`, mirroring the project spine at feature scope, idempotently.

**Architecture:** Extend the existing `scaffold` module (`crates/cli/src/scaffold.rs`). A new **pure** `feature_layout(feature, date) -> Layout` returns the dossier spec; a thin `run_feature(target, feature) -> Result<ScaffoldReport, String>` validates the name (single safe path segment — no traversal), builds the layout, and reuses the already-tested IO `materialize()`. `main.rs` grows a nested `feature <name>` subcommand under `Scaffold`; the no-arg `sensei scaffold` (project scaffold) is unchanged in behavior.

**Tech Stack:** Rust (edition 2024), `clap` 4 derive (subcommand), `std::fs`/`std::path`, `tempfile` (dev-dep, already present), `chrono` (already a dep, via `crate::format_date`).

**Scope note:** Feature scope only. The dossier is the §3.2 feature-scope spine: `brief.md` (Intent), `design.md` (Structure, depth-by-risk + cross-layer contract), `tests/` (Outcomes/Signals — acceptance), `decisions.md` (Decisions), plus `plan.md` (tasks) and `mockup-ref.md` (optional link into the one system mockup). Constraints/governance stay **live** (`get_rules`) — never a folder (§3.2). No registry-index mutation: the `docs/features/` folder listing *is* the registry; each subfolder is a feature. Memory-anchoring + baseline `--kind` remain separate follow-on plans.

**Spec:** `docs/plan/operating-model.md` §3.2 (fractal spine, feature-scope column) + `docs/features/README.md` (dossier shape, shipped by the project scaffolder). **Precedent:** `docs/plan/2026-07-17-doc-scaffold-command.md` (same module, same pure/IO split).

---

## File Structure

- **Modify** `crates/cli/src/scaffold.rs` — add `feature_layout()`, its template helpers, `is_safe_feature_name()`, `run_feature()`, and unit + tempdir tests. (`Entry`, `Layout`, `materialize`, `ScaffoldReport` are reused unchanged.)
- **Modify** `crates/cli/src/main.rs` — refactor the `Scaffold` variant to carry an optional nested `ScaffoldTarget` subcommand, add the `ScaffoldTarget::Feature { name }` variant, update the dispatch arm + `scaffold_cmd`, update the existing parse test, add a feature parse test.

Feature dossier produced under `<target>/docs/features/<name>/`:

```
docs/features/<name>/
  brief.md         # Intent — the chunk's goal + data (not layout)
  design.md        # Structure — depth dialed by risk; cross-layer contract; gates
  plan.md          # tasks
  tests/
    README.md      # Outcomes/Signals — acceptance criteria + coverage (also keeps the dir tracked)
  decisions.md     # Decisions — append-only chunk decisions + learnings
  mockup-ref.md    # optional — link into the one system mockup (docs/mockups/)
```

---

## Task 1: Pure `feature_layout()` + template helpers

**Files:**
- Modify: `crates/cli/src/scaffold.rs`

- [ ] **Step 1: Add a stubbed `feature_layout` + failing tests**

In `crates/cli/src/scaffold.rs`, add after `canonical_layout` (and its template fns), before `ScaffoldReport`:

```rust
/// The per-feature dossier (spec §3.2, feature-scope column). Pure — takes the
/// feature name + date, returns the layout rooted at `docs/features/<feature>/`.
/// Mirrors the project spine, lighter: Intent(brief) · Structure(design) ·
/// Outcomes/Signals(tests) · Decisions(decisions) + plan + optional mockup-ref.
pub fn feature_layout(_feature: &str, _date: &str) -> Layout {
    Layout { entries: vec![] } // stub — implemented in Step 3
}
```

Add these tests inside the existing `#[cfg(test)] mod tests` block (after the current tests):

```rust
    #[test]
    fn feature_layout_has_every_dossier_slot() {
        let l = feature_layout("auth", "2026-07-18");
        let p = paths(&l);
        for expected in [
            "docs/features/auth/brief.md",
            "docs/features/auth/design.md",
            "docs/features/auth/plan.md",
            "docs/features/auth/tests",
            "docs/features/auth/tests/README.md",
            "docs/features/auth/decisions.md",
            "docs/features/auth/mockup-ref.md",
        ] {
            assert!(p.contains(&expected), "dossier missing {expected}");
        }
    }

    #[test]
    fn feature_layout_is_rooted_at_the_named_feature() {
        let l = feature_layout("billing", "2026-07-18");
        assert!(
            l.entries.iter().all(|e| e.path().starts_with("docs/features/billing/")),
            "every entry is under docs/features/<name>/"
        );
    }

    #[test]
    fn feature_layout_governance_is_not_a_slot() {
        // §3.2: constraints/governance are LIVE (get_rules), never a dossier file/folder.
        let l = feature_layout("auth", "2026-07-18");
        assert!(
            !paths(&l).iter().any(|p| p.contains("governance") || p.ends_with("/rules.md")),
            "governance must not be scaffolded into the dossier"
        );
    }

    #[test]
    fn feature_brief_interpolates_name_and_date() {
        let l = feature_layout("checkout", "2026-07-18");
        let brief = l
            .entries
            .iter()
            .find_map(|e| match e {
                Entry::File { path, contents } if path == "docs/features/checkout/brief.md" => Some(contents),
                _ => None,
            })
            .expect("brief.md present");
        assert!(brief.contains("checkout"), "brief names the feature");
        assert!(brief.contains("2026-07-18"), "brief carries the date");
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p sensei-cli scaffold::tests::feature -- --nocapture`
Expected: FAIL — `feature_layout_has_every_dossier_slot`, `feature_layout_is_rooted_at_the_named_feature`, and `feature_brief_interpolates_name_and_date` fail (stub returns empty); `feature_layout_governance_is_not_a_slot` passes vacuously.

- [ ] **Step 3: Implement `feature_layout` + template helpers**

Replace the stubbed `feature_layout` with the real one, and add the template fns immediately after it:

```rust
pub fn feature_layout(feature: &str, date: &str) -> Layout {
    let base = format!("docs/features/{feature}");
    let dir = |p: String| Entry::Dir(p);
    let file = |p: String, c: String| Entry::File { path: p, contents: c };
    Layout {
        entries: vec![
            file(format!("{base}/brief.md"), feature_brief_md(feature, date)),
            file(format!("{base}/design.md"), feature_design_md(feature, date)),
            file(format!("{base}/plan.md"), feature_plan_md(feature, date)),
            dir(format!("{base}/tests")),
            file(format!("{base}/tests/README.md"), feature_tests_md(feature, date)),
            file(format!("{base}/decisions.md"), feature_decisions_md(feature, date)),
            file(format!("{base}/mockup-ref.md"), feature_mockup_ref_md(feature, date)),
        ],
    }
}

fn feature_brief_md(feature: &str, date: &str) -> String {
    format!(
        "---\nname: {feature} — brief\nupdated: {date}\n---\n\n# {feature} — brief\n\n\
         > Intent. The user objective + the data it moves — not the layout.\n\
         > Audience inherits the project personas. Keep it short; sharpen as it clarifies.\n\n\
         ## Goal\n\n<!-- One paragraph: the user objective this chunk delivers. -->\n\n\
         ## Acceptance\n\n<!-- Observable outcomes that mean done. Mirror into tests/. -->\n\n\
         ## Data\n\n<!-- The entities/fields this touches (not screens). -->\n"
    )
}

fn feature_design_md(feature: &str, date: &str) -> String {
    format!(
        "---\nname: {feature} — design\nupdated: {date}\n---\n\n# {feature} — design\n\n\
         > Structure. Depth dialed by risk: shallow for low-risk/known; deep with a\n\
         > fixed cross-layer contract for high blast-radius — settled BEFORE code.\n\n\
         ## Cross-layer contract\n\n\
         <!-- db → api → ui interface the sub-agents build against. Fix it here first. -->\n\n\
         ## Gates that apply\n\n\
         <!-- Rules relevant here — resolved live via get_rules; note the ones that bind. -->\n\n\
         ## Approach\n\n<!-- How it's built, proportional to risk. -->\n"
    )
}

fn feature_plan_md(feature: &str, date: &str) -> String {
    format!(
        "---\nname: {feature} — plan\nupdated: {date}\n---\n\n# {feature} — plan\n\n\
         > Tasks. The build steps for this chunk — bite-sized, TDD, frequent commits.\n"
    )
}

fn feature_tests_md(feature: &str, date: &str) -> String {
    format!(
        "---\nname: {feature} — acceptance\nupdated: {date}\n---\n\n# {feature} — acceptance\n\n\
         > Outcomes / signals. The acceptance criteria for this chunk and the tests that\n\
         > prove them. Coverage for {feature} lives here.\n"
    )
}

fn feature_decisions_md(feature: &str, date: &str) -> String {
    format!(
        "---\nname: {feature} — decisions\nupdated: {date}\n---\n\n# {feature} — decisions\n\n\
         > Append-only. One entry per decision: date · decision · why · alternatives.\n\
         > The chunk's anti-rework memory — never re-derive a settled choice.\n"
    )
}

fn feature_mockup_ref_md(feature: &str, date: &str) -> String {
    format!(
        "---\nname: {feature} — mockup ref\nupdated: {date}\n---\n\n# {feature} — mockup reference\n\n\
         > Optional. Link to the section of the system-wide mockup (docs/mockups/) this\n\
         > feature realizes. The mockup is one cohesive artifact — point here, don't fork it.\n"
    )
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p sensei-cli scaffold::tests::feature -- --nocapture`
Expected: PASS — all four new `feature_*` tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/cli/src/scaffold.rs
git commit -m "feat(cli): pure feature_layout() — per-feature dossier spec

Spec plan/operating-model.md §3.2 (feature-scope spine). Pure feature_layout()
returns the dossier rooted at docs/features/<name>/ (brief/design/plan/tests/
decisions + optional mockup-ref); governance asserted NOT a slot (live via get_rules).

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: `run_feature()` + name safety (IO, idempotent via `materialize`)

**Files:**
- Modify: `crates/cli/src/scaffold.rs`

- [ ] **Step 1: Add `is_safe_feature_name`, a stubbed `run_feature`, and failing tests**

In `crates/cli/src/scaffold.rs`, add after `run` (before the test module):

```rust
/// A feature name must be a single, safe path segment — no separators, no `..`,
/// non-empty. Guards against scaffolding outside `docs/features/` (path traversal).
pub fn is_safe_feature_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains("..")
}

/// Scaffold a feature dossier under `target/docs/features/<feature>/`. Validates the
/// name, stamps today's date, and reuses the idempotent `materialize`. Returns an
/// Err with a message on an unsafe name (nothing is written).
pub fn run_feature(_target: &Path, _feature: &str) -> Result<ScaffoldReport, String> {
    Err("stub".to_string()) // implemented in Step 3
}
```

Add these tests inside the `#[cfg(test)] mod tests` block:

```rust
    #[test]
    fn run_feature_creates_the_dossier_once() {
        let tmp = tempfile::tempdir().unwrap();
        let report = run_feature(tmp.path(), "auth").expect("safe name");
        assert!(report.created.contains(&"docs/features/auth/brief.md".to_string()));
        assert!(tmp.path().join("docs/features/auth/brief.md").is_file());
        assert!(tmp.path().join("docs/features/auth/tests").is_dir());
        assert!(tmp.path().join("docs/features/auth/tests/README.md").is_file());
        assert!(tmp.path().join("docs/features/auth/decisions.md").is_file());
        assert!(report.skipped.is_empty() && report.failed.is_empty());
    }

    #[test]
    fn run_feature_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        run_feature(tmp.path(), "auth").unwrap();
        // Hand-edit, re-run: skipped, not clobbered.
        let brief = tmp.path().join("docs/features/auth/brief.md");
        fs::write(&brief, "EDITED").unwrap();
        let second = run_feature(tmp.path(), "auth").unwrap();
        assert!(second.created.is_empty(), "second run creates nothing");
        assert_eq!(fs::read_to_string(&brief).unwrap(), "EDITED", "not overwritten");
    }

    #[test]
    fn run_feature_rejects_unsafe_names() {
        let tmp = tempfile::tempdir().unwrap();
        for bad in ["../evil", "a/b", "..", "", "."] {
            assert!(run_feature(tmp.path(), bad).is_err(), "must reject {bad:?}");
        }
        // Nothing leaked outside docs/features/.
        assert!(!tmp.path().join("../evil").exists());
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p sensei-cli scaffold::tests::run_feature -- --nocapture`
Expected: FAIL — `run_feature_creates_the_dossier_once` and `run_feature_is_idempotent` fail (stub returns Err); `run_feature_rejects_unsafe_names` passes (stub is always Err — acceptable, it'll still pass after the real impl).

- [ ] **Step 3: Implement `run_feature`**

Replace the stubbed `run_feature`:

```rust
pub fn run_feature(target: &Path, feature: &str) -> Result<ScaffoldReport, String> {
    if !is_safe_feature_name(feature) {
        return Err(format!(
            "invalid feature name {feature:?} — use a single path segment (no '/', '\\', or '..')"
        ));
    }
    let date = crate::format_date();
    let layout = feature_layout(feature, &date);
    Ok(materialize(target, &layout))
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p sensei-cli scaffold:: -- --nocapture`
Expected: PASS — all `scaffold::tests` (project + feature) pass.

- [ ] **Step 5: Commit**

```bash
git add crates/cli/src/scaffold.rs
git commit -m "feat(cli): run_feature() — validated, idempotent dossier scaffold

Builds feature_layout + reuses materialize; rejects unsafe names (no '/','\\','..')
so a feature name can never scaffold outside docs/features/. Tempdir-tested:
create · idempotent re-run · traversal rejection.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: Wire `sensei scaffold feature <name>`

**Files:**
- Modify: `crates/cli/src/main.rs` (Scaffold variant → nested subcommand; dispatch; `scaffold_cmd`; tests)

- [ ] **Step 1: Update the existing parse test + add the feature parse test (both failing)**

In `crates/cli/src/main.rs`, **replace** the existing `scaffold_subcommand_parses_with_and_without_path` test with the two below (the existing one won't compile once `Scaffold` gains a field, so it must be updated in lockstep):

```rust
    #[test]
    fn scaffold_subcommand_parses_project_form() {
        let bare = Cli::parse_from(["sensei", "scaffold"]);
        match bare.command {
            Some(Commands::Scaffold { what, path }) => {
                assert!(what.is_none());
                assert!(path.is_none());
            }
            _ => panic!("expected Scaffold command"),
        }
        let with = Cli::parse_from(["sensei", "scaffold", "--path", "/tmp/x"]);
        match with.command {
            Some(Commands::Scaffold { path, .. }) => assert_eq!(path.as_deref(), Some("/tmp/x")),
            _ => panic!("expected Scaffold command"),
        }
    }

    #[test]
    fn scaffold_feature_subcommand_parses() {
        let cli = Cli::parse_from(["sensei", "scaffold", "feature", "auth"]);
        match cli.command {
            Some(Commands::Scaffold { what: Some(ScaffoldTarget::Feature { name }), .. }) => {
                assert_eq!(name, "auth");
            }
            _ => panic!("expected Scaffold feature command"),
        }
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p sensei-cli scaffold_ -- --nocapture`
Expected: FAIL — does not compile: `Scaffold` has no `what` field and `ScaffoldTarget` does not exist yet.

- [ ] **Step 3: Refactor the `Scaffold` variant + add `ScaffoldTarget`**

In `crates/cli/src/main.rs`, **replace** the existing `Scaffold` variant in the `Commands` enum:

```rust
    /// Scaffold the canonical Sensei doc structure into a project
    Scaffold {
        /// What to scaffold (default: the project structure)
        #[command(subcommand)]
        what: Option<ScaffoldTarget>,
        /// Target directory (default: current directory)
        #[arg(long, global = true)]
        path: Option<String>,
    },
```

Add the subcommand enum near the `Commands` enum (after it is fine). Ensure `Subcommand` is imported (the file already derives `clap::Subcommand` on `Commands`, so the trait is in scope via `clap::`; add `Subcommand` to the existing `use clap::{...}` if not already there):

```rust
/// `sensei scaffold <what>` targets. Absent = the project-level doc structure.
#[derive(clap::Subcommand, Debug)]
enum ScaffoldTarget {
    /// Scaffold a per-feature dossier under docs/features/<name>/
    Feature {
        /// Feature name (a single path segment → docs/features/<name>/)
        name: String,
    },
}
```

- [ ] **Step 4: Update the dispatch arm + `scaffold_cmd`**

**Replace** the dispatch arm in `main()`:

```rust
        Some(Commands::Scaffold { what, path }) => scaffold_cmd(what, path.as_deref()),
```

**Replace** `scaffold_cmd` with a version that branches on `what`:

```rust
fn scaffold_cmd(what: Option<ScaffoldTarget>, path: Option<&str>) {
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
    let report = match what {
        Some(ScaffoldTarget::Feature { name }) => {
            println!("=== sensei scaffold feature {name} ===\n{}\n", target.display());
            match scaffold::run_feature(&target, &name) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("Error: {e}");
                    std::process::exit(1);
                }
            }
        }
        None => {
            println!("=== sensei scaffold ===\n{}\n", target.display());
            scaffold::run(&target)
        }
    };
    for p in &report.created {
        println!("  [created] {p}");
    }
    for p in &report.skipped {
        println!("  [exists]  {p}");
    }
    for (p, err) in &report.failed {
        eprintln!("  [failed]  {p}: {err}");
    }
    println!(
        "\n{} created, {} already present{}.",
        report.created.len(),
        report.skipped.len(),
        if report.failed.is_empty() {
            String::new()
        } else {
            format!(", {} failed", report.failed.len())
        }
    );
    if !report.failed.is_empty() {
        std::process::exit(1);
    }
}
```

> Note: `report.failed` is the `Vec<(String, String)>` field added when the project scaffolder shipped (see the doc-scaffold plan's post-impl notes). If the local `scaffold_cmd` did not previously print `failed`, this brings it in line; keep whatever the current file already does for `failed` if it differs.

- [ ] **Step 5: Run the full CLI test suite**

Run: `cargo test -p sensei-cli`
Expected: PASS — both scaffold parse tests, all `scaffold::tests`, and pre-existing tests pass.

- [ ] **Step 6: Clippy (zero-warnings house gate)**

Run: `cargo clippy -p sensei-cli -- -D warnings`
Expected: no warnings.

- [ ] **Step 7: Manual smoke test**

```bash
cargo run -p sensei-cli -- scaffold --path /tmp/scaffold-smoke              # project first
cargo run -p sensei-cli -- scaffold feature auth --path /tmp/scaffold-smoke # then a feature
find /tmp/scaffold-smoke/docs/features -type f -o -type d | sort
cargo run -p sensei-cli -- scaffold feature auth --path /tmp/scaffold-smoke # re-run → [exists]
cargo run -p sensei-cli -- scaffold feature ../evil --path /tmp/scaffold-smoke  # → Error, exit 1
```
Expected: the `auth` dossier appears (`brief.md`, `design.md`, `plan.md`, `tests/README.md`, `decisions.md`, `mockup-ref.md`); the re-run reads `[exists]`; the `../evil` run prints an error and exits non-zero and creates nothing. Clean up: `rm -rf /tmp/scaffold-smoke`.

- [ ] **Step 8: Commit**

```bash
git add crates/cli/src/main.rs
git commit -m "feat(cli): sensei scaffold feature <name> — per-feature dossier

Nested Scaffold subcommand: bare = project structure (unchanged), feature <name> =
docs/features/<name>/ dossier via scaffold::run_feature. [created]/[exists]/[failed]
report + non-zero exit on failure/unsafe name.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Self-Review

- **Spec coverage:** §3.2 feature-scope spine → Task 1 (`brief`=Intent, `design`=Structure/depth-by-risk+contract, `tests/`=Outcomes/Signals, `decisions`=Decisions; `plan.md` + optional `mockup-ref.md`); governance-is-live → asserted absent (`feature_layout_governance_is_not_a_slot`) + noted in `design.md`'s "gates" template. Path-traversal safety → Task 2 (`is_safe_feature_name` + rejection test). CLI syntax `sensei scaffold feature <name>` → Task 3. Deferred (own plans, per Scope): memory-anchoring, baseline `--kind`.
- **Type consistency:** `feature_layout(&str,&str) -> Layout`, `is_safe_feature_name(&str) -> bool`, `run_feature(&Path,&str) -> Result<ScaffoldReport,String>`, `ScaffoldTarget::Feature { name: String }`, `scaffold_cmd(Option<ScaffoldTarget>, Option<&str>)` — names/signatures identical across tasks. Reuses `Entry`/`Layout`/`materialize`/`ScaffoldReport{created,skipped,failed}`/`crate::format_date` unchanged.
- **Placeholders:** none — every step has real code + exact commands.
- **Reuse (CLAUDE.md DRY):** `run_feature` reuses `materialize` (no second IO path); `feature_layout` reuses `Entry`/`Layout`; templates mirror the project `*_md` helpers' frontmatter+voice; `scaffold_cmd` keeps the one `[created]/[exists]/[failed]` printer for both forms.

---

## Post-implementation notes (shipped 2026-07-18)

Built via inline TDD (red → green per task). Commits on `develop`: `0f15480e`
(pure `feature_layout` + `run_feature`, Tasks 1–2 combined) · `8723520a` (CLI
wiring, Task 3). 13 `scaffold::tests` + 24 `sensei-cli` tests green, `clippy -D
warnings` clean, smoke-verified (project scaffold · feature dossier 7 entries ·
idempotent re-run · `../evil` rejected exit 1, no leak).

**Reviewer** (`feature-dev:code-reviewer`): **no high-confidence issues.** Confirmed
the name-safety guard sits on the single write path and covers all realistic traversal
vectors on the target platforms (absolute paths / drive letters / `~` / unicode
separators all inert or captured, never escape); idempotency via the reused
`materialize`; clap nested-optional-subcommand + `global` path arg is standard; DRY
(reuses `Entry`/`Layout`/`materialize`/`ScaffoldReport`/`format_date`); no silent-error
discards; dossier slots match §3.2 with governance correctly excluded.

**No deviations from the plan.** Executed as written.

**Deferred to later Phase-1 plans:** memory-anchoring (L0/L1/L2 + memories → spine
slots), baseline capability-contract scaffold (`--kind`).
