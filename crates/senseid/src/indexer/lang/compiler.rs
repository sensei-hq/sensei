//! What a language's COMPILER says each name in a file refers to — read from the
//! SCIP index it emits (docs/plan/2026-10-13-indexer-full-call-placement.md).
//!
//! The AST walk knows a call's FORM (`listener.local_addr().unwrap()`); the
//! compiler knows its MEANING (`core … Result<T, E>::unwrap`). An adapter that has
//! a compiler asks this table, by the call's span, which symbol the compiler
//! resolved there — and where that symbol is DEFINED, so a first-party target is
//! minted by the walk's own rules rather than translated.
//!
//! One reader, shared: rust-analyzer and the TypeScript compiler both emit SCIP.

use std::collections::HashMap;

use protobuf::Message;

use crate::indexer::facts::{Evidence, Fqn, Reason, RefKind, Reference, Resolution, Rung, Span};
use crate::indexer::fqn::{self, Origin, Reach};

/// One name the compiler resolved, at its position in a file.
///
/// `line` is 1-based, the convention every [`Span`] here uses; SCIP is 0-based,
/// and the conversion happens once, on the way in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Occurrence {
    pub line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
    pub symbol: String,
}

/// Where a symbol is declared: its file (relative to the index's root) and line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Definition {
    pub path: String,
    pub line: u32,
}

/// A compiler's answers for one project, by file.
#[derive(Debug)]
pub struct CompilerIndex {
    by_file: HashMap<String, Vec<Occurrence>>,
    /// Where each file names a LOCAL — kept apart, because a local is never a
    /// node, and a call to one is no edge at all.
    locals: HashMap<String, Vec<Occurrence>>,
    definitions: HashMap<String, Definition>,
}

/// What the compiler resolved at one call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer<'a> {
    Symbol(&'a str),
    /// A binding inside a body — a closure in a `let`. No node, so no edge.
    Local,
    Nothing,
}

/// SCIP's `Definition` role bit.
const DEFINITION: i32 = 1;

impl CompilerIndex {
    /// Read a serialized SCIP index. An unreadable one is an `Err` — never an
    /// empty table, which would read as "the compiler resolved nothing".
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let index = scip::types::Index::parse_from_bytes(bytes)
            .map_err(|e| format!("unreadable SCIP index: {e}"))?;
        Ok(Self::from_index(&index))
    }

    pub fn from_index(index: &scip::types::Index) -> Self {
        let mut out =
            Self { by_file: HashMap::new(), locals: HashMap::new(), definitions: HashMap::new() };
        for doc in &index.documents {
            let mut occurrences = Vec::with_capacity(doc.occurrences.len());
            let mut locals = Vec::new();
            for occ in &doc.occurrences {
                if occ.symbol.is_empty() {
                    continue;
                }
                // `local N` names a binding inside one body; it is never a node.
                if occ.symbol.starts_with("local ") {
                    locals.extend(occurrence(&occ.range, &occ.symbol));
                    continue;
                }
                let Some(o) = occurrence(&occ.range, &occ.symbol) else { continue };
                if occ.symbol_roles & DEFINITION != 0 {
                    out.definitions
                        .entry(occ.symbol.clone())
                        .or_insert(Definition { path: doc.relative_path.clone(), line: o.line });
                }
                occurrences.push(o);
            }
            out.by_file.insert(doc.relative_path.clone(), occurrences);
            out.locals.insert(doc.relative_path.clone(), locals);
        }
        out
    }

    /// Whether the compiler read this file at all — "no answer for the file" and
    /// "no symbol at this span" are different root causes.
    pub fn covers(&self, path: &str) -> bool {
        self.by_file.contains_key(path)
    }

    /// The symbol the compiler resolved for `name` inside `span` of `path`: the
    /// occurrence lying within the span whose source text is `name`. `lines` is
    /// the file's text split on `\n`. `None` when there is none.
    pub fn symbol_in<'a>(
        &'a self,
        path: &str,
        span: &Span,
        name: &str,
        lines: &[&str],
    ) -> Option<&'a str> {
        match self.answer_in(path, span, name, lines) {
            Answer::Symbol(symbol) => Some(symbol),
            Answer::Local | Answer::Nothing => None,
        }
    }

    /// What the compiler resolved for `name` inside `span`: the LAST matching
    /// occurrence, project symbol or local alike, by position.
    pub fn answer_in<'a>(
        &'a self,
        path: &str,
        span: &Span,
        name: &str,
        lines: &[&str],
    ) -> Answer<'a> {
        let named = |o: &&Occurrence| within(o, span) && text_of(o, lines) == Some(name);
        let symbol = self.by_file.get(path).and_then(|v| v.iter().rfind(named));
        let local = self.locals.get(path).and_then(|v| v.iter().rfind(named));
        let at = |o: &Occurrence| (o.line, o.start_col);
        match (symbol, local) {
            (Some(s), Some(l)) if at(l) > at(s) => Answer::Local,
            (Some(s), _) => Answer::Symbol(s.symbol.as_str()),
            (None, Some(_)) => Answer::Local,
            (None, None) => Answer::Nothing,
        }
    }

    /// Where `symbol` is declared, when the index holds its definition — the
    /// case for every symbol of the indexed project itself.
    pub fn definition_of(&self, symbol: &str) -> Option<&Definition> {
        self.definitions.get(symbol)
    }
}

/// A command the TASK LAYER runs for an adapter. The adapter only describes it:
/// an adapter never touches the disk (`no_adapter_reads_the_filesystem`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: String,
}

/// What the task layer answers for an adapter: the identity the walk mints for
/// the declaration named `name` at `line` of `path` (relative to the project
/// root the index was built from). It reads that file through its own adapter,
/// so the answer is the walk's by construction.
pub trait Declarations: Sync {
    fn declared_at(&self, path: &str, line: u32, name: &str) -> Result<Fqn, Reason>;
}

/// A language's compiler, as its adapter declares it. Everything here is pure;
/// the task layer runs the invocations and hands the results back.
pub trait CompilerEngine: Sync {
    /// The manifest file name a project is found from.
    fn manifest(&self) -> &'static str;
    /// The command that names the project root for a package directory.
    fn locate(&self, package_dir: &str) -> Invocation;
    /// The project root, read from `locate`'s output.
    fn project_root(&self, located: &str) -> Option<String>;
    /// The command that names the compiler's binary for a project — located,
    /// never assumed to be on `PATH` (toolchains install it in different places).
    fn locate_compiler(&self, project_root: &str) -> Invocation;
    /// The command that compiles a project into a SCIP index at `output`, with
    /// the binary `locate_compiler` named.
    fn compile(&self, compiler: &str, project_root: &str, output: &str) -> Invocation;
    /// The identity a resolved symbol names.
    fn target(
        &self,
        symbol: &str,
        index: &CompilerIndex,
        declarations: &dyn Declarations,
    ) -> Result<Fqn, Reason>;
}

/// What the task layer hands `index_file` for a file whose project it compiled:
/// the project's index, the file's path as that index records it (relative to
/// the project root), and the declaration lookup.
#[derive(Clone, Copy)]
pub struct Compiled<'a> {
    pub index: &'a CompilerIndex,
    pub path: &'a str,
    pub declarations: &'a dyn Declarations,
}

impl std::fmt::Debug for Compiled<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Compiled").field("path", &self.path).finish_non_exhaustive()
    }
}

/// An engine's [`CompilerEngine::target`] bound to one project's answers.
struct EngineTargets<'a> {
    engine: &'a dyn CompilerEngine,
    index: &'a CompilerIndex,
    declarations: &'a dyn Declarations,
}

impl Targets for EngineTargets<'_> {
    fn target(&self, symbol: &str) -> Result<Fqn, Reason> {
        self.engine.target(symbol, self.index, self.declarations)
    }
}

/// The compiler step for one file of a language that HAS an engine: every call
/// placed through it — or, when the task layer compiled nothing for this file's
/// project, every call [`Reason::EngineNotRun`].
pub fn place_with(
    engine: &dyn CompilerEngine,
    compiled: Option<&Compiled<'_>>,
    references: Vec<Reference>,
    text: &str,
) -> Vec<Reference> {
    match compiled {
        Some(c) => {
            let answers = FileAnswers::new(c.index, c.path, text);
            let targets = EngineTargets { engine, index: c.index, declarations: c.declarations };
            place_calls(references, Some(&answers), &targets)
        }
        None => {
            struct NotRun;
            impl Targets for NotRun {
                fn target(&self, _: &str) -> Result<Fqn, Reason> {
                    Err(Reason::EngineNotRun)
                }
            }
            place_calls(references, None, &NotRun)
        }
    }
}

/// How an adapter turns a compiler symbol into the identity this indexer mints.
/// The adapter's half of the compiler step — pure: it is handed what it needs.
pub trait Targets {
    fn target(&self, symbol: &str) -> Result<Fqn, Reason>;
}

/// The compiler's answers for ONE file: the project's index, the file's path as
/// the index knows it, and the file's text (an occurrence is matched by the
/// callee's name as written there).
pub struct FileAnswers<'a> {
    index: &'a CompilerIndex,
    path: &'a str,
    lines: Vec<&'a str>,
}

impl<'a> FileAnswers<'a> {
    pub fn new(index: &'a CompilerIndex, path: &'a str, text: &'a str) -> Self {
        Self { index, path, lines: text.split('\n').collect() }
    }
}

/// Place every CALL in a file through its language's compiler.
///
/// Where the compiler covers the file it is the ONE answer for every call — the
/// walk's own placement is not kept beside it, so a library symbol is never
/// spelled two ways. Where it does not, every call says so
/// ([`Reason::EngineNotRun`]): a visible root cause, never a fallback to the
/// walk. Other reference kinds are untouched.
pub fn place_calls(
    references: Vec<Reference>,
    answers: Option<&FileAnswers<'_>>,
    targets: &dyn Targets,
) -> Vec<Reference> {
    references
        .into_iter()
        .filter_map(|reference| {
            // What persistence writes as a `calls` edge: a call and a macro
            // invocation. The other kinds are `references` and `imports`.
            if !matches!(reference.kind, RefKind::Calls | RefKind::MacroInvokes) {
                return Some(reference);
            }
            let evidence = evidence_of(&reference);
            let target = match place_one(&reference.at, &evidence.name, answers, targets) {
                Placement::Target(fqn) => {
                    Resolution::Resolved { fqn, via: Rung::ResolvedByCompiler }
                }
                // A call to a local is no edge: locals get no nodes.
                Placement::Local => return None,
                Placement::Miss(reason) => Resolution::Unresolved { reason, evidence },
            };
            Some(Reference { target, ..reference })
        })
        .collect()
}

enum Placement {
    Target(Fqn),
    Local,
    Miss(Reason),
}

/// The name a call site's occurrence covers: the last segment of a path
/// (`Default::default` → `default`, `a.b` → `b`), without a macro's bang.
fn spelled(name: &str) -> &str {
    let last = name.rsplit("::").next().unwrap_or(name);
    let last = last.rsplit('.').next().unwrap_or(last);
    last.trim_end_matches('!')
}

fn place_one(
    at: &Span,
    name: &str,
    answers: Option<&FileAnswers<'_>>,
    targets: &dyn Targets,
) -> Placement {
    let Some(answers) = answers.filter(|a| a.index.covers(a.path)) else {
        return Placement::Miss(Reason::EngineNotRun);
    };
    match answers.index.answer_in(answers.path, at, spelled(name), &answers.lines) {
        Answer::Symbol(symbol) => match targets.target(symbol) {
            Ok(fqn) => Placement::Target(fqn),
            Err(reason) => Placement::Miss(reason),
        },
        Answer::Local => Placement::Local,
        Answer::Nothing => Placement::Miss(Reason::CompilerNoSymbol),
    }
}

/// What the walk knew about the call: its own evidence when it left the call
/// unplaced, or — when it placed it — the callee's name read back out of the
/// identity it placed it on.
fn evidence_of(reference: &Reference) -> Evidence {
    match &reference.target {
        Resolution::Unresolved { evidence, .. } => evidence.clone(),
        Resolution::Resolved { fqn, .. } => Evidence {
            name: callee_name(fqn),
            node_kind: "call".to_string(),
            reach: Reach::Item,
            saw: vec![crate::indexer::facts::Observation::Named(fqn.clone())],
        },
    }
}

/// The name a call site spells for a target the walk already placed: the last
/// segment of a local identity, the last path segment of a library member.
fn callee_name(target: &Fqn) -> String {
    let Ok(parsed) = fqn::parse(target.as_str()) else {
        return target.as_str().to_string();
    };
    let last = parsed.tail.last().copied().unwrap_or(parsed.package);
    match parsed.origin {
        Origin::Lib => last.rsplit("::").next().unwrap_or(last).trim_end_matches('!').to_string(),
        Origin::Local { .. } => last.to_string(),
    }
}

/// SCIP ranges are `[line, start, end]` on one line or
/// `[start_line, start, end_line, end]`, 0-based.
fn occurrence(range: &[i32], symbol: &str) -> Option<Occurrence> {
    let as_u32 = |v: i32| u32::try_from(v).ok();
    let (line, start, end_line, end) = match *range {
        [line, start, end] => (line, start, line, end),
        [line, start, end_line, end] => (line, start, end_line, end),
        _ => return None,
    };
    Some(Occurrence {
        line: as_u32(line)? + 1,
        start_col: as_u32(start)?,
        end_line: as_u32(end_line)? + 1,
        end_col: as_u32(end)?,
        symbol: symbol.to_string(),
    })
}

fn within(o: &Occurrence, span: &Span) -> bool {
    let starts_after =
        o.line > span.start_line || (o.line == span.start_line && o.start_col >= span.start_col);
    let ends_before =
        o.end_line < span.end_line || (o.end_line == span.end_line && o.end_col <= span.end_col);
    starts_after && ends_before
}

fn text_of<'t>(o: &Occurrence, lines: &[&'t str]) -> Option<&'t str> {
    if o.line != o.end_line {
        return None;
    }
    let line = lines.get(usize::try_from(o.line).ok()?.checked_sub(1)?)?;
    line.get(usize::try_from(o.start_col).ok()?..usize::try_from(o.end_col).ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn occ(range: Vec<i32>, symbol: &str, roles: i32) -> scip::types::Occurrence {
        let mut o = scip::types::Occurrence::new();
        o.range = range;
        o.symbol = symbol.to_string();
        o.symbol_roles = roles;
        o
    }

    const UNWRAP: &str = "rust-analyzer cargo core 1.0 result/impl#[`Result<T, E>`]unwrap().";
    const LOCAL_ADDR: &str = "rust-analyzer cargo tokio 1.52 net/impl#[TcpListener]local_addr().";
    const BIND: &str = "rust-analyzer cargo demo 0.1 net/bind().";

    /// `let addr = listener.local_addr().unwrap();` on line 3 (0-based 2), and
    /// `bind` declared in another file of the project.
    fn index() -> scip::types::Index {
        let mut uses = scip::types::Document::new();
        uses.relative_path = "src/main.rs".into();
        uses.occurrences = vec![
            occ(vec![2, 4, 8], "local 1", DEFINITION),
            occ(vec![2, 24, 34], LOCAL_ADDR, 0),
            occ(vec![2, 37, 43], UNWRAP, 0),
            occ(vec![4, 4, 8], BIND, 0),
        ];
        let mut decl = scip::types::Document::new();
        decl.relative_path = "src/net.rs".into();
        decl.occurrences = vec![occ(vec![9, 7, 11], BIND, DEFINITION)];
        let mut index = scip::types::Index::new();
        index.documents = vec![uses, decl];
        index
    }

    const MAIN: &str = "fn main() {\n    let listener = open();\n    let addr = listener.local_addr().unwrap();\n\n    bind();\n}";

    fn read() -> CompilerIndex {
        let bytes = index().write_to_bytes().expect("serialises");
        CompilerIndex::from_bytes(&bytes).expect("reads")
    }

    #[test]
    fn a_call_site_is_answered_by_the_occurrence_inside_its_span_with_its_name() {
        let ix = read();
        let lines: Vec<&str> = MAIN.split('\n').collect();
        // `listener.local_addr().unwrap()` — the call span is the whole expression.
        let span = Span { start_line: 3, start_col: 15, end_line: 3, end_col: 45 };
        assert_eq!(ix.symbol_in("src/main.rs", &span, "unwrap", &lines), Some(UNWRAP));
        assert_eq!(ix.symbol_in("src/main.rs", &span, "local_addr", &lines), Some(LOCAL_ADDR));
    }

    #[test]
    fn a_name_outside_the_span_or_spelled_differently_is_not_an_answer() {
        let ix = read();
        let lines: Vec<&str> = MAIN.split('\n').collect();
        let elsewhere = Span { start_line: 5, start_col: 0, end_line: 5, end_col: 20 };
        assert_eq!(ix.symbol_in("src/main.rs", &elsewhere, "unwrap", &lines), None);
        let span = Span { start_line: 3, start_col: 15, end_line: 3, end_col: 45 };
        assert_eq!(ix.symbol_in("src/main.rs", &span, "expect", &lines), None);
    }

    #[test]
    fn locals_are_not_occurrences_and_definitions_are_kept_by_symbol() {
        let ix = read();
        assert!(ix.by_file["src/main.rs"].iter().all(|o| !o.symbol.starts_with("local ")));
        assert_eq!(
            ix.definition_of(BIND),
            Some(&Definition { path: "src/net.rs".into(), line: 10 })
        );
        assert_eq!(ix.definition_of(UNWRAP), None, "a library symbol has no definition here");
    }

    #[test]
    fn coverage_says_which_files_the_compiler_read() {
        let ix = read();
        assert!(ix.covers("src/main.rs"));
        assert!(!ix.covers("src/never_compiled.rs"));
    }

    #[test]
    fn an_unreadable_index_is_an_error_not_an_empty_answer() {
        assert!(CompilerIndex::from_bytes(b"\xff\xff not protobuf").is_err());
    }
}

#[cfg(test)]
mod placement_tests {
    use super::*;
    use crate::indexer::facts::{Evidence, Fqn, Language, RefKind, Reference, Resolution, Rung};
    use crate::indexer::fqn::{self, Form, Reach};

    fn item(name: &str) -> Fqn {
        fqn::refer(&Form::Item {
            lang: Language::Rust,
            package: "demo",
            module: "net",
            name,
            reach: Reach::Item,
        })
        .expect("mints")
    }

    fn call(at: Span, target: Resolution) -> Reference {
        Reference { from: item("main"), kind: RefKind::Calls, at, target }
    }

    fn unplaced(name: &str) -> Resolution {
        Resolution::Unresolved {
            reason: Reason::ReceiverTypeUnknown,
            evidence: Evidence {
                name: name.into(),
                node_kind: "field_expression".into(),
                reach: Reach::Item,
                saw: vec![],
            },
        }
    }

    const BIND: &str = "rust-analyzer cargo demo 0.1 net/bind().";
    const UNWRAP: &str = "rust-analyzer cargo core 1.0 result/impl#[`Result<T, E>`]unwrap().";
    const TEXT: &str = "fn main() {\n    let x = open().unwrap();\n    bind();\n}";

    /// A stand-in for an adapter: `bind` is ours, `unwrap` a library, and
    /// anything else is a symbol it cannot spell.
    struct Engine;
    impl Targets for Engine {
        fn target(&self, symbol: &str) -> Result<Fqn, Reason> {
            match symbol {
                BIND => Ok(item("bind")),
                UNWRAP => {
                    fqn::refer(&Form::Lib { package: "core", member: "result::Result::unwrap" })
                        .map_err(|_| Reason::SymbolUnnamed)
                }
                _ => Err(Reason::SymbolUnnamed),
            }
        }
    }

    fn compiled() -> CompilerIndex {
        let mut doc = scip::types::Document::new();
        doc.relative_path = "src/main.rs".into();
        let mut a = scip::types::Occurrence::new();
        a.range = vec![1, 19, 25];
        a.symbol = UNWRAP.into();
        let mut b = scip::types::Occurrence::new();
        b.range = vec![2, 4, 8];
        b.symbol = BIND.into();
        doc.occurrences = vec![a, b];
        let mut index = scip::types::Index::new();
        index.documents = vec![doc];
        CompilerIndex::from_index(&index)
    }

    fn span(line: u32, start: u32, end: u32) -> Span {
        Span { start_line: line, start_col: start, end_line: line, end_col: end }
    }

    #[test]
    fn the_compiler_places_every_call_in_a_file_it_covers() {
        let ix = compiled();
        let answers = FileAnswers::new(&ix, "src/main.rs", TEXT);
        let refs = vec![
            call(span(2, 12, 27), unplaced("unwrap")),
            // Placed by the walk already; the compiler is still the one answer.
            call(
                span(3, 4, 10),
                Resolution::Resolved { fqn: item("bind"), via: Rung::DeclaredHere },
            ),
        ];
        let placed = place_calls(refs, Some(&answers), &Engine);
        assert_eq!(
            placed[0].target,
            Resolution::Resolved {
                fqn: fqn::refer(&Form::Lib { package: "core", member: "result::Result::unwrap" })
                    .unwrap(),
                via: Rung::ResolvedByCompiler
            }
        );
        assert_eq!(
            placed[1].target,
            Resolution::Resolved { fqn: item("bind"), via: Rung::ResolvedByCompiler }
        );
    }

    #[test]
    fn a_file_the_compiler_did_not_cover_says_so_on_every_call() {
        let refs = vec![call(
            span(3, 4, 10),
            Resolution::Resolved { fqn: item("bind"), via: Rung::DeclaredHere },
        )];
        let placed = place_calls(refs, None, &Engine);
        assert!(
            matches!(&placed[0].target, Resolution::Unresolved { reason: Reason::EngineNotRun, evidence } if evidence.name == "bind"),
            "got {:?}",
            placed[0].target
        );
    }

    #[test]
    fn a_file_missing_from_the_compilers_index_is_not_run_rather_than_no_symbol() {
        // The project was compiled, but not this file — a different root cause
        // from "compiled it and found nothing here", with a different fix.
        let ix = compiled();
        let answers = FileAnswers::new(&ix, "src/elsewhere.rs", TEXT);
        let placed =
            place_calls(vec![call(span(3, 4, 10), unplaced("bind"))], Some(&answers), &Engine);
        assert!(matches!(
            placed[0].target,
            Resolution::Unresolved { reason: Reason::EngineNotRun, .. }
        ));
    }

    #[test]
    fn no_symbol_at_the_span_and_an_unspellable_symbol_are_named_root_causes() {
        let ix = compiled();
        let answers = FileAnswers::new(&ix, "src/main.rs", TEXT);
        let refs = vec![call(span(2, 12, 27), unplaced("expect"))];
        let placed = place_calls(refs, Some(&answers), &Engine);
        assert!(matches!(
            placed[0].target,
            Resolution::Unresolved { reason: Reason::CompilerNoSymbol, .. }
        ));

        struct Nothing;
        impl Targets for Nothing {
            fn target(&self, _: &str) -> Result<Fqn, Reason> {
                Err(Reason::DeclarationNotFound)
            }
        }
        let refs = vec![call(span(3, 4, 10), unplaced("bind"))];
        let placed = place_calls(refs, Some(&answers), &Nothing);
        assert!(matches!(
            placed[0].target,
            Resolution::Unresolved { reason: Reason::DeclarationNotFound, .. }
        ));
    }

    /// A macro invocation is persisted as a `calls` edge, so it is placed by
    /// the compiler like any other call.
    #[test]
    fn a_macro_invocation_is_a_call_the_compiler_places() {
        let ix = compiled();
        let answers = FileAnswers::new(&ix, "src/main.rs", TEXT);
        let mut invoke = call(span(3, 4, 10), unplaced("bind"));
        invoke.kind = RefKind::MacroInvokes;
        let placed = place_calls(vec![invoke], Some(&answers), &Engine);
        assert_eq!(
            placed[0].target,
            Resolution::Resolved { fqn: item("bind"), via: Rung::ResolvedByCompiler }
        );
    }

    /// A path call (`net::bind()`, `Default::default()`) is matched on its
    /// LAST segment — that is the text the compiler's occurrence covers.
    #[test]
    fn a_path_call_is_matched_on_its_last_segment() {
        let ix = compiled();
        let answers = FileAnswers::new(&ix, "src/main.rs", TEXT);
        let placed =
            place_calls(vec![call(span(3, 4, 10), unplaced("net::bind"))], Some(&answers), &Engine);
        assert_eq!(
            placed[0].target,
            Resolution::Resolved { fqn: item("bind"), via: Rung::ResolvedByCompiler }
        );
    }

    /// A call to a LOCAL — a closure bound with `let` — is no edge at all:
    /// locals get neither nodes nor edges, and the compiler says which callee is
    /// one.
    #[test]
    fn a_call_to_a_local_is_dropped_not_left_unplaced() {
        let text = "fn main() {\n    let edit = |x: u8| x;\n    edit(1);\n}";
        let mut doc = scip::types::Document::new();
        doc.relative_path = "src/main.rs".into();
        let mut def = scip::types::Occurrence::new();
        def.range = vec![1, 8, 12];
        def.symbol = "local 3".into();
        def.symbol_roles = 1;
        let mut used = scip::types::Occurrence::new();
        used.range = vec![2, 4, 8];
        used.symbol = "local 3".into();
        doc.occurrences = vec![def, used];
        let mut index = scip::types::Index::new();
        index.documents = vec![doc];
        let ix = CompilerIndex::from_index(&index);
        let answers = FileAnswers::new(&ix, "src/main.rs", text);
        let placed =
            place_calls(vec![call(span(3, 4, 11), unplaced("edit"))], Some(&answers), &Engine);
        assert!(placed.is_empty(), "a call to a local leaves no edge, got {placed:?}");
    }

    #[test]
    fn only_calls_are_touched() {
        let ix = compiled();
        let answers = FileAnswers::new(&ix, "src/main.rs", TEXT);
        let mut read = call(span(3, 4, 10), unplaced("bind"));
        read.kind = RefKind::Reads;
        let placed = place_calls(vec![read.clone()], Some(&answers), &Engine);
        assert_eq!(placed[0], read);
    }
}
