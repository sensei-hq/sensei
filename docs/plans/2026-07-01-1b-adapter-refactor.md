---
name: 2026-07-01 — 1b Adapter refactor
issue: https://github.com/sensei-hq/sensei/issues/93
epic: https://github.com/sensei-hq/sensei/issues/83
analysis: docs/analysis/2026-07-01-project-window-instruments-depmap-gap-analysis.md
---

# 1b — Adapter refactor

Consolidate ten inline dispatch sites into three sibling adapter traits (plus a small `LanguageAdapter` extension) so ecosystem / manifest / file-classification logic lives in one place per concern.

**No behaviour changes** except for the one new capability: **go.mod parsing** (Go dep versions are extracted for the first time). Everything else is refactor + tests.

## Traits

```rust
// existing (extend)
trait LanguageAdapter {
    fn language(&self) -> &str;
    fn display_name(&self) -> &str;       // NEW — "Rust" for label surfaces
    fn parse(...) -> ParsedFile;
    fn parse_to_ir(...) -> IRParsedFile;
}

// new
trait ManifestAdapter: Send + Sync {
    fn manifest_filenames(&self) -> &[&'static str];
    fn ecosystem(&self) -> &'static str;
    fn is_workspace_root(&self, content: &str) -> bool;
    fn parse_manifest(&self, content: &str) -> ParsedManifest;
    fn detect_workspace_members(&self, repo_root: &Path) -> Vec<PackageInfo>;
    fn parse_dependencies(&self, content: &str) -> Vec<DepVersion>;
    fn infer_role(&self, manifest: &ParsedManifest, fs: FsSignals) -> Option<&'static str>;
}

trait ConfigAdapter: Send + Sync {
    fn extensions(&self) -> &[&'static str];
    fn can_process(&self, filename: &str, ext: &str) -> bool;
    fn extract_metadata_links(&self, content: &str) -> Vec<ExternalLink>;
    fn extract_description(&self, content: &str) -> Option<String>;
    fn extract_version(&self, content: &str) -> Option<String>;
}

trait FileClassifier: Send + Sync {
    fn is_binary(&self, ext: &str) -> bool;
    fn is_source_file(&self, ext: &str) -> bool;
}
```

`ParsedManifest`:
```rust
struct ParsedManifest {
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
    links: Vec<ExternalLink>,       // homepage, repository, bugs, docs
    workspace_root: bool,
    private: bool,
}
```

Impls all live under `crates/senseid/src/adapters/` (new module). Dispatch via free functions `manifest_adapter_for_filename(&str)`, `config_adapter_for_ext(&str)`, `file_classifier()` (singleton).

## Order of work

Each step: own commit, TDD, clippy clean.

### Step 1 — `LanguageAdapter.display_name()`
- Add trait method with a default that Title-Cases the `language()` output.
- Override where the label diverges (`typescript` → "TypeScript" already Title-cases correctly; `svelte` → "Svelte" fine; `rust` → "Rust"; `sql` → "SQL" needs override; `html` → "HTML" needs override; `css` → "CSS" needs override; `md` → "Markdown"; `jsx` / `tsx` variants → "React (JS)" / "React (TS)" — keep simple, override only where obvious).
- Migrate `api/handlers/codebase.rs:337-360` — replace the `match ext { "rs" => "rust", … }` block with `adapter_for_ext(ext).map(|a| a.display_name())`. Fallback to lowercase of ext.
- Unit tests for the override cases + the migration site.

### Step 2 — `FileClassifier` trait + default impl
- New module `crates/senseid/src/classifiers.rs`.
- Trait + `DefaultFileClassifier` (holds the ext lists — currently in `helpers.rs:6-25` for binary and `scan_logic.rs:216-234` for source).
- Migrate `helpers.rs::is_binary_ext` to call `file_classifier().is_binary(ext)`.
- Migrate `scan_logic.rs::is_project_source_ext` to call `file_classifier().is_source_file(ext)`.
- Unit tests for each list plus the migration call sites.

### Step 3 — `ManifestAdapter` trait skeleton + `NpmManifestAdapter`
- New module `crates/senseid/src/adapters/manifest/mod.rs`.
- Trait, `ParsedManifest`, `ExternalLink`, `FsSignals`.
- `NpmManifestAdapter` impl covers: parse_dependencies (moves + extends `lib_indexer::parse_npm_deps`), detect_workspace_members (npm + pnpm workspaces from `config/detector.rs`), parse_manifest (name / version / description / repository / homepage / bugs from `external_links.rs` + `summary.rs`), is_workspace_root, infer_role stub.
- Free-function dispatch `manifest_adapter_for_filename(&str) -> Option<Arc<dyn ManifestAdapter>>` (or `Box<dyn>` if performance is fine).
- Migrate `libraries.rs:271-276` ecosystem match to `manifest_adapter_for_filename(source).map(|a| a.ecosystem())`.
- Unit tests: parse a real rokkit-shaped package.json fixture; workspace root detection; metadata parsing.

### Step 4 — `CargoManifestAdapter`
- Same shape as npm. Moves `lib_indexer::parse_cargo_deps` + Cargo workspace detection from `config/detector.rs` + Cargo description from `summary.rs`.
- Unit tests: sensei-shaped Cargo.toml fixture with `[workspace]` + workspace members.

### Step 5 — `PyprojectManifestAdapter`
- Same shape. Moves `lib_indexer::parse_pyproject_deps`. Adds `[tool.poetry.dependencies]` and `[tool.uv.dependency-groups]` variants (poetry/uv fallback).
- Unit tests: PEP 621 + poetry + uv fixtures.

### Step 6 — `GoManifestAdapter` (NEW capability)
- Parse `go.mod` (INI-like — `require (...)` blocks and single-line requires).
- Parse `go.work` for workspace members (moves from `config/detector.rs:141-165`).
- Ecosystem: `"go"`.
- Unit tests: go.mod with block + single-line requires, go.work with `use ./sub`.

### Step 7 — Wire adapters into `lib_indexer::extract_dep_versions`
- Replace the three inline `if let Ok(content) = ...` blocks with a dispatch over `manifest_adapter_for_filename`.
- Keep the go.mod branch that Step 6 unlocks.
- Verify no regression via full test suite.

### Step 8 — Migrate `scan_logic.rs:270-337` (workspace + stack detection)
- `is_monorepo()` → check `is_workspace_root()` across registered adapters.
- `detect_stack()` → `manifest_adapter_for_filename(...).map(|a| a.ecosystem())` plus filesystem signals (already there).
- `infer_role()` → `adapter.infer_role(parsed, fs_signals)`.
- `find_subprojects_walk()` → adapter-aware child manifest lookup.
- Unit tests for each function against fixtures.

### Step 9 — Migrate `config/detector.rs`
- The whole `detect_workspace_members` function becomes a fold over `manifest_adapter_for_filename(...)` calls.
- Unit tests already exist for the three current branches; keep them, expand for go.work.

### Step 10 — `ConfigAdapter` trait + JSON / TOML / YAML impls
- New module `crates/senseid/src/adapters/config/mod.rs`.
- `JsonConfigAdapter`, `TomlConfigAdapter`, `YamlConfigAdapter`.
- Metadata extraction (links, description, version) currently in `external_links.rs` + `summary.rs` moves here.
- Migrate `router.rs:40-60` config branch to route via `config_adapter_for_ext`.
- Migrate `external_links.rs::extract_package_json_links` + `extract_toml_links` — these become `JsonConfigAdapter.extract_metadata_links` + `TomlConfigAdapter.extract_metadata_links`.
- Migrate `summary.rs::extract_summary` — the manifest branches call `ManifestAdapter.parse_manifest().description`; other config branches call `ConfigAdapter.extract_description`.
- Unit tests for each adapter + migration call sites.

### Step 11 — Cleanup + non-regression verification
- Remove any dead `is_binary_ext` / `is_project_source_ext` / inline `match ext` code left over.
- `cargo test --workspace`.
- `SENSEI_DDL_DIR=... make install-debug` + reindex `~/Developer`; capture library + project counts.
- Compare to a pre-refactor baseline (captured before Step 1).
- Merge sub-chunk to `develop`, then `develop → main`.

## Test coverage summary

- **Unit:** every adapter impl has fixtures covering its ecosystem's common cases (single string dep, table dep with path, workspaces, description, links).
- **Non-regression:** any test whose behaviour depended on the inline dispatch continues to pass without modification.
- **Integration (DB):** existing tests for `extract_deps` + workspace-member registration pass unchanged.

## Sequencing / exit criteria

Merge sub-chunk to `main` when:
- All 26+ new unit tests green + all existing tests green.
- Clippy + check zero-warning across the workspace.
- Baseline library/project counts unchanged post-rescan (except go.mod which adds coverage).

## Filed follow-ups

Deferred ecosystem adapters (Maven, Gradle, .NET, Ruby / PHP / Swift / Elixir / Dart / Haskell / OCaml) already have issues #86–#89. Each becomes a new sibling ManifestAdapter impl once T1b's trait shape is stable.
