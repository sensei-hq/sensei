# Brew resolver split — design

**Date:** 2026-05-14
**Status:** Draft for review
**Replaces:** `crates/bootstrap/src/health/resolvers/brew_bundle.rs` (`BrewBundleResolver`); `sensei/homebrew/Brewfile`; `sensei/homebrew/Brewfile-dev`; the `brew bundle --file=…` cold-install paths in `make install-dev`, README onramps, and the tap-side `homebrew-tap/Brewfile*` files.

---

## 1. Intent

`BrewBundleResolver` lumps three independent prerequisites — postgresql@17, ollama, and sensei — into a single `brew bundle` invocation. That coupling produces two failure modes we have hit in practice:

1. **Cross-component cascade.** A user with ollama installed via the official `.dmg` (the menubar `Ollama.app` plus its `/opt/homebrew/bin/ollama` symlink) cannot `brew bundle` the file, because `brew install ollama` hits a link conflict and aborts. The unrelated sensei step never runs.
2. **Spurious failure on HEAD-only formulas.** `brew bundle` with `args: ["HEAD"]` reports `Installing … has failed!` and exits non-zero after `brew install --HEAD` itself succeeded. `make install-dev` then exits 1 even though the binaries are installed and working.

The fix is to align the resolver layer with the rest of the architecture: one resolver per `ComponentId`, each owning *its* install path, each escalating to `NeedsHumanAction` only when its own brew step fails. Failures stay isolated; remedies are component-specific; the bundle file is no longer a load-bearing abstraction.

This change is server-side only. The TS health UI in Phase 1c consumes the same `HealthPayload` wire shape (`Resolved | NeedsHumanAction(Remedy)`); no contract change.

---

## 2. Architecture

### Resolver layout (before vs after)

```
BEFORE                                       AFTER
─────────────────────────────────────        ─────────────────────────────────────────────
health/resolvers/                            health/resolvers/
├── brew_bundle.rs      [3 components]       ├── postgres_install.rs   [Postgres]
├── daemon_start.rs     [Daemon]             ├── ollama_install.rs     [Ollama]
└── db_setup.rs         [Database]           ├── sensei_install.rs     [Sensei]
                                             ├── daemon_start.rs       [Daemon]      (unchanged)
                                             ├── db_setup.rs           [Database]    (unchanged)
                                             └── brew_helpers.rs       (shared)
```

### Invocation flow

```
              ┌──────────────────┐
              │   Orchestrator   │
              └────────┬─────────┘
                       │ for each failed ComponentId
                       ▼
              ┌──────────────────┐
              │   Checker        │   (already failed — that's why we're here)
              └────────┬─────────┘
                       │
                       ▼
              ┌──────────────────┐
              │   Resolver       │   one of: PostgresInstall / OllamaInstall / SenseiInstall
              └────────┬─────────┘
                       │ calls
                       ▼
              ┌──────────────────┐
              │  brew_install()  │   shared helper — runs `brew install [args] <formula>`,
              │  in brew_helpers │   parses stderr, returns Result<(), BrewError>
              └────────┬─────────┘
                       │
        ┌──────────────┼──────────────┬──────────────────┐
        ▼              ▼              ▼                  ▼
    Resolved   NeedsHumanAction   NeedsHumanAction   NeedsHumanAction
               (homebrew_install) (overwrite_link)   (tap_missing | generic)
```

Resolvers do not pre-check. The orchestrator only invokes a resolver after the corresponding checker has failed. That means the dmg-ollama / Postgres.app cases — where the binary works fine — never reach the resolver at all; the checker already returned green and the orchestrator moved on.

---

## 3. Components

### 3.1 `brew_helpers.rs` (new shared module)

```rust
pub enum BrewError {
    BrewNotFound,
    LinkConflict { path: PathBuf },
    TapMissing,
    Other(String),  // last 500 chars of stderr
}

pub fn brew_install(formula: &str, args: &[&str]) -> Result<(), BrewError>;

// Extracted as pure function for exhaustive unit testing.
pub(crate) fn parse_brew_error(stderr: &str) -> BrewError;
```

`brew_install` resolves `brew` via `crate::util::which_binary("brew")`. On `None` returns `BrewNotFound` without shelling out. Otherwise runs `brew install <args...> <formula>`, captures stderr, and on non-zero exit dispatches to `parse_brew_error`.

`parse_brew_error` substring-matches against:
- `"already exists. You may want to remove it"` — `LinkConflict { path }` (path parsed from the preceding `Target ` line)
- `"No available formula with the name"` — `TapMissing`
- otherwise — `Other(stderr.tail(500))`

### 3.2 `postgres_install.rs`

```rust
pub struct PostgresInstallResolver;

impl Resolver for PostgresInstallResolver {
    fn id(&self) -> &'static str { "postgres_install" }
    fn resolves(&self) -> &'static [ComponentId] { &[ComponentId::Postgres] }
    fn resolve(&self, _targets: &[ComponentId]) -> ResolveOutcome {
        match brew_install("postgresql@17", &[]) {
            Ok(())                       => ResolveOutcome::Resolved,
            Err(BrewError::BrewNotFound) => ResolveOutcome::NeedsHumanAction(homebrew_install_remedy()),
            Err(BrewError::LinkConflict { path })
                                         => ResolveOutcome::NeedsHumanAction(overwrite_link_remedy("postgresql@17", &path)),
            Err(BrewError::TapMissing)   => ResolveOutcome::NeedsHumanAction(tap_missing_remedy("postgresql@17")),
            Err(BrewError::Other(s))     => ResolveOutcome::NeedsHumanAction(generic_brew_remedy("postgresql@17", &s)),
        }
    }
}
```

### 3.3 `ollama_install.rs`

Same shape as `PostgresInstallResolver`. Formula: `"ollama"`. Args: `&[]`.

### 3.4 `sensei_install.rs`

Same shape, but the formula and args branch on `SenseiConfig::is_dev()`:

| Build | Formula                       | Args         |
|-------|-------------------------------|--------------|
| prod  | `sensei-hq/tap/sensei`        | `&[]`        |
| dev   | `sensei-hq/tap/sensei-dev`    | `&["--HEAD"]` |

This routes through `brew install [--HEAD]`, which exits 0 on success — sidestepping the `brew bundle` HEAD-formula false-failure entirely.

### 3.5 Remedy builders (one per error variant, co-located with resolvers)

Each `*_remedy()` function returns a `Remedy { message, script, url }` with messaging tailored to the specific component + failure mode. Patterns (per resolver):

- `homebrew_install_remedy()` — message "Homebrew isn't installed…"; script the standard install one-liner; url `https://brew.sh`. (Identical to the existing implementation in `brew_bundle.rs`.)
- `overwrite_link_remedy(formula, path)` — message "Couldn't link `<formula>` because `<path>` already exists. If you installed it elsewhere (e.g. via .dmg or Postgres.app) you can keep that and skip this. To switch to the brew install, run the script below."; script `brew link --overwrite <formula>`.
- `tap_missing_remedy(formula)` — message "Couldn't find `<formula>`. Run the script below to add the tap, then re-check."; script `brew tap sensei-hq/tap https://github.com/sensei-hq/homebrew-tap && brew install <formula>`.
- `generic_brew_remedy(formula, stderr_tail)` — message "Couldn't install `<formula>` automatically. Last brew output was:\n\n```\n<stderr_tail>\n```\n\nRun the script below to retry."; script `brew install <formula>`.

### 3.6 Orchestrator registry

Wherever `BrewBundleResolver` is registered, replace one entry with three:

```rust
// before
vec![Box::new(BrewBundleResolver), Box::new(DbSetupResolver), Box::new(DaemonStartResolver)]

// after
vec![
    Box::new(PostgresInstallResolver),
    Box::new(OllamaInstallResolver),
    Box::new(SenseiInstallResolver),
    Box::new(DbSetupResolver),
    Box::new(DaemonStartResolver),
]
```

---

## 4. Decisions

### 4.1 Per-component resolvers, not omnibus

Granularity matches `db_setup.rs` and `daemon_start.rs` — one resolver per concern. Considered keeping `BrewBundleResolver` with internal per-component filtering, or a hybrid where per-component resolvers delegate to a shared `BrewInstallStep` struct. Both rejected: omnibus retains single-point-of-failure; hybrid adds polymorphism before duplication exists. `brew_helpers.rs` extracts only the shell-out + parser, which *is* shared.

### 4.2 Accept any working install — don't second-guess

When ollama is from a `.dmg` and `ollama --version` succeeds, the binary checker passes and the resolver is never invoked. We do not detect and switch to brew automatically; the dmg ships the menubar app + launchd autostart that brew doesn't replicate. The user keeps their install.

If the user later runs the daemon and the checker *fails* (binary not on PATH, daemon can't reach it), the resolver runs `brew install`. If that produces a link conflict, the user gets a remedy with `brew link --overwrite` and chooses.

### 4.3 Auto-attempt brew; escalate only on failure

Standardized across all three resolvers. The `Ok` arm of `brew_install` *is* the success case — we already tried brew. `NeedsHumanAction` only appears when brew has failed. Within that:

| Brew outcome | Outcome |
|---|---|
| Exit 0 | `Resolved` (auto-resolved) |
| `LinkConflict` | `NeedsHumanAction` — destructive, user chooses |
| `BrewNotFound` | `NeedsHumanAction` — install homebrew |
| `TapMissing` | `NeedsHumanAction` — add tap |
| Other failure | `NeedsHumanAction(generic + stderr tail)` |

`LinkConflict` is the only "we *could* auto-resolve but won't" case. `brew link --overwrite` deletes the conflicting file at `/opt/homebrew/bin/<name>`; if that file is the dmg-installed ollama, auto-overwriting silently breaks the user's working setup. Hand them the script, let them decide.

### 4.4 Delete `Brewfile` and `Brewfile-dev`

The bundle abstraction is the architectural mistake — it tied three independent installs into one transaction. With per-component resolvers it has no remaining job. New cold-install onramps:

| Audience | Command |
|---|---|
| Prod users | `brew install sensei-hq/tap/sensei` |
| Dev contributors | `brew install --HEAD sensei-hq/tap/sensei-dev` |

Postgres + ollama install lazily on first daemon boot via the resolvers. No file to fetch. One line.

---

## 5. Implementation surface

### Files added
- `sensei/crates/bootstrap/src/health/resolvers/postgres_install.rs`
- `sensei/crates/bootstrap/src/health/resolvers/ollama_install.rs`
- `sensei/crates/bootstrap/src/health/resolvers/sensei_install.rs`
- `sensei/crates/bootstrap/src/health/resolvers/brew_helpers.rs`

### Files removed
- `sensei/crates/bootstrap/src/health/resolvers/brew_bundle.rs`
- `sensei/homebrew/Brewfile`
- `sensei/homebrew/Brewfile-dev`

### Files modified
- `sensei/crates/bootstrap/src/health/resolvers/mod.rs` — module exports
- `sensei/crates/bootstrap/src/health/...` — orchestrator registry call-site (one entry → three)
- `sensei/Makefile`
  - `install-dev` cold-install block: `brew bundle …` → `brew tap … || true && brew install --HEAD sensei-hq/tap/sensei-dev`
  - `install-release` cold-install block (if present): equivalent change for the stable formula
  - `tap-push` target: drop the two `cp homebrew/Brewfile* "$$tmpdir/"` lines
- `sensei-hq/homebrew-tap/README.md` — replace `brew bundle --file=…` onramps with `brew install [--HEAD]`
- Any `sensei/README.md` / `sensei/docs/**` references to `brew bundle --file=…` — grep and update during implementation

### Tap subtree
After all edits land, run `make tap-push`. The tap-side `Brewfile`, `Brewfile-dev`, and any references in `homebrew-tap/README.md` are removed in that push.

---

## 6. Testing

### Unit tests (in-file `#[cfg(test)] mod tests`, matching existing resolver convention)

Per resolver (`postgres_install.rs`, `ollama_install.rs`, `sensei_install.rs`):
- `id_is_<expected>` — locks the string the orchestrator/UI keys on.
- `resolves_only_its_component` — e.g. `OllamaInstallResolver.resolves() == &[Ollama]`.
- `does_not_cover_others` — explicit negative assertion.

For `sensei_install.rs`:
- `dev_build_uses_head_arg` — under `#[cfg(feature = "dev")]`, verify computed formula + args for the dev branch.
- `prod_build_uses_stable_tap` — under `#[cfg(not(feature = "dev"))]`, verify the prod path.

For `brew_helpers.rs` — exhaustive coverage of `parse_brew_error` with captured real-output fixtures:
- `parses_ollama_link_conflict` — full ollama stderr from the original incident → `LinkConflict { path: /opt/homebrew/bin/ollama }`
- `parses_postgres_link_conflict` — synthesized stderr with `Target /opt/homebrew/bin/psql` line → `LinkConflict { path }` with the path field populated from the `Target` line. (Exact conflicting binary varies by environment; the test asserts the parser extracts whatever path appears.)
- `parses_tap_missing` — `Error: No available formula with the name "foo"` → `TapMissing`
- `parses_generic_failure` — arbitrary stderr → `Other(_)`; assert last-500-char truncation
- `extracts_path_from_target_line` — table-driven test of the path-extraction regex/parser

`brew_install` wrapper itself is not unit-tested; manual verification covers the live shell-out.

### Manual verification matrix (recorded here, run during phase 1c re-verification)

| Scenario | Setup | Expected outcome |
|---|---|---|
| Cold install, no conflicts | `brew uninstall sensei-dev` | `make install-dev` succeeds end-to-end, exit 0 |
| Warm install | `sensei-dev` already installed | brew step skipped, overlay-from-`target/debug` runs |
| Ollama link conflict | `/opt/homebrew/bin/ollama` from dmg exists; uninstall brew ollama | Resolver returns `LinkConflict`; UI shows `brew link --overwrite ollama` script |
| Postgres absent | `brew uninstall postgresql@17` | Resolver returns `Resolved` after auto-install; checker passes on re-check |
| No homebrew at all | shell with `PATH` munged to hide `brew` | `BrewNotFound` → install-homebrew remedy presented |
| Phase 1c E2E walk | full cold-deleted system | `/health` page walks `checking → resolving → ok` exactly as 1c spec requires |

### Regression gate

- Existing `brew_bundle.rs` tests are removed with the file (3 tests). The three new resolvers add ≥3 tests each (9 total) plus parser coverage in `brew_helpers.rs` (≥5 tests). Net: more coverage than before.
- Existing `app/` vitest suite (275 tests per phase 1c spec) must continue to pass — these changes are Rust-only resolver swap, no TS contract change.
- `Remedy` wire shape unchanged; `HealthPayload` unchanged.
- `cargo build --features dev -p senseid -p sensei-cli -p sensei-mcp` and `cargo build --release …` both green.
- Zero-errors-policy gate: `cargo clippy --all-targets --features dev -- -D warnings` and `cargo test` green before commit.

---

## 7. Out of scope

- **Version-drift warning.** A future checker enhancement could compare the installed postgres/ollama version against a minimum and surface a soft warning. Not in this change.
- **Uninstall remedies.** Resolvers install; they don't uninstall. Switching from dmg-ollama to brew-ollama remains a manual user choice via the `LinkConflict` remedy script.
- **Windows / Linux package managers.** The whole resolver currently targets macOS + Homebrew. Cross-platform expansion is a separate design.
- **Bootstrap-bin entry point.** The `sensei-bootstrap` crate exposes the resolvers; a separate `sensei bootstrap` CLI subcommand to run them manually is out of scope here (the daemon already drives them via the health flow).
