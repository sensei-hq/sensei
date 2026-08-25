//! `ManifestAdapter` trait + ecosystem impls.
//!
//! The daemon used to switch on manifest filenames inline in ten places
//! (`libraries.rs`, `scan_logic.rs`, `detector.rs`, `external_links.rs`,
//! `summary.rs`, etc.). This module holds one adapter per ecosystem and a
//! single dispatch function.
//!
//! Dispatch is static — `manifest_adapter_for_filename(filename)` returns a
//! `&'static dyn ManifestAdapter` when the filename is recognised, and each
//! impl is a zero-sized type so there's no allocation on the hot path.

use crate::indexer::lib_indexer::DepVersion;
use crate::types::PackageInfo;
use std::path::Path;

mod cargo;
mod composer;
mod dotnet;
mod go;
mod gradle;
mod maven;
mod npm;
mod pyproject;
mod ruby;
mod swiftpm;
pub(crate) mod workspace;
mod xml;

/// Adapter for a specific ecosystem's manifest format.
pub trait ManifestAdapter: Send + Sync {
    /// Fixed manifest filenames this adapter recognises (typically one).
    /// For ecosystems whose manifest names are extension-based (`.csproj`,
    /// `.sln`), leave this empty and populate [`Self::manifest_extensions`].
    fn manifest_filenames(&self) -> &[&'static str];

    /// Filename extensions (no dot) this adapter recognises when the
    /// ecosystem lacks fixed manifest names — used by .NET
    /// (`.csproj` / `.fsproj` / `.sln`). Default: none.
    fn manifest_extensions(&self) -> &[&'static str] {
        &[]
    }

    /// True when `filename` belongs to this adapter. Default impl accepts
    /// any name matching `manifest_filenames` or ending with a dot + any
    /// entry from `manifest_extensions`. Callers should prefer this over
    /// touching either list directly so extension-only adapters keep working.
    fn accepts(&self, filename: &str) -> bool {
        if self.manifest_filenames().contains(&filename) {
            return true;
        }
        let Some(dot) = filename.rfind('.') else { return false };
        let ext = &filename[dot + 1..];
        self.manifest_extensions().iter().any(|e| e.eq_ignore_ascii_case(ext))
    }

    /// Ecosystem slug matching the `sensei.library_ecosystem` DDL enum.
    /// One of `"npm"`, `"cargo"`, `"pypi"`, `"go"`, `"maven"`, `"nuget"`,
    /// `"rubygems"`, `"composer"`, `"swiftpm"`.
    fn ecosystem(&self) -> &'static str;

    /// Parse the raw contents of a manifest into dependency entries.
    ///
    /// Delegates the local-source protocol detection to shared helpers so
    /// npm `link:`/`workspace:`/`file:` and Cargo `path=` all set
    /// `DepVersion.local_source` the same way.
    fn parse_dependencies(&self, content: &str) -> Vec<DepVersion>;

    /// True if this manifest content declares a workspace root (npm
    /// `workspaces`, `Cargo.toml [workspace]`, `pnpm-workspace.yaml`, `go.work`).
    fn is_workspace_root(&self, content: &str) -> bool;

    /// Parse identity metadata (name, version, description) from a manifest.
    fn parse_manifest(&self, content: &str) -> ParsedManifest;

    /// Stack labels this manifest indicates (e.g. `["rust"]`, `["svelte"]`,
    /// `["typescript"]`, `["python"]`, `["go"]`). Framework detection (React,
    /// Svelte, Vue, Next) for npm happens here so the label vocabulary lives
    /// with the ecosystem that owns it. Default: the ecosystem slug.
    fn stack_labels(&self, _content: &str) -> Vec<&'static str> {
        vec![self.ecosystem()]
    }

    /// Infer a folder's semantic `folder_role` (`"library"` / `"tool"` /
    /// `"website"` / `"docs"`) from its manifest and filesystem signals.
    /// Returns `None` to leave the role unset. Default: `None` — ecosystems
    /// without a role-inference rule (Go, Python today) don't classify.
    fn infer_role(
        &self,
        _parsed: &ParsedManifest,
        _content: &str,
        _fs: &FsSignals,
    ) -> Option<&'static str> {
        None
    }

    /// Enumerate workspace-member packages of a repo root. Default: none —
    /// ecosystems that don't have a workspace concept (pyproject today) return
    /// an empty vector. Called by `config::detector::detect_workspace_members`.
    fn detect_workspace_members(&self, _repo_root: &Path) -> Vec<PackageInfo> {
        Vec::new()
    }

    /// Discoverable named commands the ecosystem exposes for this manifest
    /// (#83 T1 commands surface). Feeds `sensei.project_commands` so the
    /// project window's action buttons + the future `get_commands` MCP
    /// tool + AI-assistant "how do I run tests here?" queries all read from
    /// one authoritative source.
    ///
    /// Canonical `category` verbs (see [`command_category::categorise`]):
    /// `test` / `build` / `lint` / `run` / `format` / `typecheck` / `e2e`
    /// / `bench` / `docs` / `start` / `dev`. Ecosystems that don't expose
    /// named commands (Go, Maven at manifest level) return an empty vec —
    /// their conventional commands (`go test ./...`, `mvn test`) can be
    /// synthesised elsewhere and don't belong in the manifest-derived
    /// table.
    ///
    /// Default: empty. Override per-ecosystem to opt in.
    fn parse_commands(&self, _content: &str) -> Vec<DiscoveredCommand> {
        Vec::new()
    }
}

/// One command discovered from a manifest. Persisted into
/// `sensei.project_commands`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredCommand {
    /// Raw name as it appears in the manifest (e.g. `test:unit`, `build-fast`).
    pub raw_name: String,
    /// Full command line to invoke (`bun test:unit`, `npm run build-fast`).
    pub command_line: String,
    /// Canonical verb — one of the vocabulary in
    /// [`command_category::categorise`]. `None` for entries whose intent
    /// isn't discoverable from name alone (a caller-classifier can post-
    /// process, but the adapter shouldn't guess).
    pub category: Option<&'static str>,
}

/// Build a batch of [`DiscoveredCommand`]s from a conventional list —
/// `(subcommand, canonical_category)` pairs — for ecosystems whose commands
/// are not manifest-derived (Cargo, Go, Maven, Gradle, .NET, SwiftPM). The
/// `cli` is prepended verbatim to each subcommand to form `command_line`
/// (`cargo test`, `go test ./...`, `mvn test`). The `raw_name` is the
/// subcommand itself so uniqueness on (folder_id, raw_name) is stable across
/// scans.
pub fn conventional_commands(cli: &str, entries: &[(&str, &str)]) -> Vec<DiscoveredCommand> {
    entries
        .iter()
        .map(|(sub, verb)| DiscoveredCommand {
            raw_name: (*sub).to_string(),
            command_line: format!("{cli} {sub}"),
            category: Some(canonical_verb(verb)),
        })
        .collect()
}

/// Interns a category string to the `&'static` form the DDL check constraint
/// expects. Panics on unknown verbs — the input is always a compile-time
/// constant inside this crate, so a typo would be caught at test time.
fn canonical_verb(verb: &str) -> &'static str {
    match verb {
        "test" => "test",
        "build" => "build",
        "lint" => "lint",
        "run" => "run",
        "format" => "format",
        "typecheck" => "typecheck",
        "e2e" => "e2e",
        "bench" => "bench",
        "docs" => "docs",
        "start" => "start",
        "dev" => "dev",
        other => panic!(
            "unknown canonical verb: {other} — extend canonical_verb / command_category vocabulary"
        ),
    }
}

/// Shared classifier that maps a script name → canonical category verb.
/// Keeps the same vocabulary across ecosystems so the project window's
/// action buttons and the get_commands MCP tool speak one language.
pub mod command_category {
    /// Return the canonical verb for a script name, or `None` when nothing
    /// obvious matches. Uses lowercase substring matching, category-agnostic
    /// on the ecosystem (an npm `test:e2e` and a cargo `xtask-e2e` alias
    /// both categorise as `e2e`).
    ///
    /// Longer / more-specific matches win first — `e2e` beats `test`,
    /// `typecheck` beats `check`, so the frontend variant selector always
    /// picks the tighter category.
    pub fn categorise(name: &str) -> Option<&'static str> {
        let n = name.to_ascii_lowercase();
        // Order matters: more-specific patterns first.
        if n.contains("e2e") || n.contains("end-to-end") {
            return Some("e2e");
        }
        if n.contains("typecheck") || n.contains("type-check") || n == "tsc" {
            return Some("typecheck");
        }
        if n.contains("bench") {
            return Some("bench");
        }
        if n.contains("format") || n.contains("prettier") || n.contains("rustfmt") {
            return Some("format");
        }
        if n.contains("lint") || n.contains("clippy") || n.contains("eslint") || n.contains("ruff")
        {
            return Some("lint");
        }
        if n.contains("docs") || n == "doc" || n.contains("docgen") {
            return Some("docs");
        }
        if n.contains("test") {
            return Some("test");
        }
        if n.contains("build") || n == "compile" {
            return Some("build");
        }
        if n == "dev" || n.contains("watch") || n.starts_with("dev:") {
            return Some("dev");
        }
        if n == "start" || n.starts_with("start:") {
            return Some("start");
        }
        if n == "run" || n.starts_with("run:") {
            return Some("run");
        }
        None
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn e2e_wins_over_test() {
            // A script named `test:e2e` categorises as e2e, not test —
            // longer/more-specific match first.
            assert_eq!(categorise("test:e2e"), Some("e2e"));
            assert_eq!(categorise("end-to-end"), Some("e2e"));
        }

        #[test]
        fn typecheck_wins_over_lint() {
            assert_eq!(categorise("typecheck"), Some("typecheck"));
            assert_eq!(categorise("type-check"), Some("typecheck"));
            assert_eq!(categorise("tsc"), Some("typecheck"));
        }

        #[test]
        fn lint_variants() {
            assert_eq!(categorise("lint"), Some("lint"));
            assert_eq!(categorise("clippy"), Some("lint"));
            assert_eq!(categorise("eslint:fix"), Some("lint"));
            assert_eq!(categorise("ruff"), Some("lint"));
        }

        #[test]
        fn build_variants() {
            assert_eq!(categorise("build"), Some("build"));
            assert_eq!(categorise("build:release"), Some("build"));
            assert_eq!(categorise("compile"), Some("build"));
        }

        #[test]
        fn dev_start_run() {
            assert_eq!(categorise("dev"), Some("dev"));
            assert_eq!(categorise("dev:app"), Some("dev"));
            assert_eq!(categorise("start"), Some("start"));
            assert_eq!(categorise("watch"), Some("dev"));
            assert_eq!(categorise("run"), Some("run"));
        }

        #[test]
        fn unknown_returns_none() {
            assert_eq!(categorise("myScript"), None);
            assert_eq!(categorise("release"), None);
        }
    }
}

/// Is a discovered command the user's **preferred** tool for its category
/// (`sensei.dojo_preferences`, G10)? Matches the preference token against the
/// command's `raw_name` (exact) or `command_line` (substring), case-insensitively
/// — so a preference of `nextest` marks `cargo nextest` (raw_name) but also a
/// script whose command line merely invokes it. A `None` category or no
/// preference for it → `false`. Pure, so the bias is unit-testable without a DB.
pub fn command_matches_preference(
    category: Option<&str>,
    raw_name: &str,
    command_line: &str,
    prefs: &std::collections::HashMap<String, String>,
) -> bool {
    let Some(cat) = category else { return false };
    let Some(pref) = prefs.get(cat) else { return false };
    let p = pref.trim().to_lowercase();
    !p.is_empty() && (raw_name.to_lowercase() == p || command_line.to_lowercase().contains(&p))
}

#[cfg(test)]
mod preference_tests {
    use super::command_matches_preference;
    use std::collections::HashMap;

    #[test]
    fn matches_by_raw_name_or_command_line_case_insensitively() {
        let prefs: HashMap<String, String> =
            [("test".to_string(), "nextest".to_string())].into_iter().collect();
        // exact raw_name (case-insensitive) OR substring of the command line
        assert!(command_matches_preference(Some("test"), "nextest", "cargo nextest run", &prefs));
        assert!(command_matches_preference(Some("test"), "t", "cargo NEXTEST run", &prefs));
        // a near-but-different name that isn't the tool → not preferred
        assert!(!command_matches_preference(Some("test"), "NextTest", "", &prefs));
        // a different test command (`cargo test`) → not preferred
        assert!(!command_matches_preference(Some("test"), "test", "cargo test", &prefs));
        // no preference for this category, or no category → not preferred
        assert!(!command_matches_preference(Some("build"), "nextest", "cargo nextest", &prefs));
        assert!(!command_matches_preference(None, "nextest", "cargo nextest", &prefs));
    }
}

/// Identity metadata for a manifest.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParsedManifest {
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
}

/// Filesystem-side signals the caller supplies to `infer_role`. Kept in one
/// struct so the trait method stays stable when new signals are added.
#[derive(Debug, Clone, Default)]
pub struct FsSignals {
    /// `src/lib.rs` exists (Rust library entry point).
    pub has_lib_rs: bool,
    /// `src/routes/` exists (SvelteKit / web-app route tree).
    pub has_src_routes: bool,
    /// Folder's own directory name — reserved for future adapters that
    /// classify by folder name (e.g. `docs/` → docs). Today the docs fallback
    /// lives at the `classify_role` layer since it isn't tied to any manifest.
    #[allow(dead_code)]
    pub dir_name: String,
}

/// Pick the adapter for a manifest filename.
///
/// Returns `None` for unknown filenames — callers fall back to the legacy
/// path until every ecosystem is migrated.
pub fn manifest_adapter_for_filename(filename: &str) -> Option<&'static dyn ManifestAdapter> {
    for a in registered_adapters() {
        if a.accepts(filename) {
            return Some(*a);
        }
    }
    None
}

/// All registered ManifestAdapter impls. Add new ecosystems here.
pub fn registered_adapters() -> &'static [&'static dyn ManifestAdapter] {
    &[
        &npm::NpmManifestAdapter,
        &cargo::CargoManifestAdapter,
        &pyproject::PyprojectManifestAdapter,
        &go::GoManifestAdapter,
        &maven::MavenManifestAdapter,
        &dotnet::DotnetManifestAdapter,
        &gradle::GradleManifestAdapter,
        &ruby::RubyManifestAdapter,
        &composer::ComposerManifestAdapter,
        &swiftpm::SwiftPmManifestAdapter,
    ]
}

/// Every distinct manifest filename any registered adapter recognises. Used by
/// filesystem walks (subproject discovery, manifest scanning) that need to
/// know "is there a known manifest in this directory?" without knowing which
/// ecosystem it belongs to.
pub fn all_manifest_filenames() -> Vec<&'static str> {
    let mut out: Vec<&'static str> =
        registered_adapters().iter().flat_map(|a| a.manifest_filenames().iter().copied()).collect();
    out.sort();
    out.dedup();
    out
}

/// Every distinct manifest extension any registered adapter recognises. Used
/// by filesystem walks to catch ecosystems whose manifests are extension-keyed
/// (`.csproj` / `.fsproj` / `.sln`) rather than fixed-name.
pub fn all_manifest_extensions() -> Vec<&'static str> {
    let mut out: Vec<&'static str> = registered_adapters()
        .iter()
        .flat_map(|a| a.manifest_extensions().iter().copied())
        .collect();
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_returns_npm_for_package_json() {
        let a = manifest_adapter_for_filename("package.json").unwrap();
        assert_eq!(a.ecosystem(), "npm");
    }

    #[test]
    fn dispatch_returns_none_for_unknown_filename() {
        assert!(manifest_adapter_for_filename("BUILD.bazel").is_none());
        assert!(manifest_adapter_for_filename("").is_none());
    }

    #[test]
    fn dispatch_returns_cargo_for_cargo_toml() {
        let a = manifest_adapter_for_filename("Cargo.toml").unwrap();
        assert_eq!(a.ecosystem(), "cargo");
    }

    #[test]
    fn dispatch_returns_pyproject_for_pyproject_toml() {
        let a = manifest_adapter_for_filename("pyproject.toml").unwrap();
        assert_eq!(a.ecosystem(), "pypi");
    }

    #[test]
    fn dispatch_returns_go_for_go_mod() {
        let a = manifest_adapter_for_filename("go.mod").unwrap();
        assert_eq!(a.ecosystem(), "go");
    }

    #[test]
    fn dispatch_returns_maven_for_pom_xml() {
        let a = manifest_adapter_for_filename("pom.xml").unwrap();
        assert_eq!(a.ecosystem(), "maven");
    }

    #[test]
    fn dispatch_returns_dotnet_for_csproj_by_extension() {
        let a = manifest_adapter_for_filename("MyApp.csproj").unwrap();
        assert_eq!(a.ecosystem(), "nuget");
    }

    #[test]
    fn dispatch_returns_dotnet_for_sln_by_extension() {
        let a = manifest_adapter_for_filename("solution.sln").unwrap();
        assert_eq!(a.ecosystem(), "nuget");
    }

    #[test]
    fn dispatch_returns_gradle_for_build_gradle() {
        let a = manifest_adapter_for_filename("build.gradle").unwrap();
        assert_eq!(a.ecosystem(), "maven");
        let b = manifest_adapter_for_filename("build.gradle.kts").unwrap();
        assert_eq!(b.ecosystem(), "maven");
    }

    #[test]
    fn dispatch_returns_ruby_for_gemfile() {
        let a = manifest_adapter_for_filename("Gemfile").unwrap();
        assert_eq!(a.ecosystem(), "rubygems");
    }

    #[test]
    fn dispatch_returns_composer_for_composer_json() {
        let a = manifest_adapter_for_filename("composer.json").unwrap();
        assert_eq!(a.ecosystem(), "composer");
    }

    #[test]
    fn dispatch_returns_swiftpm_for_package_swift() {
        let a = manifest_adapter_for_filename("Package.swift").unwrap();
        assert_eq!(a.ecosystem(), "swiftpm");
    }

    #[test]
    fn all_manifest_filenames_includes_every_ecosystem() {
        let names = all_manifest_filenames();
        for expected in ["Cargo.toml", "package.json", "pyproject.toml", "go.mod", "pom.xml"] {
            assert!(names.contains(&expected), "missing {expected} in {names:?}");
        }
    }

    // ── conventional_commands helper ─────────────────────────────────

    #[test]
    fn conventional_commands_prefixes_cli_and_categorises() {
        let cmds = conventional_commands("cargo", &[("test", "test"), ("build", "build")]);
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].raw_name, "test");
        assert_eq!(cmds[0].command_line, "cargo test");
        assert_eq!(cmds[0].category, Some("test"));
        assert_eq!(cmds[1].command_line, "cargo build");
    }

    #[test]
    #[should_panic(expected = "unknown canonical verb")]
    fn canonical_verb_panics_on_typo_so_tests_catch_it() {
        // Guard against a copy-paste typo like "tests" instead of "test" —
        // this panic surfaces during test time, never in production.
        conventional_commands("x", &[("y", "testz")]);
    }

    #[test]
    fn every_registered_adapter_produces_at_least_one_command_or_none_intentionally() {
        // Sanity: for the ecosystems that emit conventional commands we
        // expect at least a `test` entry, since "how do I test this?" is
        // the load-bearing question. npm/composer/pyproject are content-
        // driven so an empty manifest is legitimately empty.
        for adapter in registered_adapters() {
            let cmds = adapter.parse_commands("");
            let has_test = cmds.iter().any(|c| c.category == Some("test"));
            match adapter.ecosystem() {
                "cargo" | "go" | "maven" | "nuget" | "swiftpm" | "rubygems" => {
                    assert!(
                        has_test,
                        "{} adapter must emit a `test` conventional command on empty content",
                        adapter.ecosystem()
                    );
                }
                _ => { /* content-driven adapters (npm/composer/pyproject) are fine with empty */ }
            }
        }
    }

    #[test]
    fn all_manifest_extensions_includes_dotnet_variants() {
        let exts = all_manifest_extensions();
        for expected in ["csproj", "fsproj", "sln"] {
            assert!(exts.contains(&expected), "missing {expected} in {exts:?}");
        }
    }
}
