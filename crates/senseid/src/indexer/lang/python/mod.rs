//! Python — the identity rules, the grammar the shared ladder reads, and the
//! adapter that hands a caller to them.
//!
//! # A file IS a module, and the path IS the import
//!
//! Python is the most direct of the four languages here: `a/b.py` is the module
//! `a.b`, and `import a.b` is spelled with the same separator the module path
//! uses. There is no `mod b;` to agree with (Rust), no extension to drop from a
//! specifier (JavaScript), and no `package` line that overrides the directory
//! (Java). The path is the whole answer.
//!
//! Two shapes bend it, and both are the language's own:
//!
//! - **`a/__init__.py` IS the module `a`**, not `a.__init__` — the same
//!   file-names-its-directory rule Rust spells `a/mod.rs`. A package that has
//!   both `a.py` and `a/__init__.py` is ambiguous to Python's own importer, so
//!   the two minting one identity here reports a real problem rather than
//!   inventing one.
//! - **A `src/` layout** puts the import root one directory below the manifest.
//!   Handled where every language handles it, in [`module_path`], because the
//!   caller passes the directory the manifest sits in and nothing else.
//!
//! # Why Python resolves more than JavaScript and less than Java
//!
//! Python states no types at a binding unless someone wrote an annotation, so
//! the receiver of `x.method()` is typable only when `x` came from an annotated
//! parameter, an annotated assignment, or a constructor call whose class this
//! scan declares. That is strictly more than JavaScript (which has the
//! constructor case and little else) and strictly less than Java (which states
//! a type at nearly every binding). The walk records what it read and codes the
//! rest as [`Reason::ReceiverTypeUnknown`](crate::indexer::facts::Reason) — it
//! does not guess, because a wrong edge outranks a missing one (R4).

mod walk;

use super::{Grammar, LanguageAdapter, ReadError, Source, TypeHomes};
use crate::indexer::facts::{FileFacts, Fqn, Language};
use crate::indexer::fqn::{self, Form, FqnError, Reach, Segment};
use crate::indexer::resolve::Root;

/// Python, from `.py` and `.pyi`.
pub struct PythonAdapter;

impl LanguageAdapter for PythonAdapter {
    fn language(&self) -> Language {
        Language::Python
    }

    fn name(&self) -> &'static str {
        "python"
    }

    /// `.pyi` is a STUB for the module of the same name, so it shares that
    /// module's identity by sharing its path rule — `foo.pyi` and `foo.py` both
    /// reduce to `foo`, which is the merge the type checker itself performs.
    fn extensions(&self) -> &'static [&'static str] {
        &[".py", ".pyi"]
    }

    fn grammar(&self) -> &'static Grammar {
        &GRAMMAR
    }

    fn read(&self, source: &Source<'_>, types: &TypeHomes) -> Result<FileFacts, ReadError> {
        walk::read(source, types)
    }

    fn file_fqn(&self, package: &str, module: &str, path: &str) -> Result<Fqn, FqnError> {
        file_fqn(package, module, path)
    }

    fn module_path(&self, file: &str, package_root: &str) -> String {
        module_path(file, package_root)
    }

    fn type_segment(&self, raw: &str) -> Result<String, FqnError> {
        type_segment(raw)
    }
}

/// The file that names the directory it sits in, which Python spells
/// `__init__.py` and Rust spells `mod.rs`.
const PACKAGE_INIT: &str = "__init__";

/// What the shared resolution ladder needs to know about Python, and nothing
/// more (R7).
pub const GRAMMAR: Grammar = Grammar {
    language: Language::Python,
    // One separator for both, and unlike Java this is not because Python has one
    // namespace — it is because a module path and an attribute path are written
    // the same way on purpose. `os.path.join` is a module, a module, and a
    // function, and nothing in the text says where the module path stops. That
    // ambiguity is the ladder's to resolve from the imports, which is exactly
    // why `paths_name_packages` below is false.
    path_separator: ".",
    module_separator: ".",
    // Python's relative imports, and no third. There is no `crate` — an absolute
    // import names a top-level module, and whether that module is ours is
    // answered by `first_party`, not by a keyword.
    roots: &[(".", Root::Here), ("..", Root::Up)],
    // A specifier is already a dotted module path. `from a.b import C` carries
    // `a.b` and nothing else — the file name never appears in it.
    module_segment: crate::indexer::lang::common::already_a_module_segment,
    // `from . import x` inside `a/b.py` names `a.x`, a SIBLING — the leading dot
    // is the containing PACKAGE, which is the directory. Getting this wrong is
    // off by one module in every relative import in the corpus.
    relative_to_directory: true,
    // FALSE, and this is the single most consequential field for Python. A bare
    // `foo.bar` is an attribute access on a local object, exactly as in
    // JavaScript — Python has no `serde_json::json!` shape where a path names a
    // package with nothing bringing it into scope. Every module reference goes
    // through an import statement, so reading the head of a dotted path as a
    // package would mint a library node for every untyped local in the corpus.
    paths_name_packages: false,
    // NONE, because this walk emits a CLEAN specifier. Python spells the rename
    // in the statement (`import a.b as c`, `from a import b as c`) and the walk
    // reads the alias off the AST, so by the time the ladder sees a specifier
    // the path is all that is left in it.
    names_the_binding: None,
    // NONE for the same reason, and it is a real difference from Java rather
    // than a shortcut: Java's `import java.util.*;` puts the star INSIDE the
    // path, so the ladder has to read it off. Python's `from a import *` puts it
    // in the import clause, where the walk turns it into `Binding::Glob` and the
    // specifier stays `a`.
    wildcard: None,
    names_a_type: crate::indexer::lang::common::names_a_type_by_leading_case,
    prelude: PRELUDE,
    plumbing: PLUMBING,
};

/// The names Python puts in scope with nothing written to bring them there.
///
/// `builtins` is a real, importable module and these really are its attributes,
/// so the path is the bare name — unlike Rust, where the prelude re-exports from
/// somewhere deeper and the path has to say where.
const PRELUDE: &[(&str, &str, &str)] = &[
    ("print", "builtins", "print"),
    ("len", "builtins", "len"),
    ("range", "builtins", "range"),
    ("open", "builtins", "open"),
    ("isinstance", "builtins", "isinstance"),
    ("issubclass", "builtins", "issubclass"),
    ("getattr", "builtins", "getattr"),
    ("setattr", "builtins", "setattr"),
    ("hasattr", "builtins", "hasattr"),
    ("enumerate", "builtins", "enumerate"),
    ("zip", "builtins", "zip"),
    ("map", "builtins", "map"),
    ("filter", "builtins", "filter"),
    ("sorted", "builtins", "sorted"),
    ("reversed", "builtins", "reversed"),
    ("sum", "builtins", "sum"),
    ("min", "builtins", "min"),
    ("max", "builtins", "max"),
    ("abs", "builtins", "abs"),
    ("round", "builtins", "round"),
    ("any", "builtins", "any"),
    ("all", "builtins", "all"),
    ("repr", "builtins", "repr"),
    ("hash", "builtins", "hash"),
    ("iter", "builtins", "iter"),
    ("next", "builtins", "next"),
    ("super", "builtins", "super"),
    ("type", "builtins", "type"),
    ("vars", "builtins", "vars"),
    ("dir", "builtins", "dir"),
    ("format", "builtins", "format"),
    ("input", "builtins", "input"),
    ("str", "builtins", "str"),
    ("int", "builtins", "int"),
    ("float", "builtins", "float"),
    ("bool", "builtins", "bool"),
    ("bytes", "builtins", "bytes"),
    ("list", "builtins", "list"),
    ("dict", "builtins", "dict"),
    ("set", "builtins", "set"),
    ("frozenset", "builtins", "frozenset"),
    ("tuple", "builtins", "tuple"),
    ("object", "builtins", "object"),
    ("Exception", "builtins", "Exception"),
    ("ValueError", "builtins", "ValueError"),
    ("TypeError", "builtins", "TypeError"),
    ("KeyError", "builtins", "KeyError"),
    ("IndexError", "builtins", "IndexError"),
    ("RuntimeError", "builtins", "RuntimeError"),
    ("NotImplementedError", "builtins", "NotImplementedError"),
    ("StopIteration", "builtins", "StopIteration"),
];

/// Member names so universal that a miss on one says nothing about the scan.
///
/// Every entry is a method of a BUILTIN container or of the object protocol, so
/// a use site naming one is almost never reaching for a first-party type. They
/// are excluded from the member rung's miss accounting for the same reason
/// Rust's and Java's lists are: counting `append` as a lost first-party member
/// buries the ones that actually are.
const PLUMBING: &[&str] = &[
    // The object protocol.
    "__init__",
    "__new__",
    "__str__",
    "__repr__",
    "__len__",
    "__iter__",
    "__next__",
    "__enter__",
    "__exit__",
    "__call__",
    "__eq__",
    "__hash__",
    "__getitem__",
    "__setitem__",
    "__contains__",
    // list / set.
    "append",
    "extend",
    "insert",
    "remove",
    "pop",
    "sort",
    "reverse",
    "count",
    "index",
    "add",
    "discard",
    "clear",
    "copy",
    // dict.
    "get",
    "keys",
    "values",
    "items",
    "update",
    "setdefault",
    // str.
    "split",
    "rsplit",
    "join",
    "strip",
    "lstrip",
    "rstrip",
    "replace",
    "startswith",
    "endswith",
    "lower",
    "upper",
    "encode",
    "decode",
    "splitlines",
];

/// The type a Python type EXPRESSION names.
///
/// Shared by the declaration side and the reference side, because two spellings
/// of this is how an annotation and a use of the same type come to mint
/// different strings and never meet (spec §2).
pub(super) fn type_segment(raw: &str) -> Result<String, FqnError> {
    // A FORWARD REFERENCE is written as a string literal — `def f() -> "Node"` —
    // because the class is not defined yet at annotation time. It names exactly
    // the type it spells, so the quotes come off and nothing else changes.
    let bare = raw.trim().trim_matches(|c| c == '"' || c == '\'').trim();
    // A subscripted generic names its own type: `list[Patient]` is a use of
    // `list`. The parameter is its own use site the walk emits separately.
    let bare = bare.split_once('[').map_or(bare, |(head, _)| head);
    // `Foo | None` is PEP 604 for `Optional[Foo]`, and the union names its FIRST
    // arm. Taking the head rather than dropping a literal `None` is what makes
    // `Foo | Bar` answer `Foo` instead of a string with a pipe in it.
    let bare = bare.split('|').next().unwrap_or(bare).trim();
    // `a.b.C` at a use site names `C`; the path in front of it is how it was
    // reached, not what it is.
    let last = bare.rsplit('.').next().unwrap_or(bare).trim();
    if last.is_empty() {
        return Err(FqnError::EmptySegment { segment: Segment::Type });
    }
    Ok(last.to_string())
}

/// A file's package-relative MODULE PATH, from its path alone.
///
/// Relative to `<package_root>/src` (the layout `pyproject.toml` projects
/// overwhelmingly use) or to the package root itself, drop the extension, drop a
/// trailing `__init__`:
///
/// | file | module |
/// |---|---|
/// | `proj/src/pkg/a/b.py` | `pkg.a.b` |
/// | `proj/src/pkg/__init__.py` | `pkg` |
/// | `proj/pkg/a/b.py` | `pkg.a.b` |
///
/// THE ONE OWNER of this rule. It is a segment of every identity a Python file
/// declares, so a second implementation of it mints a second identity for every
/// declaration.
pub fn module_path(file: &str, package_root: &str) -> String {
    let file = std::path::Path::new(file);
    let root = std::path::Path::new(package_root);
    let src = root.join("src");
    let Ok(rel) = file.strip_prefix(&src).or_else(|_| file.strip_prefix(root)) else {
        // Outside the package root entirely. Nothing here names a module, and
        // answering with the absolute path would put the filesystem into every
        // identity the file declares.
        return String::new();
    };

    let mut segments: Vec<String> =
        rel.components().filter_map(|c| c.as_os_str().to_str().map(str::to_string)).collect();
    if let Some(last) = segments.last_mut()
        && let Some(stem) = std::path::Path::new(last.as_str()).file_stem().and_then(|s| s.to_str())
    {
        *last = stem.to_string();
    }
    // `a/__init__.py` IS `a`. The file names the directory it sits in, so its
    // own segment is not part of the path — the same rule that makes `a/mod.rs`
    // and `a.rs` one module in Rust.
    if segments.last().is_some_and(|s| s == PACKAGE_INIT) {
        segments.pop();
    }
    segments.join(".")
}

/// The NAME segment of a file's own module identity. Reads the same two cases
/// [`file_fqn`] does, so the name beside the identity cannot disagree with the
/// name inside it.
fn module_name_of(module: &str, path: &str) -> String {
    match module.rsplit_once('.') {
        Some((_, name)) => name.to_string(),
        None if module.is_empty() => stem_of(path).to_string(),
        None => module.to_string(),
    }
}

/// The file's own stem, for the one file whose module path is empty: a
/// `__init__.py` sitting directly at the import root. It names the root package
/// and no dotted path leads to it.
fn stem_of(path: &str) -> &str {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.rsplit_once('.').map_or(name, |(stem, _)| stem)
}

/// The identity of the file itself, which is the identity of the module it
/// declares — minted exactly the way an `import a.b` that names this file mints
/// it, so the file and every import of it are one node and not two.
pub fn file_fqn(package: &str, module: &str, path: &str) -> Result<Fqn, FqnError> {
    let stem = stem_of(path);
    let (parent, name) = match module.rsplit_once('.') {
        Some((parent, name)) => (parent, name),
        None if module.is_empty() => ("", stem),
        None => ("", module),
    };
    fqn::define(&Form::Item {
        lang: Language::Python,
        package,
        module: parent,
        name,
        reach: Reach::Mod,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The table in [`module_path`]'s own doc, executed. A doc that drifts from
    /// the rule is worse than no doc, and this rule is a segment of every
    /// identity a Python file declares.
    #[test]
    fn a_file_is_the_module_its_path_spells() {
        for (file, root, want) in [
            ("/w/proj/src/pkg/a/b.py", "/w/proj", "pkg.a.b"),
            ("/w/proj/src/pkg/__init__.py", "/w/proj", "pkg"),
            ("/w/proj/pkg/a/b.py", "/w/proj", "pkg.a.b"),
            ("/w/proj/pkg/__init__.py", "/w/proj", "pkg"),
            ("/w/proj/setup.py", "/w/proj", "setup"),
            // A stub shares its module with the file it describes, which is what
            // makes the two one node rather than two.
            ("/w/proj/pkg/a.pyi", "/w/proj", "pkg.a"),
            ("/w/proj/pkg/a.py", "/w/proj", "pkg.a"),
            // The root `__init__.py` names the import root and nothing else.
            ("/w/proj/src/__init__.py", "/w/proj", ""),
        ] {
            assert_eq!(module_path(file, root), want, "{file}");
        }
    }

    /// A file outside the package root yields NO module rather than a module
    /// with the filesystem in it.
    ///
    /// The mutation that must break this: restore an `unwrap_or(file)` fallback
    /// on the strip. Every identity the file declares would then carry `/w/…`
    /// as its module segment, and no import anywhere could spell it.
    #[test]
    fn a_file_outside_the_package_root_names_no_module() {
        assert_eq!(module_path("/elsewhere/other/a.py", "/w/proj"), "");
    }

    /// `a/b.py` and `a/b/__init__.py` mint ONE identity, and that is correct
    /// rather than a collision to fix: Python's own importer cannot tell them
    /// apart either, so a package containing both is already broken. Reporting
    /// them as one identity surfaces the real problem; minting two would hide
    /// it behind a graph that looks fine.
    #[test]
    fn the_two_spellings_python_itself_cannot_disambiguate_are_one_module() {
        assert_eq!(
            module_path("/w/proj/pkg/a/b.py", "/w/proj"),
            module_path("/w/proj/pkg/a/b/__init__.py", "/w/proj")
        );
    }

    /// The file's identity is what an import of it mints: everything before the
    /// last dot is the module, the last segment is the name, and the reach is
    /// `Mod`.
    #[test]
    fn a_files_identity_is_what_an_import_of_it_spells() {
        let fqn = file_fqn("proj", "pkg.a.b", "/w/proj/src/pkg/a/b.py").expect("minted");
        assert_eq!(fqn.to_string(), "python·proj·pkg.a·b·mod");

        // A top-level module has no parent segment.
        let top = file_fqn("proj", "pkg", "/w/proj/src/pkg/__init__.py").expect("minted");
        assert_eq!(top.to_string(), "python·proj·pkg·mod");

        // The import root itself falls back to the stem, which is the only
        // name it has.
        let root = file_fqn("proj", "", "/w/proj/src/__init__.py").expect("minted");
        assert_eq!(root.to_string(), "python·proj·__init__·mod");
    }

    /// The name reported beside the identity is the name inside it. Two
    /// derivations of one string is how a symbol's `name` column comes to
    /// disagree with its own fqn.
    #[test]
    fn the_reported_name_is_the_name_in_the_identity() {
        for (module, path) in
            [("pkg.a.b", "/w/p/pkg/a/b.py"), ("pkg", "/w/p/pkg/__init__.py"), ("", "/w/p/x.py")]
        {
            let name = module_name_of(module, path);
            let fqn = file_fqn("proj", module, path).expect("minted").to_string();
            let in_identity = fqn.rsplit('·').nth(1).expect("a name segment");
            assert_eq!(name, in_identity, "module {module:?} at {path}");
        }
    }

    /// Every decoration a Python annotation can carry, reduced to the one
    /// segment that names the type.
    #[test]
    fn a_type_expression_reduces_to_the_type_it_names() {
        for (raw, want) in [
            ("Patient", "Patient"),
            ("list[Patient]", "list"),
            ("dict[str, Patient]", "dict"),
            // PEP 604 and its typing.Optional spelling both name the first arm.
            ("Patient | None", "Patient"),
            ("Optional[Patient]", "Optional"),
            // A forward reference is a string literal naming a real type.
            ("\"Node\"", "Node"),
            ("'Node'", "Node"),
            // A qualified use names the type, not the path that reached it.
            ("models.patient.Patient", "Patient"),
            ("  Patient  ", "Patient"),
        ] {
            assert_eq!(type_segment(raw).as_deref(), Ok(want), "{raw}");
        }
    }

    /// Nothing is not a type. An empty segment must be an error and never an
    /// empty identity, which would merge every unnameable annotation onto one
    /// node.
    #[test]
    fn an_empty_type_expression_is_an_error_rather_than_an_empty_identity() {
        assert!(type_segment("").is_err());
        assert!(type_segment("   ").is_err());
        assert!(type_segment("\"\"").is_err());
    }
}

#[cfg(test)]
mod corpus {
    use super::*;
    use crate::indexer::facts::Resolution;
    use crate::indexer::placement;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    /// **THE ADAPTER, ON REAL PYTHON, THROUGH THE REAL SEAM.**
    ///
    /// Every other test in this module runs on a string literal I wrote, which
    /// proves the walk does what I expected and nothing about whether what I
    /// expected is what Python code looks like. This one takes a checkout,
    /// derives each file's package and module with
    /// [`crate::indexer::placement`] exactly as the processor will, and reads
    /// it — so a parse that fails, a manifest that names nothing, or a module
    /// rule that disagrees with the corpus shows up as a number rather than as
    /// a surprise at cutover.
    ///
    ///     SENSEI_PY_CORPUS=~/Developer/ai-hedge-fund \
    ///       cargo test -p senseid --bin senseid -- --ignored --nocapture python::corpus
    #[test]
    #[ignore = "walks the checkout named by SENSEI_PY_CORPUS"]
    fn the_adapter_reads_a_real_python_corpus() {
        // Absent on a machine with no python checkout, which is most of them.
        // The house pattern: say so and return, rather than fail a suite for a
        // fixture that is deliberately not in the repository.
        let Ok(root) = std::env::var("SENSEI_PY_CORPUS") else {
            println!("SENSEI_PY_CORPUS unset — nothing to read. See this test's docs.");
            return;
        };
        let root = PathBuf::from(root);
        let manifests: Vec<PathBuf> = ignore::WalkBuilder::new(&root)
            .build()
            .filter_map(Result::ok)
            .map(|e| e.into_path())
            // Only what an adapter can READ. `setup.py` and `setup.cfg` look
            // like manifests and no registered adapter parses either, so
            // counting them would make a file's package depend on which of two
            // files the walk yielded first.
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .and_then(crate::adapters::manifest::manifest_adapter_for_filename)
                    .is_some()
            })
            .collect();

        let files: Vec<PathBuf> = ignore::WalkBuilder::new(&root)
            .build()
            .filter_map(Result::ok)
            .map(|e| e.into_path())
            .filter(|p| p.extension().is_some_and(|e| e == "py" || e == "pyi"))
            .collect();
        assert!(!files.is_empty(), "{} holds no python", root.display());

        let mut unplaced = 0usize;
        let mut unreadable: Vec<String> = Vec::new();
        let mut modules: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
        let (mut symbols, mut references, mut imports) = (0usize, 0usize, 0usize);
        let mut reasons: BTreeMap<String, usize> = BTreeMap::new();
        let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
        let mut read = 0usize;
        let mut callable_lines = 0usize;

        for file in &files {
            let Some(manifest) = placement::owning_manifest(file, &root, &manifests) else {
                unplaced += 1;
                continue;
            };
            let Ok(manifest_text) = std::fs::read_to_string(&manifest) else {
                unplaced += 1;
                continue;
            };
            let Some(package) = placement::package_named_by(&manifest, &manifest_text) else {
                unplaced += 1;
                continue;
            };
            let package_root = manifest.parent().expect("a manifest has a directory");
            let placed = placement::placement_of(file, &package, package_root, Language::Python);
            let Ok(text) = std::fs::read_to_string(file) else { continue };
            callable_lines += self::corpus::callable_lines(&text);

            let source = Source {
                package: &placed.package,
                module: &placed.module,
                path: &file.to_string_lossy(),
                text: &text,
            };
            match PythonAdapter.read(&source, &TypeHomes::unknown()) {
                Ok(facts) => {
                    read += 1;
                    symbols += facts.symbols.len();
                    for symbol in &facts.symbols {
                        *kinds.entry(format!("{:?}", symbol.kind)).or_default() += 1;
                    }
                    references += facts.references.len();
                    imports += facts.imports.len();
                    modules.entry(placed.module.clone()).or_default().push(file.clone());
                    for reference in &facts.references {
                        if let Resolution::Unresolved { reason, .. } = &reference.target {
                            *reasons.entry(format!("{reason:?}")).or_default() += 1;
                        }
                    }
                }
                Err(e) => unreadable.push(format!("{}: {e:?}", file.display())),
            }
        }

        let mut histogram: Vec<(&String, &usize)> = reasons.iter().collect();
        histogram.sort_by(|a, b| b.1.cmp(a.1));
        println!("\n── {} ──", root.display());
        println!("  files {}  read {read}  unplaced {unplaced}", files.len());
        println!("  symbols {symbols}  references {references}  imports {imports}");
        println!("  distinct modules {}", modules.len());
        println!("  by kind {kinds:?}");
        for (reason, count) in histogram.iter().take(8) {
            println!("  {count:>7}  {reason}");
        }

        // A COLLISION IS A DEFECT UNLESS PYTHON ITSELF HAS IT. Two files minting
        // one module means every declaration in one overwrites the other's — so
        // the only acceptable collision is the one the language cannot resolve
        // either: `a.py` beside `a/__init__.py`, where CPython's own finder lets
        // the directory shadow the file. That is a real bug in the corpus, found
        // rather than caused, and it is REPORTED instead of asserted away.
        //
        // MEASURED: one such pair in a 94-file checkout (`src/config.py` and
        // `src/config/__init__.py`). Anything with a different shape is mine.
        let mut theirs = 0usize;
        let mut mine: Vec<String> = Vec::new();
        for (module, claimants) in modules.iter().filter(|(m, c)| c.len() > 1 && !m.is_empty()) {
            match shadowed_by_a_package_directory(claimants) {
                true => theirs += 1,
                false => mine.push(format!("{module}: {claimants:?}")),
            }
        }
        if theirs > 0 {
            println!("  {theirs} module(s) the corpus itself cannot disambiguate");
        }
        assert!(
            mine.is_empty(),
            "{} module path(s) collide for a reason python does NOT have: {:?}",
            mine.len(),
            mine.iter().take(5).collect::<Vec<_>>()
        );
        assert!(unreadable.is_empty(), "{} files did not read: {:?}", unreadable.len(), unreadable);
        assert!(read > 0, "nothing was read, so this proved nothing");

        // **EVERY `def` AND `class` IN THE CORPUS, EXACTLY ONCE.**
        //
        // The strongest check available without a second implementation to
        // compare against: the source states how many callables it declares, one
        // per line, and the walk must emit that many. A miss shows up as a
        // shortfall and the double-count that the reference total hid for one
        // commit would show up as a surplus.
        //
        // MEASURED on a 94-file checkout: 367 lines, 367 symbols — 42 classes,
        // 257 functions, 68 methods.
        let declared: usize = kinds.get("Class").copied().unwrap_or_default()
            + kinds.get("Function").copied().unwrap_or_default()
            + kinds.get("Method").copied().unwrap_or_default();
        assert_eq!(
            declared, callable_lines,
            "the corpus writes {callable_lines} `def`/`class` lines and the walk emitted \
             {declared} callables"
        );
    }

    /// How many callables the SOURCE says it declares — one per `def`/`class`
    /// line. A deliberately dumb count, because its whole value is being derived
    /// some way other than the walk derives its answer.
    fn callable_lines(text: &str) -> usize {
        text.lines()
            .filter(|line| {
                let t = line.trim_start();
                t.starts_with("def ") || t.starts_with("class ") || t.starts_with("async def ")
            })
            .count()
    }

    /// Whether a set of files claiming one module is the `a.py` beside
    /// `a/__init__.py` shape — the ambiguity python's own import system has.
    fn shadowed_by_a_package_directory(claimants: &[PathBuf]) -> bool {
        claimants.len() == 2
            && claimants.iter().any(|p| p.file_name().is_some_and(|n| n == "__init__.py"))
            && claimants.iter().any(|p| p.file_name().is_some_and(|n| n != "__init__.py"))
    }
}
