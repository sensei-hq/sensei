# Baseline Capability-Contract Scaffold Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `sensei scaffold baseline --kind <code|content>` — scaffold the baseline **capability-contract** record (`docs/baseline.md`), kind-adapted, idempotently.

**Architecture:** Extend the `scaffold` module again. A pure `baseline_layout(kind, date) -> Layout` returns a single-file layout (`docs/baseline.md`); a `baseline_md(kind, date)` template renders the capability table for the kind (code vs non-code adapters) + the default gate line + the governance-is-live note. `run_baseline(target, kind) -> ScaffoldReport` reuses the idempotent `materialize`. `main.rs` adds a `ScaffoldTarget::Baseline { kind }` subcommand with a `clap::ValueEnum` `BaselineKind` (default `code`). The project scaffold stays invariant — baseline is a separate, opt-in "installed once at project start" step (spec §3.6).

**Tech Stack:** Rust (edition 2024), `clap` 4 derive (`ValueEnum`), reuses `Entry`/`Layout`/`materialize`/`ScaffoldReport`/`crate::format_date`.

**Scope note:** This scaffolds the **contract record** only — the declaration of *what capabilities* the project holds and *which adapter* fills each. Actual stack detection + concrete tool install + conformance scoring **ride the manifest-adapter / `get_commands` / `detect_toolchain` direction** (§3.6, §607) and are out of scope here. Governance (the strictness layer) stays **live** via `get_rules` — never scaffolded (§3.6). `--kind` values are `code` and `content` (the spec's two adapter columns).

**Spec:** `docs/plan/operating-model.md` §3.6 (capability contract table + default gate line + governance-is-live). **Precedent:** `docs/plan/2026-07-18-feature-dossier-scaffolder.md` (same module, same pure/IO/subcommand pattern).

---

## File Structure

- **Modify** `crates/cli/src/scaffold.rs` — add `BaselineKind` (pub enum), `baseline_layout()`, `baseline_md()`, `run_baseline()`, and unit + tempdir tests. (`Entry`/`Layout`/`materialize`/`ScaffoldReport` reused unchanged.)
- **Modify** `crates/cli/src/main.rs` — add `ScaffoldTarget::Baseline { kind }` with a `clap::ValueEnum` `BaselineKind`; extend the `scaffold_cmd` match; add a parse test.

Produced under `<target>/`:

```
docs/baseline.md      # the capability contract, adapted to --kind
```

---

## Task 1: `BaselineKind` + pure `baseline_layout()` + template

**Files:**
- Modify: `crates/cli/src/scaffold.rs`

- [ ] **Step 1: Add `BaselineKind`, a stubbed `baseline_layout`, and failing tests**

In `crates/cli/src/scaffold.rs`, add after `run_feature` (before the test module):

```rust
/// The kind of project a baseline contract targets — selects the adapter column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BaselineKind {
    Code,
    Content,
}

impl BaselineKind {
    /// Stable slug used in the contract's frontmatter.
    pub fn slug(self) -> &'static str {
        match self {
            BaselineKind::Code => "code",
            BaselineKind::Content => "content",
        }
    }
}

/// The baseline capability-contract layout (spec §3.6). Pure — one file,
/// `docs/baseline.md`, rendered for the given kind.
pub fn baseline_layout(_kind: BaselineKind, _date: &str) -> Layout {
    Layout { entries: vec![] } // stub — implemented in Step 3
}
```

Add these tests inside the `#[cfg(test)] mod tests` block:

```rust
    #[test]
    fn baseline_layout_is_a_single_contract_file() {
        let l = baseline_layout(BaselineKind::Code, "2026-07-18");
        assert_eq!(paths(&l), vec!["docs/baseline.md"]);
    }

    #[test]
    fn baseline_code_names_code_tools_and_the_gate() {
        let l = baseline_layout(BaselineKind::Code, "2026-07-18");
        let md = match &l.entries[0] {
            Entry::File { contents, .. } => contents,
            _ => panic!("baseline.md is a file"),
        };
        assert!(md.contains("kind: code"), "frontmatter carries the kind");
        assert!(md.contains("2026-07-18"), "carries the date");
        assert!(md.contains("eslint") || md.contains("clippy"), "code lint adapter");
        assert!(md.contains("80%"), "the ≥80% coverage gate");
        assert!(md.contains("get_rules"), "governance is live, not scaffolded");
    }

    #[test]
    fn baseline_content_uses_non_code_adapters() {
        let l = baseline_layout(BaselineKind::Content, "2026-07-18");
        let md = match &l.entries[0] {
            Entry::File { contents, .. } => contents,
            _ => panic!("baseline.md is a file"),
        };
        assert!(md.contains("kind: content"), "frontmatter carries the kind");
        assert!(md.contains("grammar") || md.contains("tone"), "content lint adapter");
        assert!(!md.contains("eslint"), "no code tools in the content contract");
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p sensei-cli scaffold::tests::baseline -- --nocapture`
Expected: FAIL — the three `baseline_*` tests fail (stub returns empty).

- [ ] **Step 3: Implement `baseline_layout` + `baseline_md`**

Replace the stubbed `baseline_layout` and add `baseline_md` after it:

```rust
pub fn baseline_layout(kind: BaselineKind, date: &str) -> Layout {
    Layout {
        entries: vec![Entry::File {
            path: "docs/baseline.md".to_string(),
            contents: baseline_md(kind, date),
        }],
    }
}

fn baseline_md(kind: BaselineKind, date: &str) -> String {
    // Capability × adapter, per §3.6. One row per capability; the adapter column
    // switches on kind.
    let rows: &[(&str, &str)] = match kind {
        BaselineKind::Code => &[
            ("Format", "prettier / rustfmt"),
            ("Lint", "eslint / clippy"),
            ("Unit test", "vitest / cargo test"),
            ("Flow test", "Playwright e2e"),
            ("Coverage", "coverage %"),
            ("Quality", "qlty.sh score"),
            ("Security", "semgrep / deps scan"),
            ("Churn + velocity", "git signal"),
            ("Design system", "rokkit tokens + component catalog (no hand-rolled primitives)"),
        ],
        BaselineKind::Content => &[
            ("Format", "style-guide conformance"),
            ("Lint", "grammar / tone"),
            ("Unit test", "fact / continuity check"),
            ("Integration", "chapter-to-chapter coherence"),
            ("Flow test", "full read-through / arc check"),
            ("Coverage", "outline coverage"),
            ("Quality", "readability / pacing"),
            ("Churn + velocity", "draft-revision signal"),
            ("Design system", "template / layout system"),
        ],
    };
    let mut table = String::from("| Capability | Adapter |\n|---|---|\n");
    for (cap, adapter) in rows {
        table.push_str(&format!("| {cap} | {adapter} |\n"));
    }
    format!(
        "---\nname: Baseline — capability contract\nkind: {kind}\nupdated: {date}\n---\n\n\
         # Baseline — the capability contract\n\n\
         > The capabilities every change must satisfy — a *contract*, not a fixed toolset.\n\
         > Installed once at project start (recommend-and-confirm); after that it's just\n\
         > `bun run x` / `make x`. Sensei detects the stack and fills concrete tools\n\
         > (rides the manifest-adapter); conformance streams into the Signals slot as the\n\
         > project health score.\n\n\
         {table}\n\
         ## Gates\n\n\
         - **Security scan — block.**\n\
         - **Test coverage ≥ 80% — block** (org-tunable via Dōjō).\n\
         - **Quality floor — block.**\n\n\
         Everything else is installed + guided, ratcheting up over time.\n\n\
         ## Governance\n\n\
         The strictness layer is **live**, not recorded here — resolved at the point of\n\
         work via `get_rules` (org / Dōjō top-down + contributed bottom-up); mandatory\n\
         rules are non-overridable. This file records the *contract*; governance sets the\n\
         *strictness*.\n",
        kind = kind.slug()
    )
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p sensei-cli scaffold::tests::baseline -- --nocapture`
Expected: PASS — all three `baseline_*` tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/cli/src/scaffold.rs
git commit -m "feat(cli): pure baseline_layout() — capability-contract record

Spec plan/operating-model.md §3.6. Pure baseline_layout(kind,date) renders
docs/baseline.md — the capability × adapter table (code vs content), the default
gate line (security + ≥80% coverage + quality block), and the governance-is-live
note (get_rules, never scaffolded). Detection/install ride the manifest-adapter.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: `run_baseline()` (IO, idempotent)

**Files:**
- Modify: `crates/cli/src/scaffold.rs`

- [ ] **Step 1: Add a stubbed `run_baseline` + failing tests**

In `crates/cli/src/scaffold.rs`, add after `baseline_md` (before the test module):

```rust
/// Scaffold the baseline contract into `target/docs/baseline.md`. Stamps today's
/// date and reuses the idempotent `materialize`.
pub fn run_baseline(_target: &Path, _kind: BaselineKind) -> ScaffoldReport {
    ScaffoldReport::default() // stub — implemented in Step 3
}
```

Add these tests inside the `#[cfg(test)] mod tests` block:

```rust
    #[test]
    fn run_baseline_writes_the_contract() {
        let tmp = tempfile::tempdir().unwrap();
        let report = run_baseline(tmp.path(), BaselineKind::Code);
        assert_eq!(report.created, vec!["docs/baseline.md".to_string()]);
        assert!(tmp.path().join("docs/baseline.md").is_file());
        assert!(report.failed.is_empty());
    }

    #[test]
    fn run_baseline_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        run_baseline(tmp.path(), BaselineKind::Code);
        let f = tmp.path().join("docs/baseline.md");
        fs::write(&f, "EDITED").unwrap();
        let second = run_baseline(tmp.path(), BaselineKind::Code);
        assert!(second.created.is_empty(), "second run creates nothing");
        assert_eq!(fs::read_to_string(&f).unwrap(), "EDITED", "not overwritten");
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p sensei-cli scaffold::tests::run_baseline -- --nocapture`
Expected: FAIL — `run_baseline_writes_the_contract` fails (stub creates nothing); `run_baseline_is_idempotent` fails on the `created.is_empty()` assert path (second run also creates nothing, but the file was never made — the `fs::write` still works, so it may pass vacuously; the first test is the real red).

- [ ] **Step 3: Implement `run_baseline`**

Replace the stubbed `run_baseline`:

```rust
pub fn run_baseline(target: &Path, kind: BaselineKind) -> ScaffoldReport {
    let date = crate::format_date();
    let layout = baseline_layout(kind, &date);
    materialize(target, &layout)
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p sensei-cli scaffold:: -- --nocapture`
Expected: PASS — all `scaffold::tests` pass.

- [ ] **Step 5: Commit**

```bash
git add crates/cli/src/scaffold.rs
git commit -m "feat(cli): run_baseline() — idempotent capability-contract scaffold

Builds baseline_layout + reuses materialize (skip-if-exists). Tempdir-tested:
create + idempotent re-run (never clobbers an edited baseline.md).

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: Wire `sensei scaffold baseline --kind <code|content>`

**Files:**
- Modify: `crates/cli/src/main.rs`

- [ ] **Step 1: Add the failing parse test**

In `crates/cli/src/main.rs`, add to the `#[cfg(test)] mod tests` block:

```rust
    #[test]
    fn scaffold_baseline_subcommand_parses_kind() {
        let cli = Cli::parse_from(["sensei", "scaffold", "baseline", "--kind", "content"]);
        match cli.command {
            Some(Commands::Scaffold {
                what: Some(ScaffoldTarget::Baseline { kind }),
                ..
            }) => assert_eq!(kind, scaffold::BaselineKind::Content),
            _ => panic!("expected Scaffold baseline command"),
        }
        // default kind = code
        let d = Cli::parse_from(["sensei", "scaffold", "baseline"]);
        match d.command {
            Some(Commands::Scaffold {
                what: Some(ScaffoldTarget::Baseline { kind }),
                ..
            }) => assert_eq!(kind, scaffold::BaselineKind::Code),
            _ => panic!("expected Scaffold baseline command"),
        }
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p sensei-cli scaffold_baseline -- --nocapture`
Expected: FAIL — does not compile: `ScaffoldTarget::Baseline` does not exist; `BaselineKind` is not `clap::ValueEnum` / `PartialEq`-comparable in the arg yet.

- [ ] **Step 3: Make `BaselineKind` a clap `ValueEnum` + add the subcommand variant**

In `crates/cli/src/scaffold.rs`, extend the `BaselineKind` derive so clap can parse it as an arg value (rename variants to lowercase on the CLI):

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
#[value(rename_all = "lower")]
pub enum BaselineKind {
    Code,
    Content,
}
```

In `crates/cli/src/main.rs`, add a variant to the `ScaffoldTarget` enum (after `Feature`):

```rust
    /// Scaffold the baseline capability contract (docs/baseline.md)
    Baseline {
        /// Project kind — selects the adapter column
        #[arg(long, value_enum, default_value_t = scaffold::BaselineKind::Code)]
        kind: scaffold::BaselineKind,
    },
```

- [ ] **Step 4: Extend `scaffold_cmd` to handle the Baseline arm**

In `crates/cli/src/main.rs`, add a match arm inside `scaffold_cmd`'s `let report = match what { ... }` (alongside `Feature` and `None`):

```rust
        Some(ScaffoldTarget::Baseline { kind }) => {
            println!(
                "=== sensei scaffold baseline --kind {} ===\n{}\n",
                kind.slug(),
                target.display()
            );
            scaffold::run_baseline(&target, kind)
        }
```

- [ ] **Step 5: Run the full CLI test suite**

Run: `cargo test -p sensei-cli`
Expected: PASS — the new baseline parse test + all `scaffold::tests` + pre-existing tests pass.

- [ ] **Step 6: Clippy**

Run: `cargo clippy -p sensei-cli -- -D warnings`
Expected: no warnings.

- [ ] **Step 7: Manual smoke test**

```bash
cargo run -p sensei-cli -- scaffold baseline --path /tmp/baseline-smoke                # default: code
cargo run -p sensei-cli -- scaffold baseline --kind content --path /tmp/baseline-smoke # content (skips existing)
sed -n '1,12p' /tmp/baseline-smoke/docs/baseline.md
cargo run -p sensei-cli -- scaffold baseline --path /tmp/baseline-smoke                # re-run → [exists]
```
Expected: first run creates `docs/baseline.md` with `kind: code` + code adapters + the ≥80% gate; the content run reports `[exists]` (idempotent — does not overwrite); re-run reads `[exists]`. Clean up: `rm -rf /tmp/baseline-smoke`.

> Note: a second `scaffold baseline` with a different `--kind` on an existing file is a **no-op** (materialize skips existing) — the contract is authored once, then hand-tuned. This is intentional; document it in the smoke expectation rather than adding overwrite logic.

- [ ] **Step 8: Commit**

```bash
git add crates/cli/src/scaffold.rs crates/cli/src/main.rs
git commit -m "feat(cli): sensei scaffold baseline --kind <code|content>

Third Scaffold subcommand: scaffolds docs/baseline.md via scaffold::run_baseline.
BaselineKind is a clap ValueEnum (default code). Shared [created]/[exists] printer.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Self-Review

- **Spec coverage:** §3.6 capability table → Task 1 (`baseline_md` code + content columns); default gate line (security + ≥80% coverage + quality block) → asserted in `baseline_code_names_code_tools_and_the_gate`; governance-is-live → asserted (`get_rules`) + the Governance section; kinds `code|content` → `BaselineKind`. Deferred (own direction, per Scope): stack detection + tool install + conformance scoring (manifest-adapter, §607).
- **Type consistency:** `BaselineKind{Code,Content}` + `.slug()`, `baseline_layout(BaselineKind,&str)->Layout`, `run_baseline(&Path,BaselineKind)->ScaffoldReport`, `ScaffoldTarget::Baseline{kind: BaselineKind}` — identical across tasks. Reuses `Entry`/`Layout`/`materialize`/`ScaffoldReport`/`format_date`.
- **Placeholders:** none.
- **Reuse (CLAUDE.md DRY):** `run_baseline` reuses `materialize`; the Baseline arm reuses `scaffold_cmd`'s one printer; template mirrors the project/feature `*_md` frontmatter+voice.

---

## Post-implementation notes (shipped 2026-07-18)

Built via inline TDD (red → green). Commits on `develop`: `6c05e6ce` (pure
`baseline_layout` + `run_baseline`, Tasks 1–2) · `910e1177` (CLI wiring, Task 3). 18
`scaffold::tests` + 30 `sensei-cli` tests green, `clippy -D warnings` clean,
smoke-verified (code default · content adapters `grammar / tone` with no `eslint` ·
idempotent no-clobber on re-run / kind-change).

**Reviewer** (`feature-dev:code-reviewer`): **no high-confidence issues.** Confirmed
correctness vs §3.6 (n/a rows dropped rather than rendered — deliberate; gate line +
governance-is-live match), idempotency via reused `materialize`, clap `ValueEnum`
default + both parse forms, DRY reuse, no silent-error discards, and **no traversal
surface** (kind is a closed enum — unlike the feature name, no user string reaches a
path segment).

**No deviations from the plan** (implemented essentially verbatim).

**Last Phase-1 sub-unit remaining:** memory-anchoring (L0/L1/L2 + memories → spine
slots) — needs a brainstorm (touches the memory system, not just the CLI).
