//! The TypeScript, JavaScript and Svelte adapters' compiler step: the
//! project's own TypeScript checker, run by `ts_compiler_driver.mjs`, read
//! through [`crate::indexer::lang::compiler`] and turned into the identities
//! this indexer mints (docs/plan/2026-10-13-indexer-full-call-placement.md).
//!
//! The driver writes SCIP with three kinds of symbol:
//! `local <n>` (never a node), `ts <path>:<line>:<name>` (declared in the
//! project) and `npm <package> <member>` (a library or the language itself,
//! named and never opened).

use crate::indexer::facts::{Fqn, Reason};
use crate::indexer::fqn::{self, Form};
use crate::indexer::lang::compiler::{CompilerEngine, CompilerIndex, Declarations, Invocation};

/// The driver, embedded: the daemon carries it, so nothing is installed beside
/// the binary but the npm packages it loads — which sensei-bootstrap installs
/// into its engines directory, passed to the driver.
const DRIVER: &str = include_str!("ts_compiler_driver.mjs");

/// TypeScript's checker, as the TypeScript, JavaScript and Svelte adapters
/// declare it.
pub struct TsEngine;

pub static TS_ENGINE: TsEngine = TsEngine;

impl CompilerEngine for TsEngine {
    fn manifest(&self) -> &'static str {
        "package.json"
    }

    /// A package is its own project: its `tsconfig.json` and its
    /// `node_modules` sit beside its `package.json`.
    fn locate(&self, package_dir: &str) -> Invocation {
        Invocation { program: "pwd".into(), args: vec![], cwd: package_dir.to_string() }
    }

    fn project_root(&self, located: &str) -> Option<String> {
        let root = located.trim();
        (!root.is_empty()).then(|| root.to_string())
    }

    /// The `node` this machine resolves, by absolute path.
    fn locate_compiler(&self, project_root: &str) -> Invocation {
        Invocation {
            program: "node".into(),
            args: vec!["-p".into(), "process.execPath".into()],
            cwd: project_root.to_string(),
        }
    }

    /// Scripts, components, and the JSON that configures them.
    fn sources(&self) -> &'static [&'static str] {
        &["ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs", "svelte", "json"]
    }

    /// One configuration: the driver reads the project's own `tsconfig.json`,
    /// so the file the task layer writes for it carries nothing.
    fn configs(&self) -> &'static [&'static str] {
        &["{}"]
    }

    fn compile(
        &self,
        compiler: &str,
        project_root: &str,
        output: &str,
        _config: &str,
    ) -> Invocation {
        Invocation {
            program: compiler.to_string(),
            args: vec![
                "--input-type=module".into(),
                "-e".into(),
                DRIVER.into(),
                project_root.into(),
                sensei_bootstrap::engines_dir().to_string_lossy().into_owned(),
                output.into(),
            ],
            cwd: project_root.to_string(),
        }
    }

    /// An arrow or function expression invoked where it is written, or the
    /// function a call returned (oxc's node kinds, as the walk records them).
    fn anonymous_callee(&self, node_kind: &str) -> bool {
        matches!(node_kind, "ArrowFunctionExpression" | "FunctionExpression" | "CallExpression")
    }

    fn target(
        &self,
        symbol: &str,
        _index: &CompilerIndex,
        declarations: &dyn Declarations,
    ) -> Result<Fqn, Reason> {
        if let Some(declared) = symbol.strip_prefix("ts ") {
            // `<path>:<line>:<name>` — read from the right: a path may hold `:`.
            let mut parts = declared.rsplitn(3, ':');
            let (Some(name), Some(line), Some(path)) = (parts.next(), parts.next(), parts.next())
            else {
                return Err(Reason::SymbolUnnamed);
            };
            let line = line.parse().map_err(|_| Reason::SymbolUnnamed)?;
            return declarations.declared_at(path, line, name);
        }
        // The driver's mark for a call on a receiver typed `any`.
        if symbol == "untyped" {
            return Err(Reason::UntypedReceiver);
        }
        // The driver's mark for a member no declaration names: whichever
        // function each value holds.
        if symbol == "dynamic" {
            return Err(Reason::DynamicDispatch);
        }
        if let Some(library) = symbol.strip_prefix("npm ") {
            let (package, member) = library.split_once(' ').ok_or(Reason::SymbolUnnamed)?;
            return fqn::refer(&Form::Lib { package, member }).map_err(|_| Reason::SymbolUnnamed);
        }
        Err(Reason::SymbolUnnamed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `declared_at` question: path, line, name.
    type Question = (String, u32, String);

    struct Asked(std::sync::Mutex<Vec<Question>>);

    impl Declarations for Asked {
        fn declared_at(&self, path: &str, line: u32, name: &str) -> Result<Fqn, Reason> {
            self.0.lock().unwrap().push((path.into(), line, name.into()));
            fqn::refer(&Form::Lib { package: "asked", member: name })
                .map_err(|_| Reason::SymbolUnnamed)
        }
    }

    fn target(symbol: &str) -> (Result<Fqn, Reason>, Vec<Question>) {
        let asked = Asked(std::sync::Mutex::new(Vec::new()));
        let index = CompilerIndex::from_index(&scip::types::Index::new());
        let got = TS_ENGINE.target(symbol, &index, &asked);
        (got, asked.0.into_inner().unwrap())
    }

    fn lib(package: &str, member: &str) -> Fqn {
        fqn::refer(&Form::Lib { package, member }).unwrap()
    }

    /// A library — a package, Svelte's runes, the language's own globals — is
    /// the `lib·` node the walk names it by, never opened.
    #[test]
    fn a_library_symbol_is_its_package_and_member() {
        assert_eq!(target("npm @sveltejs/kit resolve").0, Ok(lib("@sveltejs/kit", "resolve")));
        assert_eq!(target("npm svelte $state").0, Ok(lib("svelte", "$state")));
        assert_eq!(target("npm ecmascript Array.map").0, Ok(lib("ecmascript", "Array.map")));
    }

    /// A project symbol is the declaration the walk mints where the checker
    /// says it is declared — asked of the task layer, never guessed.
    #[test]
    fn a_project_symbol_is_the_declaration_at_its_place() {
        let (got, asked) = target("ts src/lib/a b.ts:12:load");
        assert_eq!(got, Ok(lib("asked", "load")));
        assert_eq!(asked, vec![("src/lib/a b.ts".to_string(), 12, "load".to_string())]);
    }

    /// A call on a receiver the checker typed `any` (an import it could not
    /// resolve, untyped JavaScript) has no static callee: the driver marks it,
    /// and that is its root cause.
    #[test]
    fn a_call_on_an_untyped_receiver_says_so() {
        assert_eq!(target("untyped").0, Err(Reason::UntypedReceiver));
    }

    /// A member of an anonymous object type is whichever function each value
    /// holds: the driver marks it, and no declaration answers.
    #[test]
    fn a_member_no_declaration_names_is_dynamic_dispatch() {
        assert_eq!(target("dynamic").0, Err(Reason::DynamicDispatch));
    }

    /// A symbol this cannot read is a named root cause, never a silent miss.
    #[test]
    fn a_symbol_it_cannot_read_is_a_named_root_cause() {
        for bad in ["ts src/a.ts:x:load", "ts nothing", "npm onlypackage", "scip-typescript x", ""]
        {
            assert_eq!(target(bad).0, Err(Reason::SymbolUnnamed), "{bad:?}");
        }
    }

    /// Which callees are anonymous functions, in the walk's own words: an
    /// arrow or function expression invoked where it is written, or the
    /// function a call returned.
    #[test]
    fn an_anonymous_callee_is_a_function_literal_or_a_returned_function() {
        for kind in ["ArrowFunctionExpression", "FunctionExpression", "CallExpression"] {
            assert!(TS_ENGINE.anonymous_callee(kind), "{kind}");
        }
        for kind in ["Identifier", "StaticMemberExpression", "Super", "ComputedMemberExpression"] {
            assert!(!TS_ENGINE.anonymous_callee(kind), "{kind}");
        }
    }

    /// Every file the checker reads — TypeScript, JavaScript, a Svelte
    /// component — is placed through it.
    #[test]
    fn typescript_javascript_and_svelte_files_are_placed_through_the_checker() {
        for ext in [".ts", ".tsx", ".mts", ".js", ".jsx", ".mjs", ".cjs", ".svelte"] {
            let adapter = crate::indexer::lang::adapter_for_ext(ext).expect(ext);
            let engine = adapter.compiler().unwrap_or_else(|| panic!("{ext} has no compiler"));
            assert!(std::ptr::addr_eq(engine, &TS_ENGINE as &dyn CompilerEngine), "{ext}");
        }
    }

    /// The package directory is the project; node runs the embedded driver over
    /// it — the node that `node` resolves to on this machine, located first —
    /// and tells it where bootstrap installs the packages a project lacks.
    #[test]
    fn the_engine_runs_the_embedded_driver_with_node_over_the_package() {
        assert_eq!(TS_ENGINE.manifest(), "package.json");
        let located = TS_ENGINE.locate("/r/dojo");
        assert_eq!((located.program.as_str(), located.cwd.as_str()), ("pwd", "/r/dojo"));
        assert_eq!(TS_ENGINE.project_root("/r/dojo\n"), Some("/r/dojo".to_string()));
        assert_eq!(TS_ENGINE.project_root("\n"), None);
        let node = TS_ENGINE.locate_compiler("/r/dojo");
        assert_eq!(node.program, "node");
        assert_eq!(node.args, vec!["-p".to_string(), "process.execPath".to_string()]);
        assert_eq!(TS_ENGINE.configs().len(), 1);
        let compile = TS_ENGINE.compile("/bin/node", "/r/dojo", "/tmp/o.scip", "/tmp/c.json");
        assert_eq!(compile.program, "/bin/node");
        assert_eq!(compile.cwd, "/r/dojo");
        assert_eq!(
            compile.args,
            vec![
                "--input-type=module".to_string(),
                "-e".to_string(),
                DRIVER.to_string(),
                "/r/dojo".to_string(),
                sensei_bootstrap::engines_dir().to_string_lossy().into_owned(),
                "/tmp/o.scip".to_string(),
            ]
        );
    }
}
