//! The Rust adapter's compiler step: rust-analyzer's answers, read through
//! [`crate::indexer::lang::compiler`], turned into the identities this indexer
//! mints (docs/history/plan/2026-10-13-indexer-full-call-placement.md).
//!
//! A library target (the language's own `core`/`std`/`alloc`, or a dependency)
//! is named, never opened: a [`Form::Lib`] of the crate and a path written the way Rust
//! itself qualifies a name — `option::Option::Some`, `<T as Into<U>>::into`.

use scip::symbol::parse_symbol;
use scip::types::descriptor::Suffix;

use crate::indexer::facts::{Fqn, Reason};
use crate::indexer::fqn::{self, Form};
use crate::indexer::lang::compiler::{CompilerEngine, CompilerIndex, Declarations, Invocation};

/// The [`Form::Lib`] identity of a symbol rust-analyzer resolved into a library.
/// A symbol this cannot spell — a local, a malformed symbol, a descriptor shape
/// no Rust path names — is [`Reason::SymbolUnnamed`], a root cause to fix, never
/// a silent `None`.
pub fn library_target(symbol: &str) -> Result<Fqn, Reason> {
    let parsed = parse_symbol(symbol).map_err(|_| Reason::SymbolUnnamed)?;
    let package = parsed.package.as_ref().ok_or(Reason::SymbolUnnamed)?;
    let path = rust_path(&parsed.descriptors).ok_or(Reason::SymbolUnnamed)?;
    fqn::refer(&Form::Lib { package: package.name.as_str(), member: &path })
        .map_err(|_| Reason::SymbolUnnamed)
}

/// rust-analyzer, as the Rust adapter declares it.
pub struct RustEngine;

pub static RUST_ENGINE: RustEngine = RustEngine;

impl CompilerEngine for RustEngine {
    fn manifest(&self) -> &'static str {
        "Cargo.toml"
    }

    /// Cargo's own answer to "which workspace is this package in" — a nested
    /// package is in an ancestor workspace only if that workspace lists it, a
    /// rule this indexer does not restate.
    fn locate(&self, package_dir: &str) -> Invocation {
        Invocation {
            program: "cargo".into(),
            args: ["locate-project", "--workspace", "--message-format", "plain"]
                .map(String::from)
                .to_vec(),
            cwd: package_dir.to_string(),
        }
    }

    fn project_root(&self, located: &str) -> Option<String> {
        let manifest = located.trim();
        let root = manifest.strip_suffix("/Cargo.toml")?;
        (!root.is_empty()).then(|| root.to_string())
    }

    /// rustup names the toolchain's own rust-analyzer for this project — the
    /// pinned toolchain if the project pins one — however rustup was installed.
    fn locate_compiler(&self, project_root: &str) -> Invocation {
        Invocation {
            program: "rustup".into(),
            args: vec!["which".into(), "rust-analyzer".into()],
            cwd: project_root.to_string(),
        }
    }

    /// Without rustup (a nix or distro toolchain), the `rust-analyzer` on `PATH`
    /// is that toolchain's own.
    fn compiler_on_path(&self) -> Option<&'static str> {
        Some("rust-analyzer")
    }

    /// Rust sources and the manifests and lockfile that shape their build.
    fn sources(&self) -> &'static [&'static str] {
        &["rs", "toml", "lock"]
    }

    /// Every Cargo feature on, once per platform: code behind a feature or a
    /// `cfg(target_os)` is the repository's own source, and one default build
    /// compiles it out. Analysing for a target needs only the std SOURCES
    /// (rust-src), not that target's installed std.
    fn configs(&self) -> &'static [&'static str] {
        &[
            r#"{"cargo": {"features": "all", "target": "aarch64-apple-darwin"}}"#,
            r#"{"cargo": {"features": "all", "target": "x86_64-unknown-linux-gnu"}}"#,
            r#"{"cargo": {"features": "all", "target": "x86_64-pc-windows-msvc"}}"#,
        ]
    }

    fn compile(
        &self,
        compiler: &str,
        project_root: &str,
        output: &str,
        config: &str,
    ) -> Invocation {
        Invocation {
            program: compiler.to_string(),
            args: vec![
                "scip".into(),
                project_root.into(),
                "--output".into(),
                output.into(),
                "--config-path".into(),
                config.into(),
            ],
            cwd: project_root.to_string(),
        }
    }

    /// A closure invoked where it is written, or the function a call returned
    /// (tree-sitter's node kinds, as the walk records them).
    fn anonymous_callee(&self, node_kind: &str) -> bool {
        matches!(node_kind, "closure_expression" | "call_expression")
    }

    /// `handlers[i]()` — a function chosen by an index.
    fn dynamic_callee(&self, node_kind: &str) -> bool {
        node_kind == "index_expression"
    }

    /// A symbol the project DEFINES is the declaration the walk mints at that
    /// place; anything else is a library, named and never opened.
    fn target(
        &self,
        symbol: &str,
        index: &CompilerIndex,
        declarations: &dyn Declarations,
    ) -> Result<Fqn, Reason> {
        match index.definition_of(symbol) {
            Some(definition) => {
                let name = declared_name(symbol)?;
                declarations.declared_at(&definition.path, definition.line, &name)
            }
            None => library_target(symbol),
        }
    }
}

/// The name a symbol declares: its last descriptor.
fn declared_name(symbol: &str) -> Result<String, Reason> {
    let parsed = parse_symbol(symbol).map_err(|_| Reason::SymbolUnnamed)?;
    let last = parsed.descriptors.last().ok_or(Reason::SymbolUnnamed)?;
    Ok(last.name.clone())
}

/// Render descriptors as a Rust path. `impl#[Self]` and `impl#[Self][Trait]`
/// become `Self` and `<Self as Trait>`, the forms Rust writes a method of an
/// impl block with.
fn rust_path(descriptors: &[scip::types::Descriptor]) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    let mut i = 0;
    while i < descriptors.len() {
        let d = &descriptors[i];
        let suffix = d.suffix.enum_value().ok()?;
        match suffix {
            Suffix::Type if d.name == "impl" => {
                let mut params = Vec::new();
                while let Some(next) = descriptors.get(i + 1) {
                    if next.suffix.enum_value().ok()? != Suffix::TypeParameter {
                        break;
                    }
                    params.push(next.name.clone());
                    i += 1;
                }
                match params.as_slice() {
                    [ty] => parts.push(ty.clone()),
                    [ty, tr] => parts.push(format!("<{ty} as {tr}>")),
                    _ => return None,
                }
            }
            Suffix::Namespace | Suffix::Package | Suffix::Type | Suffix::Term | Suffix::Method => {
                parts.push(d.name.clone())
            }
            Suffix::Macro => parts.push(format!("{}!", d.name)),
            // A type parameter outside an impl header, a parameter, a local or
            // meta: no Rust path names these.
            _ => return None,
        }
        i += 1;
    }
    (!parts.is_empty()).then(|| parts.join("::"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lib(symbol: &str) -> Option<String> {
        library_target(symbol).ok().map(|f| f.to_string())
    }

    const CORE: &str = "rust-analyzer cargo core https://github.com/rust-lang/rust/library/core";
    const STD: &str = "rust-analyzer cargo std https://github.com/rust-lang/rust/library/std";
    const ALLOC: &str = "rust-analyzer cargo alloc https://github.com/rust-lang/rust/library/alloc";

    #[test]
    fn a_free_function_is_its_module_path() {
        assert_eq!(lib(&format!("{STD} env/var().")).as_deref(), Some("lib·std·env::var"));
        assert_eq!(
            lib("rust-analyzer cargo serde_json 1.0.149 de/from_str().").as_deref(),
            Some("lib·serde_json·de::from_str")
        );
    }

    #[test]
    fn a_macro_keeps_its_bang_so_it_never_merges_with_a_function() {
        assert_eq!(
            lib(&format!("{ALLOC} macros/format!")).as_deref(),
            Some("lib·alloc·macros::format!")
        );
        assert_eq!(
            lib(&format!("{CORE} macros/assert_eq!")).as_deref(),
            Some("lib·core·macros::assert_eq!")
        );
    }

    #[test]
    fn a_variant_and_a_trait_method_are_their_type_paths() {
        assert_eq!(
            lib(&format!("{CORE} option/Option#Some#")).as_deref(),
            Some("lib·core·option::Option::Some")
        );
        assert_eq!(
            lib(&format!("{CORE} iter/traits/iterator/Iterator#collect().")).as_deref(),
            Some("lib·core·iter::traits::iterator::Iterator::collect")
        );
    }

    #[test]
    fn an_impl_method_is_qualified_the_way_rust_writes_it() {
        assert_eq!(
            lib(&format!("{CORE} slice/impl#[`[T]`]iter().")).as_deref(),
            Some("lib·core·slice::[T]::iter")
        );
        assert_eq!(
            lib(&format!("{CORE} convert/impl#[T][`Into<U>`]into().")).as_deref(),
            Some("lib·core·convert::<T as Into<U>>::into")
        );
        assert_eq!(
            lib(&format!("{CORE} result/impl#[`Result<T, E>`]unwrap().")).as_deref(),
            Some("lib·core·result::Result<T, E>::unwrap")
        );
    }

    #[test]
    fn a_symbol_it_cannot_spell_is_a_named_root_cause() {
        assert_eq!(library_target("local 12"), Err(Reason::SymbolUnnamed));
        assert_eq!(library_target("not a symbol"), Err(Reason::SymbolUnnamed));
    }
}

#[cfg(test)]
mod engine_tests {
    use super::*;
    use crate::indexer::facts::Language;
    use crate::indexer::fqn::Reach;
    use crate::indexer::lang::compiler::{CompilerEngine, CompilerIndex, Declarations};

    const BIND: &str = "rust-analyzer cargo demo 0.1 net/impl#[Listener]bind().";
    const SOME: &str = "rust-analyzer cargo core https://github.com/rust-lang/rust/library/core option/Option#Some#";

    fn index() -> CompilerIndex {
        let mut decl = scip::types::Document::new();
        decl.relative_path = "src/net.rs".into();
        let mut d = scip::types::Occurrence::new();
        d.range = vec![9, 11, 15];
        d.symbol = BIND.into();
        d.symbol_roles = 1;
        decl.occurrences = vec![d];
        let mut ix = scip::types::Index::new();
        ix.documents = vec![decl];
        CompilerIndex::from_index(&ix)
    }

    /// The task layer's half, recorded: which declaration it was asked for.
    struct Asked(std::sync::Mutex<Vec<(String, u32, String)>>);
    impl Declarations for Asked {
        fn declared_at(&self, path: &str, line: u32, name: &str) -> Result<Fqn, Reason> {
            self.0.lock().unwrap().push((path.into(), line, name.into()));
            fqn::refer(&Form::Member {
                lang: Language::Rust,
                package: "demo",
                module: "net",
                ty: "Listener",
                member: name,
                reach: Reach::Item,
            })
            .map_err(|_| Reason::SymbolUnnamed)
        }
    }

    #[test]
    fn a_project_symbol_is_the_declaration_the_walk_mints_where_the_compiler_says_it_is() {
        let asked = Asked(Default::default());
        let got = RUST_ENGINE.target(BIND, &index(), &asked).expect("placed");
        assert_eq!(got.as_str(), "rust·demo·net·Listener·bind·item");
        assert_eq!(
            *asked.0.lock().unwrap(),
            vec![("src/net.rs".to_string(), 10, "bind".to_string())]
        );
    }

    #[test]
    fn a_symbol_with_no_definition_in_the_project_is_a_library_target() {
        let asked = Asked(Default::default());
        let got = RUST_ENGINE.target(SOME, &index(), &asked).expect("placed");
        assert_eq!(got.as_str(), "lib·core·option::Option::Some");
        assert!(asked.0.lock().unwrap().is_empty(), "a library is named, never opened");
    }

    /// Code behind a Cargo feature is compiled only when the feature is on, so
    /// the compiler is run with EVERY feature: a call in feature-gated code is
    /// still a call of the repository's own source.
    #[test]
    fn the_compiler_sees_every_feature_on_every_platform() {
        let mut targets = Vec::new();
        for config in RUST_ENGINE.configs() {
            let config: serde_json::Value = serde_json::from_str(config).expect("JSON");
            assert_eq!(config["cargo"]["features"], "all");
            targets.push(config["cargo"]["target"].as_str().unwrap_or_default().to_string());
        }
        // Code gated to a platform is compiled only for it, so each platform the
        // repository's own source targets is compiled and the answers merged.
        for os in ["linux", "windows", "apple-darwin"] {
            assert!(targets.iter().any(|t| t.contains(os)), "no compile for {os}: {targets:?}");
        }
    }

    #[test]
    fn a_function_chosen_by_an_index_is_dynamic() {
        assert!(RUST_ENGINE.dynamic_callee("index_expression"));
        assert!(!RUST_ENGINE.dynamic_callee("field_expression"));
    }

    #[test]
    fn an_anonymous_callee_is_a_closure_or_a_returned_function() {
        for kind in ["closure_expression", "call_expression"] {
            assert!(RUST_ENGINE.anonymous_callee(kind), "{kind}");
        }
        for kind in
            ["identifier", "field_expression", "scoped_identifier", "parenthesized_expression"]
        {
            assert!(!RUST_ENGINE.anonymous_callee(kind), "{kind}");
        }
    }

    #[test]
    fn the_engine_runs_rust_analyzer_over_the_cargo_workspace() {
        assert_eq!(RUST_ENGINE.manifest(), "Cargo.toml");
        let locate = RUST_ENGINE.locate("/repo/crates/a");
        assert_eq!(locate.program, "cargo");
        assert_eq!(locate.args, ["locate-project", "--workspace", "--message-format", "plain"]);
        assert_eq!(locate.cwd, "/repo/crates/a");
        assert_eq!(RUST_ENGINE.project_root("/repo/Cargo.toml\n").as_deref(), Some("/repo"));
        // The binary is LOCATED, not assumed on PATH: rustup names the
        // toolchain's rust-analyzer for this project (a pinned toolchain
        // included), whichever way rustup was installed.
        let which = RUST_ENGINE.locate_compiler("/repo");
        assert_eq!(which.program, "rustup");
        assert_eq!(which.args, ["which", "rust-analyzer"]);
        assert_eq!(which.cwd, "/repo");
        assert_eq!(RUST_ENGINE.compiler_on_path(), Some("rust-analyzer"), "the no-rustup fallback");
        let run =
            RUST_ENGINE.compile("/opt/ra/rust-analyzer", "/repo", "/tmp/out.scip", "/tmp/ra.json");
        assert_eq!(run.program, "/opt/ra/rust-analyzer");
        assert_eq!(
            run.args,
            ["scip", "/repo", "--output", "/tmp/out.scip", "--config-path", "/tmp/ra.json"]
        );
    }
}
