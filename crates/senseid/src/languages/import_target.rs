//! What an import target IS — the one classifier for import edges.
//!
//! ## Why this exists
//!
//! MEASURED 2026-09-01 on 136,484 `imports` edges: **0% resolved**. Not because
//! resolution failed, but because nothing ever tried — `process.rs` inserts every
//! import edge with `target_id = None`, from a field named `unresolved_imports`.
//!
//! Twelve lines below that insert, the comment on CALL edges explains why calls
//! reach 64.8%: their target is get-or-created by FQN, becoming a `lib_symbol`
//! node when it is external. **Calls resolve because they create the target.
//! Imports never do.**
//!
//! So the majority of those 136,484 rows are not failures at all. Classified:
//!
//! ```text
//! bare             62,600   mostly npm/crates packages   EXTERNAL
//! java-stdlib      29,237                                EXTERNAL
//! relative         15,924   ./x  ../y                    LOCAL
//! node-builtin     11,989   node:fs                      EXTERNAL
//! ts-alias          7,032   @/…  ~/…                     LOCAL (needs tsconfig)
//! scoped-npm        6,675   @scope/pkg                   EXTERNAL
//! sveltekit-alias   1,802   $lib/…                       LOCAL
//! rust-internal     1,225   crate:: super:: self::       LOCAL
//! ```
//!
//! Only ~19% could ever point at a local node. Reporting the other 81% as
//! "unresolved" is the same misattribution `sensei.metric_status` had before #128,
//! where a group that keeps no cursor was read as one that never ran.
//!
//! ## What this can and cannot decide
//!
//! It reads the target STRING, so it is authoritative only where the language
//! marks locality syntactically. MEASURED per source language:
//!
//! ```text
//! language     edges    local imports syntactically distinguishable?
//! java        74,325    NO   — own classes look like org.junit.Test
//! typescript  45,638    yes  — ./  ../  $lib  @/  ~/
//! kotlin       3,713    NO   — same shape as Java
//! rust         3,711    yes  — crate::  super::  self::
//! javascript   3,535    yes
//! svelte       3,100    yes
//! python       1,924    partly — `.mod` yes; `app.models.x` NO
//! c              537    NO   — the "local.h" vs <system.h> distinction is
//!                              dropped at extraction, so `pljava/Type.h`
//!                              (a project header) is indistinguishable from
//!                              `postgres.h` (a system one)
//! ```
//!
//! So for **59%** of edges (Java, Kotlin, Python-absolute, C) a local import is
//! NOT distinguishable from an external one by string. This classifier calls them
//! `External`, which is right for most of them and wrong for a project's own
//! packages — and it cannot tell which without knowing the repository's owned
//! package roots.
//!
//! **That gap does not have to be closed here.** The resolver this feeds must try a
//! LOCAL LOOKUP FIRST and fall back to a `lib_symbol` only on a miss — data, not
//! string. Under that rule a misclassified Java import still resolves correctly,
//! because the lookup finds the node. The classification stays useful for
//! REPORTING (what shape are these imports) and as a hint, and must not be treated
//! as the authority on locality.
//!
//! ## Resolution DROPS the name — which the resolver must account for
//!
//! MEASURED across all 715,985 edges: `target_id` and `target_name` are mutually
//! exclusive. A resolved edge carries an id and a NULL name (207,868 calls, 601
//! covers); an unresolved one carries a name and a NULL id. Zero edges carry both,
//! and zero carry neither.
//!
//! So resolving an import ERASES the original target string. The node it resolves
//! to must therefore carry that string — which is what a `lib_symbol` keyed on the
//! package does, and why the external branch produces a package name rather than
//! discarding it.
//!
//! ## One owner, two consumers
//!
//! This is a pure function of the target string — `node:fs` is external whenever
//! you ask — so it is derived, never stored, and cannot go stale. It serves both
//! the honest reporting of what an import edge is, and (next) the resolver's
//! decision: an EXTERNAL target becomes a `lib_symbol` node exactly as an external
//! call target does; a LOCAL one resolves to a file or module node.
//!
//! Deliberately not duplicated into SQL. Two copies of a resolution rule is how
//! the scan exclusion resolver came to gate the watcher while pruning nothing.

/// Where an import points.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportTarget {
    /// Outside the indexed codebase — a package, a language stdlib, a runtime
    /// builtin. `package` is the installable/importable unit, which is what a
    /// `lib_symbol` node should be keyed on: `node:fs` → `node:fs`,
    /// `java.util.List` → `java.util`, `@scope/pkg/sub` → `@scope/pkg`,
    /// `lodash/debounce` → `lodash`.
    ///
    /// NOT a failure. An agent asking "what does this file depend on" is answered
    /// by exactly this.
    External { package: String },
    /// A path relative to the importing file — `./x`, `../y`. Resolvable by
    /// dirname arithmetic; measured 70.3% hit rate on a 3,000-edge sample.
    Relative,
    /// A build-tool alias that maps to a local directory. Resolvable only with
    /// that tool's config (tsconfig `paths`, SvelteKit's `$lib`), which is why it
    /// is a distinct case and not simply `Relative`.
    Alias { kind: AliasKind },
    /// Internal to the crate/module tree — Rust `crate::` / `super::` / `self::`.
    /// Local, and resolvable without any external config.
    Internal,
}

/// Which build tool owns the alias, because resolving one needs that tool's config.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AliasKind {
    /// SvelteKit's `$lib`, `$app`, `$env` — `$lib` maps to `src/lib` by default.
    SvelteKit,
    /// A tsconfig/jsconfig `paths` entry — `@/…`, `~/…`. The mapping is arbitrary
    /// and must be read from config; there is no default.
    TsPaths,
}

impl ImportTarget {
    /// True when the target lies outside the indexed codebase.
    ///
    /// The distinction the 81% needs: an external import is a complete, useful
    /// fact, not a resolution that failed.
    pub fn is_external(&self) -> bool {
        matches!(self, Self::External { .. })
    }

    /// A stable label for reporting — the vocabulary a caller renders.
    pub fn label(&self) -> &'static str {
        match self {
            Self::External { .. } => "external",
            Self::Relative => "relative",
            Self::Alias { kind: AliasKind::SvelteKit } => "sveltekit-alias",
            Self::Alias { kind: AliasKind::TsPaths } => "ts-alias",
            Self::Internal => "internal",
        }
    }
}

/// The package an external target belongs to — the unit a `lib_symbol` is keyed on.
///
/// Kept narrow on purpose. A dotted Java/Kotlin path collapses to its first two
/// segments (`java.util.List` → `java.util`), which is the granularity a reader
/// recognises as "a library"; going deeper would make every imported class its own
/// package, and stopping at one segment would put all of `java.*` in one bucket.
fn external_package(target: &str) -> String {
    // Runtime builtins keep their scheme — `node:fs` IS the package name.
    if let Some(rest) = target.strip_prefix("node:") {
        return format!("node:{}", rest.split('/').next().unwrap_or(rest));
    }
    // Scoped npm: `@scope/pkg/sub` → `@scope/pkg`.
    if target.starts_with('@') {
        let mut it = target.splitn(3, '/');
        return match (it.next(), it.next()) {
            (Some(scope), Some(pkg)) => format!("{scope}/{pkg}"),
            _ => target.to_string(),
        };
    }
    // Dotted (Java/Kotlin/Python): first two segments.
    if target.contains('.') && !target.contains('/') {
        let parts: Vec<&str> = target.split('.').collect();
        if parts.len() >= 2 {
            return format!("{}.{}", parts[0], parts[1]);
        }
    }
    // Plain npm/crates: `lodash/debounce` → `lodash`.
    target.split('/').next().unwrap_or(target).to_string()
}

/// Classify one import target.
///
/// Order matters: the LOCAL forms are recognised first, because several of them
/// would otherwise be swallowed by a broader external rule (`$lib/x` contains a
/// slash, `crate::x` contains no dot).
///
/// An empty or whitespace-only target is `External` with an empty package rather
/// than a separate variant — it cannot point anywhere local, and inventing an
/// "unknown" case would give callers a branch with nothing useful to do in it.
pub fn classify_import(target: &str) -> ImportTarget {
    let t = target.trim();

    if t.starts_with("./") || t.starts_with("../") || t == "." || t == ".." {
        return ImportTarget::Relative;
    }
    // Python explicit-relative: `.module`, `..package.module`. A leading dot with
    // no slash. MEASURED: 85 such edges, which the dotted-external rule below
    // would have called a package named `.module`.
    if t.starts_with('.') && !t.contains('/') {
        return ImportTarget::Relative;
    }
    if t.starts_with("$lib") || t.starts_with("$app") || t.starts_with("$env") {
        return ImportTarget::Alias { kind: AliasKind::SvelteKit };
    }
    if t.starts_with("@/") || t.starts_with("~/") {
        return ImportTarget::Alias { kind: AliasKind::TsPaths };
    }
    if t.starts_with("crate::") || t.starts_with("super::") || t.starts_with("self::") {
        return ImportTarget::Internal;
    }

    ImportTarget::External { package: external_package(t) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_paths_are_local() {
        assert_eq!(classify_import("./utils"), ImportTarget::Relative);
        assert_eq!(classify_import("../lib/x.ts"), ImportTarget::Relative);
    }

    /// `$lib/x` and `@/x` contain a slash, and a naive "has a slash → npm subpath"
    /// rule would call both external. They are the second- and third-largest LOCAL
    /// classes (7,032 + 1,802 edges), so getting this wrong would misclassify 8,834
    /// resolvable imports as dependencies.
    #[test]
    fn build_tool_aliases_are_local_not_npm_subpaths() {
        assert_eq!(
            classify_import("$lib/components/List.svelte"),
            ImportTarget::Alias { kind: AliasKind::SvelteKit }
        );
        assert_eq!(
            classify_import("$app/navigation"),
            ImportTarget::Alias { kind: AliasKind::SvelteKit }
        );
        assert_eq!(classify_import("@/lib/x"), ImportTarget::Alias { kind: AliasKind::TsPaths });
        assert_eq!(
            classify_import("~/stores/user"),
            ImportTarget::Alias { kind: AliasKind::TsPaths }
        );
    }

    /// `@/x` (tsconfig alias) and `@scope/pkg` (npm) differ by ONE character after
    /// the `@`. Confusing them sends a local import to a `lib_symbol` node and an
    /// npm package to a file lookup that cannot succeed.
    #[test]
    fn a_scoped_package_is_not_a_tsconfig_alias() {
        assert_eq!(
            classify_import("@rokkit/ui"),
            ImportTarget::External { package: "@rokkit/ui".into() }
        );
        assert_eq!(
            classify_import("@rokkit/ui/List.svelte"),
            ImportTarget::External { package: "@rokkit/ui".into() },
            "a subpath still belongs to its package",
        );
    }

    /// Python's explicit-relative form is a leading dot with no slash. The dotted
    /// EXTERNAL rule would otherwise call `.module` a package named `.module`.
    /// MEASURED: 85 such edges live.
    #[test]
    fn python_explicit_relative_is_local() {
        assert_eq!(classify_import(".module"), ImportTarget::Relative);
        assert_eq!(classify_import("..package.module"), ImportTarget::Relative);
    }

    /// The limit of a string-only classifier, pinned so nobody mistakes it for
    /// authority. A Java project's own class and a third-party one are the same
    /// shape, and C loses the quoted-vs-angled distinction at extraction — 59% of
    /// import edges are in languages where this holds.
    ///
    /// Both are called External here, which is right for most and wrong for a
    /// project's own packages. The resolver must try a local lookup FIRST and fall
    /// back to `lib_symbol` on a miss, so a misclassification here does not become
    /// a wrong edge.
    #[test]
    fn java_and_c_local_imports_are_not_string_distinguishable() {
        // Indistinguishable from `org.junit.Test` without knowing the repo's roots.
        assert!(classify_import("org.postgresql.pljava.Function").is_external());
        // A real project header from the measured data, indistinguishable from a
        // system include once the angle brackets are gone.
        assert!(classify_import("pljava/type/Type_priv.h").is_external());
        assert!(classify_import("postgres.h").is_external());
    }

    #[test]
    fn rust_internal_paths_are_local() {
        assert_eq!(classify_import("crate::db::pg_store"), ImportTarget::Internal);
        assert_eq!(classify_import("super::common"), ImportTarget::Internal);
        assert_eq!(classify_import("self::helpers"), ImportTarget::Internal);
    }

    #[test]
    fn runtime_builtins_keep_their_scheme_as_the_package() {
        // `node:fs` IS the package — stripping the scheme would collide with a
        // userland package named `fs`, which exists on npm.
        assert_eq!(
            classify_import("node:fs"),
            ImportTarget::External { package: "node:fs".into() }
        );
        assert_eq!(
            classify_import("node:assert/strict"),
            ImportTarget::External { package: "node:assert".into() }
        );
    }

    #[test]
    fn dotted_stdlib_collapses_to_two_segments() {
        // 29,237 java-stdlib edges. One segment would put all of `java.*` in one
        // bucket; the full path would make every imported class its own package.
        assert_eq!(
            classify_import("java.util.List"),
            ImportTarget::External { package: "java.util".into() }
        );
        assert_eq!(
            classify_import("javax.xml.bind.annotation.XmlType"),
            ImportTarget::External { package: "javax.xml".into() }
        );
        assert_eq!(
            classify_import("lombok.Getter"),
            ImportTarget::External { package: "lombok.Getter".into() },
            "a two-segment target is already its own package",
        );
    }

    #[test]
    fn a_bare_package_subpath_belongs_to_its_package() {
        assert_eq!(
            classify_import("lodash/debounce"),
            ImportTarget::External { package: "lodash".into() }
        );
        assert_eq!(classify_import("react"), ImportTarget::External { package: "react".into() });
    }

    #[test]
    fn is_external_separates_the_81_percent_from_a_failure() {
        assert!(classify_import("node:fs").is_external());
        assert!(classify_import("java.util.List").is_external());
        assert!(!classify_import("./x").is_external());
        assert!(!classify_import("$lib/x").is_external());
        assert!(!classify_import("crate::x").is_external());
    }

    #[test]
    fn labels_match_the_measured_vocabulary() {
        assert_eq!(classify_import("./x").label(), "relative");
        assert_eq!(classify_import("$lib/x").label(), "sveltekit-alias");
        assert_eq!(classify_import("@/x").label(), "ts-alias");
        assert_eq!(classify_import("crate::x").label(), "internal");
        assert_eq!(classify_import("react").label(), "external");
    }

    #[test]
    fn whitespace_is_trimmed_and_empty_does_not_panic() {
        assert_eq!(classify_import("  ./x  "), ImportTarget::Relative);
        assert_eq!(classify_import(""), ImportTarget::External { package: String::new() });
    }
}
