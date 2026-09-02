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

/// Resolve a relative import against the current module path (both `/`-joined,
/// extension-free). `lib/util` + `./helper` → `lib/helper`; `+ ../x/y` → `x/y`.
///
/// Hoisted here from `typescript_fqn` because it is specifier arithmetic, which
/// this module owns. The TS classifier now calls it rather than keeping a second
/// copy — two copies of a resolution rule is the incident named at the top of
/// this file.
pub fn resolve_relative(current_module: &str, spec: &str) -> String {
    let mut parts: Vec<&str> = current_module.split('/').filter(|s| !s.is_empty()).collect();
    parts.pop(); // drop the current file, leaving its directory
    for seg in spec.split('/') {
        match seg {
            "." | "" => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    let joined = parts.join("/");
    // Drop a trailing file extension on the final segment.
    match joined.rsplit_once('/') {
        Some((head, tail)) => format!("{head}/{}", strip_ext(tail)),
        None => strip_ext(&joined).to_string(),
    }
}

/// Drop a JS-family module extension. `./foo.ts` and `./foo` name one module.
pub fn strip_ext(s: &str) -> &str {
    for ext in [".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".svelte", ".vue"] {
        if let Some(b) = s.strip_suffix(ext) {
            return b;
        }
    }
    s
}

/// The module paths a LOCAL specifier could name, in priority order.
///
/// `src/`-stripped variants are offered because `ts_module_path` already strips a
/// leading `src/` when it builds a module path, so a `../` that climbs out of
/// `src` lands one segment too high. MEASURED: adding the variant lifts `../`
/// hits from 5,049 to 6,292.
fn local_module_candidates(class: &ImportTarget, current_module: &str, spec: &str) -> Vec<String> {
    let with_src_variant = |m: String| -> Vec<String> {
        match m.strip_prefix("src/") {
            Some(s) if !s.is_empty() => vec![m.clone(), s.to_string()],
            _ => vec![m],
        }
    };
    match class {
        ImportTarget::Relative => with_src_variant(resolve_relative(current_module, spec)),
        // `$lib` maps to `src/lib` by SvelteKit convention, and module paths are
        // already `src/`-relative, so the target module is `lib/…`. `$app` and
        // `$env` are FRAMEWORK modules with no file in this repo — they must stay
        // unresolved rather than mint a stub for something that does not exist.
        ImportTarget::Alias { kind: AliasKind::SvelteKit } => {
            let t = spec.trim();
            if let Some(rest) = t.strip_prefix("$lib/") {
                with_src_variant(format!("lib/{}", strip_ext(rest)))
            } else if t == "$lib" {
                vec!["lib".to_string()]
            } else {
                Vec::new()
            }
        }
        // `@/x` and `~/x` are tsconfig `paths` entries whose mapping is in
        // principle arbitrary, but MEASURED 6,413 of 7,032 such edges resolve by
        // simply stripping the prefix — the `@/*` → `./src/*` convention is
        // near-universal and module paths are already `src/`-relative. No config
        // read is needed for that, and a miss stays a miss.
        ImportTarget::Alias { kind: AliasKind::TsPaths } => {
            let t = spec.trim();
            match t.strip_prefix("@/").or_else(|| t.strip_prefix("~/")) {
                Some(rest) if !rest.is_empty() => with_src_variant(strip_ext(rest).to_string()),
                _ => Vec::new(),
            }
        }
        // Rust `crate::`/`super::`/`self::` needs the module-path arithmetic that
        // lives in `rust_lang`, not this string rewrite. Deliberately empty until
        // that is hoisted — an empty candidate list leaves the edge unresolved,
        // which is the honest outcome, not a wrong one.
        ImportTarget::Internal => Vec::new(),
        ImportTarget::External { .. } => Vec::new(),
    }
}

/// The FQN language segments to try for a module in `lang`.
///
/// The segment is NOT a function of the file's extension. MEASURED on live
/// `kind='module'` nodes: `.svelte` files carry 757 `svelte·…` fqns AND 544
/// `typescript·…`; `.js` files carry 1,023 `javascript·…` AND 1,072
/// `typescript·…`. The derivation copies the leading segment of the first def's
/// fqn and only falls back to the file's language, so the same file type lands on
/// different segments depending on what it happens to define.
///
/// So a single-segment lookup misses a real target for reasons that have nothing
/// to do with the import. Fanning out across the JS family recovers 881 relative
/// edges. Stabilising the segment instead would rewrite existing fqns, which is a
/// bigger decision than this slice.
fn fqn_language_candidates(lang: &str) -> Vec<&str> {
    match lang {
        "typescript" | "javascript" | "svelte" | "vue" => {
            let mut out = vec![lang];
            for l in ["typescript", "javascript", "svelte"] {
                if l != lang {
                    out.push(l);
                }
            }
            out
        }
        other => vec![other],
    }
}

/// Every FQN a LOCAL import specifier could resolve to, best guess first.
///
/// Empty for anything this cannot place — an external package, a `$app`/`$env`
/// framework module, a Rust internal path. An empty list means "leave the edge
/// unresolved", which is the honest answer; the caller must NOT invent a target
/// from a guess.
///
/// The caller must try these as a LOOKUP first and only create a node on a total
/// miss. Get-or-creating on the first candidate would poison the second: the
/// created stub would satisfy candidate 1 forever and the real target at
/// candidate 2 would never be found.
pub fn local_import_candidates(
    lang: &str,
    package: &str,
    current_module: &str,
    spec: &str,
    class: &ImportTarget,
) -> Vec<String> {
    let modules = local_module_candidates(class, current_module, spec);
    let mut out: Vec<String> = Vec::new();
    for m in &modules {
        if m.is_empty() {
            continue;
        }
        for l in fqn_language_candidates(lang) {
            let fqn = crate::languages::fqn::item(l, package, "", m);
            if !out.contains(&fqn) {
                out.push(fqn);
            }
        }
    }
    out
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

    // ── local_import_candidates ────────────────────────────────────────────
    //
    // 15,924 relative + 7,032 `@/`,`~/` + 1,836 `$lib` import edges point at
    // local code and NONE resolve, because `process.rs` inserts every import
    // edge with `target_id = None`. These pin the arithmetic that gives that
    // emit site something to look up.

    /// The relative case, and the one line that makes it correct: a specifier is
    /// relative to the importing file's DIRECTORY, so the current file's own
    /// segment must be dropped first.
    ///
    /// Breaking mutation: delete `parts.pop()` in `resolve_relative` — the
    /// candidate becomes `typescript·app·lib/builder/util`, i.e. resolving as if
    /// `builder` were a directory.
    #[test]
    fn a_relative_import_resolves_against_the_importing_files_directory() {
        let c = local_import_candidates(
            "typescript",
            "app",
            "lib/builder",
            "./util",
            &ImportTarget::Relative,
        );
        assert_eq!(c[0], "typescript·app·lib/util");

        // `../` climbs out of the directory.
        let c = local_import_candidates(
            "typescript",
            "app",
            "routes/admin/page",
            "../shared/table",
            &ImportTarget::Relative,
        );
        assert_eq!(c[0], "typescript·app·routes/shared/table");

        // An extension names the same module as the bare path.
        let c = local_import_candidates(
            "svelte",
            "app",
            "lib/x",
            "./Card.svelte",
            &ImportTarget::Relative,
        );
        assert_eq!(c[0], "svelte·app·lib/Card");
    }

    /// Aliases. `@/`,`~/` recover 6,413 of 7,032 edges by prefix strip alone —
    /// module paths are already `src/`-relative, so no tsconfig read is needed.
    ///
    /// Breaking mutation: delete the `$lib/` → `lib/` rewrite — `$lib/nav`
    /// produces no candidate and 1,836 edges stay unresolved.
    #[test]
    fn aliases_map_to_local_modules_but_framework_modules_do_not() {
        assert_eq!(
            local_import_candidates(
                "svelte",
                "app",
                "routes/page",
                "$lib/triage/view",
                &classify_import("$lib/triage/view")
            )[0],
            "svelte·app·lib/triage/view"
        );
        assert_eq!(
            local_import_candidates(
                "typescript",
                "app",
                "routes/page",
                "@/lib/x",
                &classify_import("@/lib/x")
            )[0],
            "typescript·app·lib/x"
        );
        assert_eq!(
            local_import_candidates(
                "typescript",
                "app",
                "routes/page",
                "~/stores/user",
                &classify_import("~/stores/user")
            )[0],
            "typescript·app·stores/user"
        );

        // `$app`/`$env` are SvelteKit FRAMEWORK modules — no file exists in the
        // repo, so they must yield NO candidate. Minting a stub for them would
        // fabricate a local module that cannot be opened.
        for framework in ["$app/navigation", "$app/state", "$env/dynamic/public"] {
            assert!(
                local_import_candidates(
                    "svelte",
                    "app",
                    "routes/page",
                    framework,
                    &classify_import(framework)
                )
                .is_empty(),
                "{framework} has no local file and must stay unresolved",
            );
        }
    }

    /// Nothing this cannot place may produce a candidate. An external package or
    /// a Rust internal path returning a guess here would resolve an edge to the
    /// wrong node, which is worse than leaving it unresolved.
    #[test]
    fn unplaceable_specifiers_yield_no_candidate() {
        for spec in ["react", "node:fs", "java.util.List", "@rokkit/ui"] {
            assert!(
                local_import_candidates("typescript", "app", "lib/x", spec, &classify_import(spec))
                    .is_empty(),
                "{spec} is external and must not produce a local candidate",
            );
        }
        // Rust internal paths need rust_lang's module arithmetic, not a string
        // rewrite — empty until that is hoisted.
        assert!(
            local_import_candidates(
                "rust",
                "senseid",
                "db::pg_store",
                "crate::db::graph",
                &classify_import("crate::db::graph")
            )
            .is_empty()
        );
    }

    /// The FQN language segment is not a function of the file type — live,
    /// `.svelte` files carry both `svelte·` and `typescript·` module fqns. A
    /// single-segment lookup therefore misses real targets for a reason unrelated
    /// to the import, so the JS family fans out.
    ///
    /// Breaking mutation: return `vec![lang]` from `fqn_language_candidates` —
    /// the cross-segment candidates vanish and 881 relative edges stop resolving.
    #[test]
    fn js_family_fans_out_across_unstable_fqn_language_segments() {
        let c =
            local_import_candidates("svelte", "app", "lib/x", "./util", &ImportTarget::Relative);
        assert_eq!(c[0], "svelte·app·lib/util", "the file's own language ranks first");
        assert!(c.contains(&"typescript·app·lib/util".to_string()));
        assert!(c.contains(&"javascript·app·lib/util".to_string()));

        // A non-JS language does NOT fan out — rust modules are not typescript.
        let c =
            local_import_candidates("python", "app", "pkg/mod", "./sib", &ImportTarget::Relative);
        assert_eq!(c, vec!["python·app·pkg/sib"]);
    }

    /// `ts_module_path` strips a leading `src/`, so a `../` that climbs out of
    /// `src` lands one segment high. Offering the stripped variant lifts `../`
    /// hits from 5,049 to 6,292 — measured.
    #[test]
    fn a_src_stripped_variant_is_offered_for_paths_that_climb_out_of_src() {
        let c = local_import_candidates(
            "typescript",
            "app",
            "routes/page",
            "../src/lib/x",
            &ImportTarget::Relative,
        );
        assert!(c.contains(&"typescript·app·src/lib/x".to_string()), "the literal path");
        assert!(c.contains(&"typescript·app·lib/x".to_string()), "and the src-stripped form");
    }
}
