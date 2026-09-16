//! Reading the JavaScript and TypeScript grammars
//! (`docs/spec/indexer/04b-walk-js.md`).
//!
//! Two adapters, one reader, one [`Language`] — see the note on
//! [`Language::TypeScript`] for why `.js` and `.ts` file their symbols under one
//! label. `.svelte` joins them through `super::svelte`, which extracts a script
//! block and hands it here.
//!
//! # The one structural difference from Rust (04b §1)
//!
//! **A Rust binding is lexical. A JavaScript binding is flow-sensitive.**
//!
//! ```js
//! let x = new Foo();
//! x.method();        // Foo
//! x = makeBar();
//! x.method();        // NOT Foo — and nothing lexical says so
//! ```
//!
//! Rust's `let` SHADOWS: a second `let x` introduces a new binding, and a
//! lexically-scoped map is exactly right. JavaScript's `x = …` REASSIGNS the
//! same binding, so the type at a use site is whatever the most recent
//! assignment before it said — a property of position in the statement list,
//! not of scope. That is why [`Flow`] below is threaded through the walk in
//! statement order instead of being cloned per block the way the Rust walk's
//! `Scope::bindings` is, and it is the whole of why that map could not be
//! reused.
//!
//! # What this reader does NOT attempt
//!
//! The two biggest measured routes — `import { x }` (848 of 6,406 member calls)
//! and "not bound in this file" (1,887) — are the same question asked twice:
//! the type lives in another module. That is the cross-file lookup and it waits
//! on issue #174 (04b S4). Everything here is answerable from ONE file.
//
// These modules have no caller on purpose — see the note in `indexer/mod.rs`.
#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};

use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span as OxcSpan};

use super::common::{Miss, considered};
use super::{Home, LanguageAdapter, ReadError, Source, TypeHomes};
use crate::indexer::facts::{
    Binding, DeclaredType, FileFacts, Fqn, Import, ImportOrigin, Language, Observation, Param,
    Reason, RefKind, Reference, Relation, RelationKind, Resolution, Rung, Span, Symbol, SymbolKind,
    Visibility,
};
use crate::indexer::fqn::{self, Form, FqnError, Reach, SEPARATOR};
use crate::indexer::resolve::{Grammar, Root};

/// TypeScript: the dialect that STATES types, and the canonical adapter for
/// [`Language::TypeScript`] — the identity rules the other two are checked
/// against live here.
pub struct TypeScriptAdapter;

/// JavaScript. Same reader, same identity, and no annotations anywhere in the
/// source, which is why 04b S3 puts `new T()` at the head of the route list: it
/// is the only in-file route that works here at all.
pub struct JavaScriptAdapter;

impl LanguageAdapter for TypeScriptAdapter {
    fn language(&self) -> Language {
        Language::TypeScript
    }

    fn name(&self) -> &'static str {
        "typescript"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &[".ts", ".tsx", ".cts", ".mts"]
    }

    fn grammar(&self) -> &'static Grammar {
        &GRAMMAR
    }

    fn read(&self, source: &Source<'_>, types: &TypeHomes) -> Result<FileFacts, ReadError> {
        read(source, types)
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

impl LanguageAdapter for JavaScriptAdapter {
    fn language(&self) -> Language {
        Language::TypeScript
    }

    fn name(&self) -> &'static str {
        "javascript"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &[".js", ".jsx", ".mjs", ".cjs"]
    }

    fn grammar(&self) -> &'static Grammar {
        &GRAMMAR
    }

    fn read(&self, source: &Source<'_>, types: &TypeHomes) -> Result<FileFacts, ReadError> {
        read(source, types)
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

/// What the shared resolution ladder needs to know about this language, and
/// nothing more (R7).
pub const GRAMMAR: Grammar = Grammar {
    language: Language::TypeScript,
    // A use-site path is DOTTED (`ns.thing`) and a module path is SLASHED
    // (`./lib/store`). Rust spells both `::`; nothing else does.
    path_separator: ".",
    module_separator: "/",
    // No package root: JavaScript has no `crate`. A bare specifier head IS a
    // package name, which `import_origin` below already reads off the
    // specifier — so listing a placeholder root here would make some real
    // directory called `crate` root a path by accident.
    roots: &[(".", Root::Here), ("..", Root::Up)],
    // A specifier is a FILE path, so its last segment carries an extension the
    // declaration side already dropped. The SAME function `module_path` uses.
    module_segment,
    relative_to_directory: true,
    // A JavaScript module reaches an external ONLY through an import; `a.b` is
    // a property access, not a package path.
    paths_name_packages: false,
    // The binding is stated in the import CLAUSE (`import { a as b }`), not in
    // the specifier string, so there is nothing in the path for the ladder to
    // split off.
    names_the_binding: None,
    // `export *` re-exports from a module; it is not a suffix on a specifier.
    // The walk records it as `Binding::Glob` directly.
    wildcard: None,
    // TypeScript lints types into PascalCase and values into camelCase, the
    // same signal Rust's casing gives. A misread here cannot produce a WRONG
    // edge, only a dangling one (see `Grammar::names_a_type`).
    names_a_type: |segment| segment.starts_with(|c: char| c.is_uppercase()),
    prelude: PRELUDE,
    plumbing: PLUMBING,
};

/// The names in scope with nothing written to bring them there.
///
/// Named `ecmascript` rather than a package, because that is what they are:
/// properties of the global object the language specifies, plus the handful the
/// host environment always supplies. They are members of something we never
/// open, like any other external (R5).
const PRELUDE: &[(&str, &str, &str)] = &[
    ("Object", "ecmascript", "Object"),
    ("Array", "ecmascript", "Array"),
    ("String", "ecmascript", "String"),
    ("Number", "ecmascript", "Number"),
    ("Boolean", "ecmascript", "Boolean"),
    ("Symbol", "ecmascript", "Symbol"),
    ("BigInt", "ecmascript", "BigInt"),
    ("Math", "ecmascript", "Math"),
    ("JSON", "ecmascript", "JSON"),
    ("Date", "ecmascript", "Date"),
    ("RegExp", "ecmascript", "RegExp"),
    ("Error", "ecmascript", "Error"),
    ("TypeError", "ecmascript", "TypeError"),
    ("RangeError", "ecmascript", "RangeError"),
    ("SyntaxError", "ecmascript", "SyntaxError"),
    ("Promise", "ecmascript", "Promise"),
    ("Map", "ecmascript", "Map"),
    ("Set", "ecmascript", "Set"),
    ("WeakMap", "ecmascript", "WeakMap"),
    ("WeakSet", "ecmascript", "WeakSet"),
    ("Proxy", "ecmascript", "Proxy"),
    ("Reflect", "ecmascript", "Reflect"),
    ("Intl", "ecmascript", "Intl"),
    ("globalThis", "ecmascript", "globalThis"),
    ("console", "ecmascript", "console"),
    ("fetch", "ecmascript", "fetch"),
    ("URL", "ecmascript", "URL"),
    ("URLSearchParams", "ecmascript", "URLSearchParams"),
    ("Request", "ecmascript", "Request"),
    ("Response", "ecmascript", "Response"),
    ("Headers", "ecmascript", "Headers"),
    ("AbortController", "ecmascript", "AbortController"),
    ("setTimeout", "ecmascript", "setTimeout"),
    ("clearTimeout", "ecmascript", "clearTimeout"),
    ("setInterval", "ecmascript", "setInterval"),
    ("clearInterval", "ecmascript", "clearInterval"),
    ("queueMicrotask", "ecmascript", "queueMicrotask"),
    ("structuredClone", "ecmascript", "structuredClone"),
    ("parseInt", "ecmascript", "parseInt"),
    ("parseFloat", "ecmascript", "parseFloat"),
    ("isNaN", "ecmascript", "isNaN"),
    ("encodeURIComponent", "ecmascript", "encodeURIComponent"),
    ("decodeURIComponent", "ecmascript", "decodeURIComponent"),
    // SVELTE 5 RUNES. In scope with nothing written, exactly like a prelude
    // name, and belonging to `svelte` rather than to the language — which is
    // why a prelude entry carries its own package. MEASURED: `$derived` 466,
    // `$state` 316, `$props` 256 sat in `NoImportInScope` because nothing
    // could name them.
    ("$state", "svelte", "$state"),
    ("$derived", "svelte", "$derived"),
    ("$props", "svelte", "$props"),
    ("$effect", "svelte", "$effect"),
    ("$bindable", "svelte", "$bindable"),
    ("$inspect", "svelte", "$inspect"),
    ("$host", "svelte", "$host"),
];

/// Members every value has, from `Object.prototype` or from the two prototypes
/// almost every expression passes through.
///
/// Filtering, not failure — its own reason so a reader can exclude it without
/// also excluding genuine misses, and only ever applied to a reference the
/// ladder has already failed to place, so a provable edge is never dropped.
///
/// `map` and `filter` are on the list ADVISEDLY: they are `Array.prototype`
/// members thousands of times over, and a first-party `map` method that the
/// ladder DID place never reaches this list at all.
const PLUMBING: &[&str] = &[
    "toString",
    "valueOf",
    "hasOwnProperty",
    "then",
    "catch",
    "finally",
    "map",
    "filter",
    "forEach",
    "reduce",
    "find",
    "findIndex",
    "some",
    "every",
    "flat",
    "flatMap",
    "includes",
    "indexOf",
    "join",
    "slice",
    "splice",
    "concat",
    "push",
    "pop",
    "shift",
    "unshift",
    "sort",
    "reverse",
    "keys",
    "values",
    "entries",
    "length",
    "trim",
    "split",
    "replace",
    "startsWith",
    "endsWith",
    "toLowerCase",
    "toUpperCase",
    "padStart",
    "padEnd",
    "bind",
    "call",
    "apply",
];

/// Parse one file once (R1) and return everything that parse saw (spec §3).
pub fn read(source: &Source<'_>, types: &TypeHomes) -> Result<FileFacts, ReadError> {
    let from =
        file_fqn(source.package, source.module, source.path).map_err(ReadError::NoFileIdentity)?;
    let found = read_file(source, types, from)?;
    Ok(FileFacts {
        language: Language::TypeScript,
        package: source.package.to_string(),
        module: source.module.to_string(),
        path: source.path.to_string(),
        symbols: found.symbols,
        references: found.references,
        relations: found.relations,
        imports: found.imports,
    })
}

/// Everything one walk produced. Not a [`FileFacts`]: that carries the file's
/// package, module and path, which are identity and are stated by the caller
/// rather than read out of the source.
pub(super) struct Found {
    pub symbols: Vec<Symbol>,
    pub references: Vec<Reference>,
    pub relations: Vec<Relation>,
    pub imports: Vec<Import>,
}

impl Found {
    pub(super) fn empty() -> Self {
        Self {
            symbols: Vec::new(),
            references: Vec::new(),
            relations: Vec::new(),
            imports: Vec::new(),
        }
    }

    pub(super) fn absorb(&mut self, other: Found) {
        self.symbols.extend(other.symbols);
        self.references.extend(other.references);
        self.relations.extend(other.relations);
        self.imports.extend(other.imports);
    }
}

/// Read a whole `.js`/`.ts` file. The Svelte path is [`read_component`], which
/// has several blocks and markup to thread one walk through.
fn read_file(source: &Source<'_>, types: &TypeHomes, from: Fqn) -> Result<Found, ReadError> {
    let (text, offset, syntax_path) = (source.text, 0u32, source.path);
    let source_type = SourceType::from_path(syntax_path)
        .map_err(|e| ReadError::GrammarUnavailable(e.to_string()))?;
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, text, source_type).parse();
    // `panicked` means oxc produced no usable tree — distinct from a tree with
    // recovered errors in it, which is a normal thing to walk (R10.3).
    if parsed.panicked {
        return Err(ReadError::NotParsed);
    }

    let lines = LineIndex::of(source.text);
    let mut walk = Walk {
        package: source.package,
        module: source.module,
        text,
        offset,
        lines: &lines,
        types,
        declared_here: types_declared_in(&parsed.program.body).into_iter().collect(),
        namespaces: BTreeSet::new(),
        reassigned: assigned_in(&parsed.program.body).into_iter().collect(),
        found: Found::empty(),
    };
    let scope = Scope { from, container: Container::File, fn_scope: Vec::new() };
    let mut flow = Flow::empty();
    // No hoisting pass, and that is not an omission: a `new Widget()` types its
    // binding from the NAME it writes, so nothing here has to have seen
    // `class Widget` first. What does need the whole file — placing that name —
    // is the ladder's, and it gets the complete symbol list.
    for statement in &parsed.program.body {
        walk.statement(statement, &scope, &mut flow);
    }
    Ok(walk.found)
}

/// Read a whole Svelte component: every `<script>` block, then the markup
/// around them, through ONE walk.
///
/// One walk and not one per block, deliberately. The markup's `{store.load()}`
/// has to see the binding the script's `const store = new Store()` made, and a
/// namespace import in the module block has to still be a namespace in the
/// instance block. Two walks would each start from an empty [`Flow`] and the
/// markup would type nothing.
pub(super) fn read_component<'a>(
    source: &Source<'a>,
    types: &TypeHomes,
    blocks: &[super::svelte::ScriptBlock],
    from: Fqn,
) -> Result<Found, ReadError> {
    let lines = LineIndex::of(source.text);
    let allocator = Allocator::default();

    // Every block is PARSED before any is walked, because `reassigned` is a
    // property of the whole component: a module block's `const` that the
    // instance block never touches is still safe to carry into a closure, and
    // one the instance block reassigns is not.
    let mut parsed_blocks = Vec::with_capacity(blocks.len());
    for block in blocks {
        let text = &source.text[block.start as usize..block.end as usize];
        // The dialect is what the block STATES. Reading a plain `<script>` as
        // TypeScript would accept syntax the file is not written in; reading a
        // `lang="ts"` block as JavaScript would fail on every annotation.
        let syntax = if block.typescript { "block.ts" } else { "block.js" };
        let source_type = SourceType::from_path(syntax)
            .map_err(|e| ReadError::GrammarUnavailable(e.to_string()))?;
        let parsed = Parser::new(&allocator, text, source_type).parse();
        if parsed.panicked {
            return Err(ReadError::NotParsed);
        }
        parsed_blocks.push((block, text, parsed));
    }
    let reassigned =
        parsed_blocks.iter().flat_map(|(_, _, parsed)| assigned_in(&parsed.program.body)).collect();
    // And for the same reason the types a component declares are the whole
    // component's: a class in the module block is the home of its members
    // however many instance blocks read them.
    let declared_here = parsed_blocks
        .iter()
        .flat_map(|(_, _, parsed)| types_declared_in(&parsed.program.body))
        .collect();

    let mut walk = Walk {
        package: source.package,
        module: source.module,
        text: source.text,
        offset: 0,
        lines: &lines,
        types,
        declared_here,
        namespaces: BTreeSet::new(),
        reassigned,
        found: Found::empty(),
    };
    let scope = Scope { from, container: Container::File, fn_scope: Vec::new() };
    let mut flow = Flow::empty();

    for (block, text, parsed) in &parsed_blocks {
        // The walk reports spans against the WHOLE file, so a line number here
        // is one a reader can click.
        walk.text = text;
        walk.offset = block.start;
        for statement in &parsed.program.body {
            walk.statement(statement, &scope, &mut flow);
        }
    }

    let typescript = blocks.iter().any(|b| b.typescript);
    walk.markup(source.text, blocks, typescript, &scope, &mut flow)?;
    Ok(walk.found)
}

impl<'a> Walk<'a> {
    /// Every markup interpolation, as a use site (04b §4).
    fn markup(
        &mut self,
        text: &'a str,
        blocks: &[super::svelte::ScriptBlock],
        typescript: bool,
        scope: &Scope,
        flow: &mut Flow,
    ) -> Result<(), ReadError> {
        let allocator = Allocator::default();
        let syntax = if typescript { "markup.ts" } else { "markup.js" };
        let source_type = SourceType::from_path(syntax)
            .map_err(|e| ReadError::GrammarUnavailable(e.to_string()))?;

        for region in interpolations(text, blocks) {
            let raw = &text[region.start as usize..region.end as usize];
            let Some(expression) = markup_expression(raw) else {
                continue;
            };
            let offset = region.start + expression.offset;
            let parsed = Parser::new(&allocator, expression.text, source_type).parse();
            let statement = parsed.program.body.first();
            let Some(Statement::ExpressionStatement(statement)) = statement else {
                // A tag whose contents will not parse as an expression is NAMED
                // rather than dropped: the histogram says what was not
                // understood (R2, S8).
                let miss = Miss::unhandled("SvelteTag", raw.trim(), Reach::Item);
                self.found.references.push(Reference {
                    from: scope.from.clone(),
                    kind: RefKind::Reads,
                    at: self.lines.locate(region.start, region.end),
                    target: miss.resolution(),
                });
                continue;
            };
            // `{#each items as item}` binds `item` for the block that follows.
            // Its type would be the element type of `items`, which nothing here
            // states — so it is CLEARED, not guessed (the same refusal
            // `for_of` makes).
            for name in &expression.binds {
                flow.bind(name, None);
            }
            self.text = expression.text;
            self.offset = offset;
            self.expression(&statement.expression, scope, flow);
        }
        Ok(())
    }
}

/// A `{ … }` region of markup, as a byte range in the whole file.
struct Interpolation {
    start: u32,
    end: u32,
}

/// Every interpolation OUTSIDE a script or style block.
///
/// Brace-counting, and string- and comment-aware, because `{ '{' }` and
/// `{ a ? "}" : "" }` are both real and both would end the region early
/// otherwise. It reads less than a Svelte parser would and never more.
fn interpolations(text: &str, blocks: &[super::svelte::ScriptBlock]) -> Vec<Interpolation> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let skip: Vec<(usize, usize)> = blocks
        .iter()
        .map(|b| (b.start as usize, b.end as usize))
        .chain(style_blocks(text))
        .collect();
    let mut i = 0usize;
    while i < bytes.len() {
        if let Some((_, end)) = skip.iter().find(|(s, e)| i >= *s && i < *e) {
            i = *end;
            continue;
        }
        if bytes[i] != b'{' {
            i += 1;
            continue;
        }
        let start = i;
        let mut depth = 0usize;
        let mut quote: Option<u8> = None;
        while i < bytes.len() {
            let c = bytes[i];
            match quote {
                Some(q) => {
                    if c == b'\\' {
                        i += 1;
                    } else if c == q {
                        quote = None;
                    }
                }
                None => match c {
                    b'\'' | b'"' | b'`' => quote = Some(c),
                    b'{' => depth += 1,
                    b'}' => {
                        depth -= 1;
                        if depth == 0 {
                            i += 1;
                            break;
                        }
                    }
                    _ => {}
                },
            }
            i += 1;
        }
        if depth == 0 && i > start + 1 {
            out.push(Interpolation { start: start as u32, end: i as u32 });
        }
    }
    out
}

/// `<style>` block CONTENT ranges — CSS, which declares nothing this graph
/// names and whose braces are not interpolations.
fn style_blocks(text: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut at = 0usize;
    while let Some(open) = text[at..].find("<style").map(|i| i + at) {
        let Some(tag_end) = text[open..].find('>').map(|i| i + open) else {
            break;
        };
        let Some(close) = text[tag_end..].find("</style").map(|i| i + tag_end) else {
            break;
        };
        out.push((open, close));
        at = close + "</style".len();
    }
    out
}

/// What a `{ … }` region contains, once the Svelte tag around it is read away.
struct MarkupExpression<'a> {
    text: &'a str,
    /// Where `text` starts inside the region, so a span still points at the
    /// right column.
    offset: u32,
    /// Names the tag BINDS for the block that follows — `{#each xs as x}`.
    binds: Vec<String>,
}

/// The expression a markup region states, or `None` when it states none.
///
/// The decision 04b §4 asked for, written out: a closing tag and a bare
/// `{:else}` NAME NOTHING and emit nothing, which is different from a tag this
/// reader does not understand.
fn markup_expression(raw: &str) -> Option<MarkupExpression<'_>> {
    let inner_start = 1u32;
    let inner = raw.strip_prefix('{')?.strip_suffix('}')?;
    let trimmed = inner.trim_start();
    let lead = inner_start + (inner.len() - trimmed.len()) as u32;

    // A closing tag, and `{:else}` with no condition. Neither names anything.
    if trimmed.starts_with('/') || trimmed == ":else" || trimmed.trim() == ":then" {
        return None;
    }
    // A comment.
    if trimmed.starts_with("<!--") {
        return None;
    }

    for (tag, keeps_binding) in [
        ("#if ", false),
        (":else if ", false),
        ("#each ", true),
        ("#await ", false),
        (":then ", false),
        (":catch ", false),
        ("#key ", false),
        ("@html ", false),
        ("@debug ", false),
        ("@render ", false),
    ] {
        let Some(rest) = trimmed.strip_prefix(tag) else {
            continue;
        };
        let offset = lead + tag.len() as u32;
        if !keeps_binding {
            return Some(MarkupExpression { text: rest, offset, binds: Vec::new() });
        }
        // `{#each expr as name (key)}` — the EXPRESSION is `expr` and `name` is
        // bound for the block. Split at the ` as ` the grammar requires.
        let (expression, tail) = match rest.split_once(" as ") {
            Some((expression, tail)) => (expression, tail),
            None => (rest, ""),
        };
        let binds = tail
            .split(&[',', '(', ')', '[', ']', '{', '}'][..])
            .map(str::trim)
            .filter(|s| {
                !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '$')
            })
            .map(str::to_string)
            .collect();
        return Some(MarkupExpression { text: expression, offset, binds });
    }
    // `{@const x = expr}` declares a name in markup. The declaration is not a
    // symbol — it is scoped to a block this reader does not model — but the
    // expression is still a use site.
    if let Some(rest) = trimmed.strip_prefix("@const ")
        && let Some((_, expression)) = rest.split_once('=')
    {
        let offset = lead + (trimmed.len() - expression.len()) as u32;
        return Some(MarkupExpression { text: expression, offset, binds: Vec::new() });
    }
    // Anything else with a sigil is a tag this reader has no rule for; the
    // caller names it in the histogram rather than dropping it.
    if trimmed.starts_with(['#', ':', '@']) {
        return Some(MarkupExpression { text: trimmed, offset: lead, binds: Vec::new() });
    }
    if trimmed.trim().is_empty() {
        return None;
    }
    Some(MarkupExpression { text: trimmed, offset: lead, binds: Vec::new() })
}

// ── identity (spec §2) ───────────────────────────────────────────────────────

/// The identity of the file itself, which is the identity of the module it
/// declares.
///
/// The last path segment is the name and everything before it is the module,
/// the same split [`super::rust::file_fqn`] makes — and the same [`Reach::Mod`],
/// so a file and a reference to it are one node.
///
/// No package-root special case, and that is a consequence of [`module_path`]
/// dropping NOTHING but the extension: every file's module path is non-empty,
/// so there is no file whose identity has to be recovered from its stem.
pub fn file_fqn(package: &str, module: &str, _path: &str) -> Result<Fqn, FqnError> {
    let (parent, name) = match module.rsplit_once('/') {
        Some((parent, name)) => (parent, name),
        None => ("", module),
    };
    if name.is_empty() {
        return Err(FqnError::EmptySegment { segment: fqn::Segment::Member });
    }
    fqn::define(&Form::Item {
        lang: Language::TypeScript,
        package,
        module: parent,
        name,
        reach: MODULE,
    })
}

/// The reach every module declaration is minted under — the file's own
/// identity and a TypeScript `namespace`. A constant so the two producers
/// cannot be changed apart.
const MODULE: Reach = Reach::Mod;

/// A file's package-relative MODULE PATH, from its path alone.
///
/// Relative to `<package_root>/src` (or the package root itself), with the
/// extension dropped and **nothing else**:
///
/// | file | module |
/// |---|---|
/// | `src/lib/store.ts` | `lib/store` |
/// | `src/index.ts` | `index` |
/// | `src/lib/store/index.ts` | `lib/store/index` |
/// | `routes/+page.svelte` | `routes/+page` |
///
/// **The trailing `index` is deliberately KEPT, and it is a trade rather than
/// an oversight.** Dropping it would mirror Rust's `mod.rs` rule and it would
/// collide: `src/index.ts` and `src/index/index.ts` would both reduce to
/// `index`, and two files claiming one identity is the failure spec §2 exists
/// to prevent. Keeping it costs a MISS instead — `import x from './a'`, which
/// Node resolves to `a/index.ts`, mints `a` and matches nothing — and a miss is
/// what R4 asks us to prefer. Closing it needs the resolver to try both
/// spellings, which is the cross-file lookup deferred to #174.
pub fn module_path(file: &str, package_root: &str) -> String {
    // Both sides reduced to segments before anything is compared: a
    // `package_root` of `.` makes `./src`, which `strip_prefix` does not match
    // against `src/...` — and the whole module path then keeps its `src/`,
    // which is a wrong segment in every identity the file declares.
    let segments_of = |path: &str| -> Vec<String> {
        std::path::Path::new(path)
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .filter(|s| !s.is_empty() && *s != ".")
            .map(str::to_string)
            .collect()
    };
    let root = segments_of(package_root);
    let mut segments = segments_of(file);
    if segments.starts_with(&root) {
        segments.drain(..root.len());
    }
    if segments.first().is_some_and(|s| s == "src") {
        segments.remove(0);
    }
    if let Some(last) = segments.last_mut() {
        *last = module_segment(last).to_string();
    }
    segments.join("/")
}

/// The module segment a FILE NAME reduces to: its name with every extension
/// dropped.
///
/// `.d.ts` and `.spec.ts` are ONE extension each as far as a module path is
/// concerned; `file_stem` would leave `.d` and `.spec` behind.
///
/// Hung on [`Grammar::module_segment`] as well as called here, because the two
/// sides of an import have to agree: the declaration side reduces
/// `src/lib/buckets.ts` and the use site reduces the specifier `./buckets.js`,
/// and a second spelling of this rule is how they would come to disagree. An
/// empty stem — a dotfile like `.eslintrc` — is left alone rather than reduced
/// to nothing.
pub fn module_segment(segment: &str) -> &str {
    match segment.split('.').next() {
        Some(stem) if !stem.is_empty() => stem,
        _ => segment,
    }
}

/// Reduce the source text of a TypeScript type to the one segment that names
/// it.
///
/// The single owner of that rule for this language (R7): a field's annotation
/// and a use site's `new T()` must reduce the same way or the two sides mint
/// different strings and never merge.
///
/// A UNION is refused rather than reduced to one arm. `Widget | null` states
/// that the value is sometimes not a `Widget` at all, and picking the non-null
/// arm would type a receiver that may not have those members — a wrong identity,
/// which R4 ranks below no identity. The ONE exception is a union with `null`
/// or `undefined`, where every other arm is the same type: `Widget | null` has
/// exactly one type in it and TypeScript's own narrowing says so.
pub fn type_segment(raw: &str) -> Result<String, FqnError> {
    let not_a_type = || FqnError::NotATypeName { value: raw.to_string() };
    let mut rest = raw.trim();

    // `readonly T`, and a parenthesised type.
    loop {
        let before = rest;
        rest = rest.strip_prefix("readonly ").unwrap_or(rest).trim_start();
        if let Some(inner) = rest.strip_prefix('(').and_then(|r| r.strip_suffix(')')) {
            rest = inner.trim();
        }
        if rest == before {
            break;
        }
    }

    // A union or intersection names ONE type only when the other arms are the
    // absent-value types. `A | B` names two and there is nothing here to
    // choose between them.
    let arms: Vec<&str> = rest
        .split(['|', '&'])
        .map(str::trim)
        .filter(|a| !a.is_empty() && *a != "null" && *a != "undefined")
        .collect();
    let sole = match arms.as_slice() {
        [one] => *one,
        // Every arm the same type is one type written twice.
        [first, others @ ..] if others.iter().all(|o| o == first) => *first,
        _ => return Err(not_a_type()),
    };

    // `T[]` is an Array of T, whose members are Array's and not T's. Refused
    // for the same reason the Rust reader refuses `Arc<T>`.
    if sole.ends_with("[]") {
        return Err(not_a_type());
    }

    // Generic arguments belong to the use, not to the identity: `Widget<T>` and
    // `Widget` are one type. A qualified name is typed by its LAST segment; the
    // namespace in front says where it is declared, not what it is.
    let head = sole.split('<').next().unwrap_or(sole).trim();
    let name = head.rsplit('.').next().unwrap_or(head).trim();

    if name.is_empty() {
        return Err(not_a_type());
    }
    // A tuple, an object literal type, a string literal, a primitive keyword —
    // an identifier starts with a letter, `_` or `$`, so anything else names no
    // declaration this grammar can mint a segment for.
    if !name.starts_with(|c: char| c.is_alphabetic() || c == '_' || c == '$') {
        return Err(not_a_type());
    }
    if name.contains(SEPARATOR) {
        return Err(FqnError::SeparatorInSegment {
            segment: fqn::Segment::Type,
            value: name.to_string(),
        });
    }
    Ok(name.to_string())
}

// ── the flow-sensitive binding map (04b S1, S2) ──────────────────────────────

/// The type of every name in scope AT THIS POINT in the statement list.
///
/// Threaded through the walk by `&mut` rather than cloned per block, because a
/// JavaScript assignment reassigns the binding a `let` introduced rather than
/// introducing a new one (04b §1). Cloning per block is what makes the Rust
/// walk correct and would make this one WRONG.
#[derive(Clone, PartialEq, Eq)]
struct Flow {
    types: BTreeMap<String, String>,
}

impl Flow {
    fn empty() -> Self {
        Self { types: BTreeMap::new() }
    }

    fn get(&self, name: &str) -> Option<&str> {
        self.types.get(name).map(String::as_str)
    }

    /// **S1.** Record what the most recent assignment said. An assignment the
    /// walk cannot type CLEARS the binding rather than leaving the previous type
    /// in place — "I no longer know" is the truth, and keeping a stale type is
    /// the fabrication (R4).
    fn bind(&mut self, name: &str, ty: Option<String>) {
        match ty {
            Some(ty) => {
                self.types.insert(name.to_string(), ty);
            }
            None => {
                self.types.remove(name);
            }
        }
    }

    /// **S2.** Where control flow joins, a name keeps its type only if every arm
    /// agrees. Which arm ran is not knowable, so the join is the INTERSECTION
    /// and an empty intersection is an honest unknown — not "the last arm wins".
    fn join(arms: &[Flow]) -> Flow {
        let Some((first, rest)) = arms.split_first() else {
            return Flow::empty();
        };
        let mut out = first.clone();
        out.types.retain(|name, ty| rest.iter().all(|arm| arm.get(name) == Some(ty.as_str())));
        out
    }
}

// ── the walk ─────────────────────────────────────────────────────────────────

/// Where the walk currently is, in the terms the fqn grammar needs.
///
/// Small compared with the Rust walk's `Scope`, and deliberately: everything
/// the Rust one carries for TYPING a receiver lives in [`Flow`] here, because
/// in this language that state is positional rather than lexical.
#[derive(Clone)]
struct Scope {
    /// The symbol a use site found here sits inside — [`Reference::from`].
    from: Fqn,
    container: Container,
    /// The chain of FUNCTIONS enclosing this point, outermost first. See the
    /// Rust walk's `Scope::fn_scope`: the AST carries the parent, `from` was
    /// already being set from it, and only the naming side was not told — so a
    /// `const` in a function body minted at module scope and two of them in one
    /// file became one node.
    fn_scope: Vec<String>,
}

impl Scope {
    /// The module segment a declaration found HERE is named in — the file's
    /// module, extended by every enclosing function.
    fn module_here(&self, module: &str) -> String {
        if self.fn_scope.is_empty() {
            return module.to_string();
        }
        let inner = self.fn_scope.join("/");
        if module.is_empty() { format!("fn/{inner}") } else { format!("{module}/fn/{inner}") }
    }
}

/// What a declaration found here is a member OF. This is the only thing that
/// decides which fqn form a declaration takes, so the choice is made once.
#[derive(Clone)]
enum Container {
    /// A free item, at the module level or inside a `namespace`.
    File,
    /// A class, interface or enum body. Carries the module path the type is
    /// declared in as well as its name, because a `namespace` can move it.
    Type { module: String, name: String },
}

struct Walk<'a> {
    package: &'a str,
    module: &'a str,
    /// The text that was PARSED, which a Svelte script block is a slice of.
    text: &'a str,
    /// Where that text starts in the whole file, so a span reported here is one
    /// a reader can find.
    offset: u32,
    lines: &'a LineIndex,
    /// Where each type is declared. See [`TypeHomes`] — the walk is TOLD this,
    /// never looks it up, and an absent entry is the BOUNDARY rather than a
    /// licence to use the reading file's own module.
    ///
    /// It was once argued that this language does not need the table, because a
    /// class owns its members lexically and there is no `impl` block to sit
    /// somewhere else. That is true of the DECLARATION side and says nothing
    /// about the REFERENCE side: `b.error` is written in whatever file happens
    /// to hold `b`, and the member's identity carries the module its TYPE lives
    /// in. MEASURED: 936 field reads minted a candidate whose only defect was
    /// that segment, and not one of the 1,997 candidates minted at field reach
    /// named a declaration this scan holds.
    types: &'a TypeHomes,
    /// Every type THIS FILE declares.
    ///
    /// Consulted before the table, for the reason the Rust walk consults its own
    /// first: the table is a barrier artifact built from a COMPLETED pass, so
    /// the pass that builds it is handed an empty one and every local type would
    /// read as external. A type declared here lives here, whatever a later
    /// barrier says.
    ///
    /// A SET and not a map, because there is one answer: a class, an interface
    /// and an enum all name their members under `module_of`, which is this
    /// file's module for every container the language admits. Rust needs a map
    /// there — a type declared at file scope and used from `mod tests` is one
    /// module segment apart — and this language has no such split.
    declared_here: BTreeSet<String>,
    /// Names bound by `import * as ns`. A namespace is a MODULE, and its
    /// members are that module's exports — a different lookup from a method on
    /// a type, and conflating them mints members on a thing that has none
    /// (04b §3).
    namespaces: BTreeSet<String>,
    /// Every name the file ASSIGNS to anywhere, at any depth.
    ///
    /// What lets a closure be typed at all. A nested function can run at any
    /// time, so a name the enclosing scope holds may have been reassigned before
    /// it does — carrying the outer type in would be exactly the stale type S1
    /// refuses. But a name that is never assigned ANYWHERE cannot have changed,
    /// so its type still holds inside the closure. That is a proof, not an
    /// optimism, and it is what types `{() => store.load()}` in Svelte markup
    /// where `store` is a module-level `const`.
    reassigned: BTreeSet<String>,
    found: Found,
}

impl Walk<'_> {
    // ── plumbing ─────────────────────────────────────────────────────────────

    fn span(&self, span: OxcSpan) -> Span {
        self.lines.locate(span.start + self.offset, span.end + self.offset)
    }

    /// The source text a span covers, verbatim.
    ///
    /// Sliced rather than fetched-with-a-fallback. Both `text` and `offset` are
    /// rebound per script block, so a span is in range by construction and the
    /// old fallback was unreachable — but an empty string standing in for a
    /// failed read is one a caller cannot tell from a name the source carried,
    /// and it would have gone on to NAME a symbol. A bad span is now loud.
    fn text_of(&self, span: OxcSpan) -> &str {
        &self.text[span.start as usize..span.end as usize]
    }

    /// Where a type lives: THIS FILE first, then the scan, then outside.
    ///
    /// The same three-way answer the Rust walk gives, and for the same reasons —
    /// see [`Walk::declared_here`] for why the file is asked first, and
    /// [`Home`] for why "two of ours share the name" is not a miss.
    fn home_of(&self, ty: &str) -> Home<'_> {
        if self.declared_here.contains(ty) {
            return Home::Ours { module: self.module };
        }
        self.types.lookup(self.package, ty)
    }

    /// The module path a declaration found here is named in. One place, so a
    /// `namespace` cannot extend it for some declarations and not others.
    fn module_of(&self, scope: &Scope) -> String {
        match &scope.container {
            Container::File => self.module.to_string(),
            Container::Type { module, .. } => module.clone(),
        }
    }

    /// Mint the identity of a declaration named `member` in the current
    /// container. One place, so a declaration and a use site of it cannot pick
    /// different forms.
    fn declare(&self, scope: &Scope, member: &str, reach: Reach) -> Result<Fqn, FqnError> {
        let lang = Language::TypeScript;
        match &scope.container {
            Container::File => fqn::define(&Form::Item {
                lang,
                package: self.package,
                module: &scope.module_here(self.module),
                name: member,
                reach,
            }),
            Container::Type { module, name } => fqn::define(&Form::Member {
                lang,
                package: self.package,
                module,
                ty: name,
                member,
                reach,
            }),
        }
    }

    fn push(&mut self, symbol: Result<Symbol, FqnError>, scope: &Scope) -> Scope {
        match symbol {
            Ok(symbol) => {
                if let Container::Type { module, name } = &scope.container
                    && let Ok(owner) = fqn::define(&Form::Item {
                        lang: Language::TypeScript,
                        package: self.package,
                        module,
                        name,
                        reach: Reach::Item,
                    })
                {
                    self.found.relations.push(Relation {
                        kind: RelationKind::Owns,
                        child: symbol.fqn.clone(),
                        // The owner is a declaration in THIS file — the walk
                        // read both sides — so the rung is the first one.
                        parent: Resolution::Resolved { fqn: owner, via: Rung::DeclaredHere },
                        at: symbol.span,
                    });
                }
                let inner = Scope {
                    from: symbol.fqn.clone(),
                    container: scope.container.clone(),
                    fn_scope: scope.fn_scope.clone(),
                };
                self.found.symbols.push(symbol);
                inner
            }
            Err(_) => scope.clone(),
        }
    }

    fn symbol(
        &self,
        span: OxcSpan,
        scope: &Scope,
        name: &str,
        kind: SymbolKind,
        reach: Reach,
        declared_type: DeclaredType,
    ) -> Result<Symbol, FqnError> {
        Ok(Symbol {
            fqn: self.declare(scope, name, reach)?,
            kind,
            name: name.to_string(),
            span: self.span(span),
            // Nothing here is `export`ed until the enclosing statement says so;
            // `exported` below rewrites it rather than each arm guessing.
            visibility: Visibility::Private,
            docstring: None,
            declared_type,
            params: Vec::new(),
        })
    }

    fn emit(&mut self, scope: &Scope, kind: RefKind, at: OxcSpan, miss: Miss) {
        self.found.references.push(Reference {
            from: scope.from.clone(),
            kind,
            at: self.span(at),
            target: miss.resolution(),
        });
    }
}

// ── declarations (spec §3.1) ─────────────────────────────────────────────────

impl Walk<'_> {
    /// One statement, walked with the flow state as of just before it and
    /// leaving the state as of just after. ORDER is the whole of S1: nothing
    /// here may look ahead, and nothing may re-read a statement it has passed.
    fn statement(&mut self, statement: &Statement<'_>, scope: &Scope, flow: &mut Flow) {
        match statement {
            Statement::FunctionDeclaration(f) => self.function(f, scope, flow),
            Statement::ClassDeclaration(c) => self.class(c, scope, flow),
            Statement::VariableDeclaration(v) => self.variables(v, scope, flow),
            Statement::TSInterfaceDeclaration(i) => self.interface(i, scope, flow),
            Statement::TSTypeAliasDeclaration(a) => {
                let symbol = self.symbol(
                    a.span,
                    scope,
                    &a.id.name,
                    SymbolKind::TypeAlias,
                    Reach::Item,
                    DeclaredType::Stated(self.text_of(a.type_annotation.span()).to_string()),
                );
                let inner = self.push(symbol, scope);
                self.type_use(a.type_annotation.span(), &inner);
            }
            Statement::TSEnumDeclaration(e) => self.enumeration(e, scope),
            Statement::TSModuleDeclaration(m) => self.namespace(m, scope, flow),
            Statement::ImportDeclaration(i) => self.import(i),
            Statement::ExportNamedDeclaration(e) => self.export_named(e, scope, flow),
            Statement::ExportDefaultDeclaration(e) => self.export_default(e, scope, flow),
            Statement::ExportAllDeclaration(e) => self.export_all(e),
            Statement::ExpressionStatement(s) => self.expression(&s.expression, scope, flow),
            Statement::ReturnStatement(s) => {
                if let Some(argument) = &s.argument {
                    self.expression(argument, scope, flow);
                }
            }
            Statement::BlockStatement(b) => self.block(&b.body, scope, flow),
            Statement::IfStatement(s) => self.conditional(s, scope, flow),
            Statement::SwitchStatement(s) => self.switch(s, scope, flow),
            Statement::TryStatement(s) => self.try_statement(s, scope, flow),
            Statement::WhileStatement(s) => {
                self.expression(&s.test, scope, flow);
                self.loop_body(std::slice::from_ref(&s.body), scope, flow);
            }
            Statement::DoWhileStatement(s) => {
                self.loop_body(std::slice::from_ref(&s.body), scope, flow);
                self.expression(&s.test, scope, flow);
            }
            Statement::ForStatement(s) => self.for_statement(s, scope, flow),
            Statement::ForInStatement(s) => self.for_of(&s.right, &s.left, &s.body, scope, flow),
            Statement::ForOfStatement(s) => self.for_of(&s.right, &s.left, &s.body, scope, flow),
            Statement::ThrowStatement(s) => self.expression(&s.argument, scope, flow),
            Statement::LabeledStatement(s) => self.statement(&s.body, scope, flow),
            // Nothing to read and nothing to say. Distinct from a form with no
            // rule, which the arm below names.
            Statement::EmptyStatement(_)
            | Statement::BreakStatement(_)
            | Statement::ContinueStatement(_)
            | Statement::DebuggerStatement(_)
            | Statement::TSExportAssignment(_)
            | Statement::TSNamespaceExportDeclaration(_)
            | Statement::TSImportEqualsDeclaration(_) => {}
            // No arm yields NOTHING silently (R2, S8): a statement shape with no
            // rule here is named in the histogram under the file's own identity,
            // so what was missed can be counted instead of guessed at.
            other => {
                let at = other.span();
                let miss =
                    Miss::unhandled(statement_kind(other), self.text_of(at).trim(), Reach::Item);
                self.emit(scope, RefKind::Reads, at, miss);
            }
        }
    }

    /// A block introduces no new flow state: `let` is block-scoped but an
    /// assignment inside the block still reassigns the binding outside it, and
    /// the second is what S1 is about. Names the block DECLARES are dropped on
    /// the way out, which is what keeps a block-local `x` from typing an outer
    /// one.
    fn block(&mut self, body: &[Statement<'_>], scope: &Scope, flow: &mut Flow) {
        let before: Vec<String> = flow.types.keys().cloned().collect();
        for statement in body {
            self.statement(statement, scope, flow);
        }
        let declared = declared_in(body);
        for name in declared {
            if !before.contains(&name) {
                flow.bind(&name, None);
            }
        }
    }

    /// **S2.** Each arm is walked from the state before the branch, and what
    /// survives is what every arm agrees on. An arm that is absent contributes
    /// the state as it stood, which is the same intersection with the implicit
    /// empty arm written out.
    fn conditional(&mut self, s: &IfStatement<'_>, scope: &Scope, flow: &mut Flow) {
        self.expression(&s.test, scope, flow);
        let mut consequent = flow.clone();
        self.statement(&s.consequent, scope, &mut consequent);
        let mut alternate = flow.clone();
        if let Some(otherwise) = &s.alternate {
            self.statement(otherwise, scope, &mut alternate);
        }
        *flow = Flow::join(&[consequent, alternate]);
    }

    fn switch(&mut self, s: &SwitchStatement<'_>, scope: &Scope, flow: &mut Flow) {
        self.expression(&s.discriminant, scope, flow);
        let mut arms = Vec::new();
        let mut has_default = false;
        for case in &s.cases {
            if case.test.is_none() {
                has_default = true;
            }
            let mut arm = flow.clone();
            if let Some(test) = &case.test {
                self.expression(test, scope, &mut arm);
            }
            for statement in &case.consequent {
                self.statement(statement, scope, &mut arm);
            }
            arms.push(arm);
        }
        // With no `default`, falling through the whole switch is an arm too.
        if !has_default {
            arms.push(flow.clone());
        }
        *flow = Flow::join(&arms);
    }

    /// `try`/`catch`/`finally`. The `try` block may stop at ANY statement, so
    /// what reaches `catch` is not what reaches the end of `try` — the join
    /// after the whole statement is over the states that can actually arrive
    /// there, and the conservative reading of "any statement may have thrown"
    /// is the state as it stood before `try`.
    fn try_statement(&mut self, s: &TryStatement<'_>, scope: &Scope, flow: &mut Flow) {
        let mut attempted = flow.clone();
        self.block(&s.block.body, scope, &mut attempted);
        let mut arms = vec![attempted];
        if let Some(handler) = &s.handler {
            let mut caught = flow.clone();
            self.block(&handler.body.body, scope, &mut caught);
            arms.push(caught);
        }
        *flow = Flow::join(&arms);
        if let Some(finally) = &s.finalizer {
            self.block(&finally.body, scope, flow);
        }
    }

    /// A loop body may run zero times or many, and on the second pass it sees
    /// what the first pass left. A single forward walk cannot model that, so
    /// every name the body ASSIGNS is cleared before the body is read — the
    /// body then types nothing it might itself have changed, and the state
    /// after the loop joins against not having run at all.
    fn loop_body(&mut self, body: &[Statement<'_>], scope: &Scope, flow: &mut Flow) {
        let before = flow.clone();
        for name in assigned_in(body) {
            flow.bind(&name, None);
        }
        let mut inside = flow.clone();
        for statement in body {
            self.statement(statement, scope, &mut inside);
        }
        *flow = Flow::join(&[inside, before]);
    }

    fn for_statement(&mut self, s: &ForStatement<'_>, scope: &Scope, flow: &mut Flow) {
        if let Some(init) = &s.init {
            match init {
                ForStatementInit::VariableDeclaration(v) => self.variables(v, scope, flow),
                expression => {
                    if let Some(e) = expression.as_expression() {
                        self.expression(e, scope, flow);
                    }
                }
            }
        }
        if let Some(test) = &s.test {
            self.expression(test, scope, flow);
        }
        if let Some(update) = &s.update {
            self.expression(update, scope, flow);
        }
        self.loop_body(std::slice::from_ref(&s.body), scope, flow);
    }

    /// `for (const x of xs)`. The bound name's type would be the ELEMENT type of
    /// `xs`, which nothing in this file states — an array's element type is on
    /// the annotation if there is one, and `T[]` is refused by `type_segment`
    /// for the reason recorded there. So the binding is CLEARED rather than
    /// guessed, and the body's member calls on it stay unresolved.
    fn for_of(
        &mut self,
        right: &Expression<'_>,
        left: &ForStatementLeft<'_>,
        body: &Statement<'_>,
        scope: &Scope,
        flow: &mut Flow,
    ) {
        self.expression(right, scope, flow);
        if let ForStatementLeft::VariableDeclaration(v) = left {
            for declarator in &v.declarations {
                for name in bound_names(&declarator.id) {
                    flow.bind(&name, None);
                }
            }
        }
        self.loop_body(std::slice::from_ref(body), scope, flow);
    }

    // ── declarations ─────────────────────────────────────────────────────────

    fn function(&mut self, f: &Function<'_>, scope: &Scope, flow: &mut Flow) {
        let Some(id) = &f.id else {
            // An anonymous function declaration cannot be named, so it declares
            // nothing — its BODY is still read, or every use site inside it
            // would be lost.
            return self.function_body(f, scope, flow);
        };
        let kind = match scope.container {
            Container::File => SymbolKind::Function,
            Container::Type { .. } => SymbolKind::Method,
        };
        let returns = match &f.return_type {
            Some(annotation) => {
                DeclaredType::Stated(self.text_of(annotation.type_annotation.span()).to_string())
            }
            None => DeclaredType::Unstated,
        };
        let symbol =
            self.symbol(f.span, scope, &id.name, kind, Reach::Item, returns).map(|mut s| {
                s.params = self.params(&f.params);
                s
            });
        let mut inner = self.push(symbol, scope);
        // The body is INSIDE this function, and everything it declares is named
        // so — the same fact the Rust walk records, for the same reason.
        inner.fn_scope.push(id.name.to_string());
        self.function_body(f, &inner, flow);
    }

    /// A function body is walked in a flow state of its OWN, seeded from the
    /// signature.
    ///
    /// Not from the enclosing flow, and that is the conservative reading a
    /// closure forces: a nested function can be called at any time, including
    /// after every name the outer scope holds has been reassigned, so what was
    /// true at the point of definition is not what is true when the body runs.
    /// Carrying the outer state in would be exactly the stale type S1 refuses.
    fn function_body(&mut self, f: &Function<'_>, scope: &Scope, outer: &mut Flow) {
        let mut inner = self.captured(outer);
        self.bind_params(&f.params, &mut inner);
        self.param_defaults(&f.params, scope, &mut inner);
        if let Some(body) = &f.body {
            for statement in &body.statements {
                self.statement(statement, scope, &mut inner);
            }
        }
    }

    /// What a nested function may keep from the scope around it: the bindings
    /// whose names this file never assigns to. See [`Walk::reassigned`].
    fn captured(&self, outer: &Flow) -> Flow {
        let mut inner = Flow::empty();
        for (name, ty) in &outer.types {
            if !self.reassigned.contains(name) {
                inner.bind(name, Some(ty.clone()));
            }
        }
        inner
    }

    /// **04b S3, route 3.** A parameter's type is STATED in the signature, so
    /// the body knows it from its first statement. Only a plain name is bound:
    /// a destructuring parameter binds several names of several types and
    /// giving each the whole type is false.
    fn bind_params(&mut self, params: &FormalParameters<'_>, flow: &mut Flow) {
        for param in &params.items {
            let BindingPattern::BindingIdentifier(id) = &param.pattern else {
                continue;
            };
            flow.bind(&id.name, self.annotated_type(param.type_annotation.as_deref()));
        }
    }

    /// A parameter's DEFAULT is an expression that runs, and it was walked
    /// nowhere — so `function f(now: Date = new Date())` lost the construction
    /// and `f({ reload = vi.fn() })` lost the call.
    ///
    /// Both spellings: the default on a plain parameter, and the one inside a
    /// destructuring pattern, which the grammar carries as an
    /// `AssignmentPattern` rather than on the parameter.
    fn param_defaults(&mut self, params: &FormalParameters<'_>, scope: &Scope, flow: &mut Flow) {
        for param in &params.items {
            if let Some(initializer) = &param.initializer {
                self.expression(initializer, scope, flow);
            }
            self.pattern_defaults(&param.pattern, scope, flow);
        }
    }

    /// The defaults a binding pattern carries, however deeply nested:
    /// `{ a: { b = f() } }`.
    fn pattern_defaults(&mut self, pattern: &BindingPattern<'_>, scope: &Scope, flow: &mut Flow) {
        match pattern {
            BindingPattern::AssignmentPattern(a) => {
                self.expression(&a.right, scope, flow);
                self.pattern_defaults(&a.left, scope, flow);
            }
            BindingPattern::ObjectPattern(o) => {
                for property in &o.properties {
                    self.pattern_defaults(&property.value, scope, flow);
                }
            }
            BindingPattern::ArrayPattern(a) => {
                for element in a.elements.iter().flatten() {
                    self.pattern_defaults(element, scope, flow);
                }
            }
            BindingPattern::BindingIdentifier(_) => {}
        }
    }

    /// Parameters are typed PROPS on their function, never nodes (D2).
    fn params(&self, params: &FormalParameters<'_>) -> Vec<Param> {
        params
            .items
            .iter()
            .enumerate()
            .map(|(position, param)| Param {
                name: match &param.pattern {
                    BindingPattern::BindingIdentifier(id) => id.name.to_string(),
                    other => self.text_of(other.span()).to_string(),
                },
                position: position as u32,
                declared_type: match self.annotated_type(param.type_annotation.as_deref()) {
                    Some(ty) => DeclaredType::Stated(ty),
                    None => DeclaredType::Unstated,
                },
            })
            .collect()
    }

    /// The type an annotation STATES, reduced to the one segment that names it.
    /// `None` where the source states none — a `.js` file states none anywhere,
    /// which is why route 1 leads S3.
    fn annotated_type(&self, annotation: Option<&TSTypeAnnotation<'_>>) -> Option<String> {
        type_segment(self.text_of(annotation?.type_annotation.span())).ok()
    }

    fn class(&mut self, c: &Class<'_>, scope: &Scope, flow: &mut Flow) {
        let Some(id) = &c.id else {
            return;
        };
        let symbol = self.symbol(
            c.span,
            scope,
            &id.name,
            SymbolKind::Class,
            Reach::Item,
            DeclaredType::Unstated,
        );
        let outer = self.push(symbol, scope);
        let child = outer.from.clone();

        // `extends` and `implements` are the two things this language states in
        // the shape an inheritance edge is stated in. A class body on its own
        // states neither, and emitting one for it would be a false edge that
        // pattern detection reads as real (R8).
        if let Some(parent) = &c.super_class {
            let miss = self.name_type(parent.span(), &outer);
            self.found.relations.push(Relation {
                kind: RelationKind::Extends,
                child: child.clone(),
                parent: miss.resolution(),
                at: self.span(parent.span()),
            });
        }
        for implemented in &c.implements {
            let miss = self.name_type(implemented.span(), &outer);
            self.found.relations.push(Relation {
                kind: RelationKind::Implements,
                child: child.clone(),
                parent: miss.resolution(),
                at: self.span(implemented.span()),
            });
        }

        let inner = Scope {
            from: outer.from.clone(),
            fn_scope: outer.fn_scope.clone(),
            container: Container::Type { module: self.module_of(scope), name: id.name.to_string() },
        };
        for element in &c.body.body {
            self.class_element(element, &inner, flow);
        }
    }

    fn class_element(&mut self, element: &ClassElement<'_>, scope: &Scope, flow: &mut Flow) {
        match element {
            ClassElement::MethodDefinition(m) => {
                let Some(name) = property_name(&m.key) else {
                    return;
                };
                // A getter or setter PRESENTS as a slot and is code; the fact
                // vocabulary keeps those apart (`Property` vs `Field`).
                let kind = match m.kind {
                    MethodDefinitionKind::Get | MethodDefinitionKind::Set => SymbolKind::Property,
                    _ => SymbolKind::Method,
                };
                let returns = match &m.value.return_type {
                    Some(a) => {
                        DeclaredType::Stated(self.text_of(a.type_annotation.span()).to_string())
                    }
                    None => DeclaredType::Unstated,
                };
                let symbol =
                    self.symbol(m.span, scope, &name, kind, Reach::Item, returns).map(|mut s| {
                        s.params = self.params(&m.value.params);
                        s
                    });
                let inner = self.push(symbol, scope);
                self.function_body(&m.value, &inner, flow);
            }
            ClassElement::PropertyDefinition(p) => {
                let Some(name) = property_name(&p.key) else {
                    return;
                };
                let declared = match &p.type_annotation {
                    Some(a) => {
                        DeclaredType::Stated(self.text_of(a.type_annotation.span()).to_string())
                    }
                    None => DeclaredType::Unstated,
                };
                let symbol =
                    self.symbol(p.span, scope, &name, SymbolKind::Field, Reach::Field, declared);
                let inner = self.push(symbol, scope);
                if let Some(a) = &p.type_annotation {
                    self.type_use(a.type_annotation.span(), &inner);
                }
                if let Some(value) = &p.value {
                    let mut initialiser = Flow::empty();
                    self.expression(value, &inner, &mut initialiser);
                }
            }
            ClassElement::StaticBlock(b) => {
                let mut inner = Flow::empty();
                for statement in &b.body {
                    self.statement(statement, scope, &mut inner);
                }
            }
            ClassElement::AccessorProperty(p) => {
                if let Some(name) = property_name(&p.key) {
                    let symbol = self.symbol(
                        p.span,
                        scope,
                        &name,
                        SymbolKind::Property,
                        Reach::Field,
                        DeclaredType::Unstated,
                    );
                    self.push(symbol, scope);
                }
            }
            // A `declare` member states a type and no code. It is a
            // declaration, and it is read as one.
            ClassElement::TSIndexSignature(_) => {}
        }
    }

    fn interface(&mut self, i: &TSInterfaceDeclaration<'_>, scope: &Scope, _flow: &mut Flow) {
        let symbol = self.symbol(
            i.span,
            scope,
            &i.id.name,
            SymbolKind::Interface,
            Reach::Item,
            DeclaredType::Unstated,
        );
        let outer = self.push(symbol, scope);
        let child = outer.from.clone();
        for extended in &i.extends {
            let miss = self.name_type(extended.span(), &outer);
            self.found.relations.push(Relation {
                kind: RelationKind::Extends,
                child: child.clone(),
                parent: miss.resolution(),
                at: self.span(extended.span()),
            });
        }
        let inner = Scope {
            from: outer.from.clone(),
            fn_scope: outer.fn_scope.clone(),
            container: Container::Type {
                module: self.module_of(scope),
                name: i.id.name.to_string(),
            },
        };
        for member in &i.body.body {
            let (span, key, kind, reach) = match member {
                TSSignature::TSPropertySignature(p) => {
                    (p.span, property_name(&p.key), SymbolKind::Field, Reach::Field)
                }
                TSSignature::TSMethodSignature(m) => {
                    (m.span, property_name(&m.key), SymbolKind::Method, Reach::Item)
                }
                _ => continue,
            };
            let Some(key) = key else { continue };
            let symbol = self.symbol(span, &inner, &key, kind, reach, DeclaredType::Unstated);
            self.push(symbol, &inner);
        }
    }

    fn enumeration(&mut self, e: &TSEnumDeclaration<'_>, scope: &Scope) {
        let symbol = self.symbol(
            e.span,
            scope,
            &e.id.name,
            SymbolKind::Enum,
            Reach::Item,
            DeclaredType::Unstated,
        );
        let outer = self.push(symbol, scope);
        let inner = Scope {
            from: outer.from.clone(),
            fn_scope: outer.fn_scope.clone(),
            container: Container::Type {
                module: self.module_of(scope),
                name: e.id.name.to_string(),
            },
        };
        for member in &e.body.members {
            let name = match &member.id {
                TSEnumMemberName::Identifier(id) => id.name.to_string(),
                TSEnumMemberName::String(s) => s.value.to_string(),
                _ => continue,
            };
            let symbol = self.symbol(
                member.span,
                &inner,
                &name,
                SymbolKind::EnumVariant,
                Reach::Item,
                DeclaredType::Unstated,
            );
            self.push(symbol, &inner);
        }
    }

    /// A TypeScript `namespace` extends the module path, so a declaration inside
    /// one is named the same as if it lived in its own file. Two spellings of
    /// one symbol would never merge (spec §2).
    fn namespace(&mut self, m: &TSModuleDeclaration<'_>, scope: &Scope, flow: &mut Flow) {
        let TSModuleDeclarationName::Identifier(id) = &m.id else {
            return;
        };
        let symbol = self.symbol(
            m.span,
            scope,
            &id.name,
            SymbolKind::Module,
            MODULE,
            DeclaredType::Unstated,
        );
        let outer = self.push(symbol, scope);
        let Some(TSModuleDeclarationBody::TSModuleBlock(block)) = &m.body else {
            return;
        };
        let inner = Scope {
            from: outer.from.clone(),
            container: Container::File,
            fn_scope: outer.fn_scope.clone(),
        };
        for statement in &block.body {
            self.statement(statement, &inner, flow);
        }
    }

    /// `const`/`let`/`var`. A `const` whose initialiser IS a function is a
    /// function — `export const load = () => {}` is the shape SvelteKit states
    /// every route entry point in, and filing it as a constant would leave the
    /// call graph with no node to point at.
    fn variables(&mut self, v: &VariableDeclaration<'_>, scope: &Scope, flow: &mut Flow) {
        for declarator in &v.declarations {
            // A dynamic `import()` states a dependency in a string literal just
            // as a static clause does, so it is recorded as the import it is —
            // before anything below reads the binding.
            if let Some(path) = declarator.init.as_ref().and_then(dynamic_import_of) {
                self.dynamic_import(path, &declarator.id, declarator.span);
            }
            // A function initialiser bound to a PLAIN NAME is walked once, by
            // the typed arm at the end of this loop, which gives its body the
            // declaration's own scope. Walking it here as well emitted every
            // symbol inside it TWICE — measured at 375 duplicate identities,
            // a third of everything A7 was reporting.
            //
            // The `BindingIdentifier` half of the condition is load-bearing: a
            // destructuring pattern `continue`s below without ever reaching
            // that arm, so skipping the outer walk for one would lose the whole
            // body instead of de-duplicating it.
            let re_walked_below = matches!(declarator.id, BindingPattern::BindingIdentifier(_))
                && declarator.init.as_ref().is_some_and(is_a_function);

            // Otherwise the INITIALISER is walked first, in the outer state:
            // `const x = x.foo()` reads the outer `x`, and binding first would
            // type it as itself.
            if let Some(init) = &declarator.init
                && !re_walked_below
            {
                self.expression(init, scope, flow);
            }

            let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                // A destructuring pattern binds several names of several types.
                // Each is CLEARED rather than given the whole type.
                for name in bound_names(&declarator.id) {
                    flow.bind(&name, None);
                }
                continue;
            };

            let is_function = declarator.init.as_ref().is_some_and(|e| {
                matches!(
                    e,
                    Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_)
                )
            });
            let kind = if is_function {
                SymbolKind::Function
            } else if v.kind == VariableDeclarationKind::Const {
                SymbolKind::Const
            } else {
                // A `let` or `var` has an address that outlives the statement,
                // which is the distinction `Static` carries (R8 reads it for
                // singleton detection).
                SymbolKind::Static
            };
            let declared = match self.annotated_type(declarator.type_annotation.as_deref()) {
                Some(ty) => DeclaredType::Stated(ty),
                None => DeclaredType::Unstated,
            };
            let symbol = self.symbol(declarator.span, scope, &id.name, kind, Reach::Item, declared);
            let inner = self.push(symbol, scope);

            // The two STATED routes a binding can carry, annotation first: it is
            // what the author wrote about the binding, while the initialiser is
            // only what happens to be assigned here.
            let ty = self
                .annotated_type(declarator.type_annotation.as_deref())
                .or_else(|| declarator.init.as_ref().and_then(|e| self.stated_type(e)));
            flow.bind(&id.name, ty);

            // A function assigned to a name is a function BODY, and its own
            // flow state, for the reason `function_body` records.
            if let Some(init) = &declarator.init {
                match init {
                    Expression::ArrowFunctionExpression(a) => {
                        let mut nested = self.captured(flow);
                        self.bind_params(&a.params, &mut nested);
                        self.param_defaults(&a.params, &inner, &mut nested);
                        let mut body_scope = inner.clone();
                        body_scope.fn_scope.push(id.name.to_string());
                        for statement in &a.body.statements {
                            self.statement(statement, &body_scope, &mut nested);
                        }
                    }
                    Expression::FunctionExpression(f) => {
                        let mut body_scope = inner.clone();
                        body_scope.fn_scope.push(id.name.to_string());
                        self.function_body(f, &body_scope, flow);
                    }
                    _ => {}
                }
            }
        }
    }

    // ── use sites (spec §3.2, R2) ────────────────────────────────────────────

    /// Every expression. No arm yields nothing for a shape that NAMES something:
    /// a call, a construction, a member access and a type use each emit exactly
    /// one reference, and a shape with no rule is named in the histogram.
    fn expression(&mut self, expression: &Expression<'_>, scope: &Scope, flow: &mut Flow) {
        match expression {
            Expression::CallExpression(c) => {
                let miss = self.name_callee(&c.callee, scope, flow);
                self.emit(scope, RefKind::Calls, c.span, miss);
                // The RECEIVER of a member call is an expression in its own
                // right, and skipping it would lose `a.b().c()`'s inner call.
                if let Some(member) = as_member(&c.callee) {
                    self.expression(member_object(member), scope, flow);
                } else if !matches!(c.callee, Expression::Identifier(_)) {
                    self.expression(&c.callee, scope, flow);
                }
                for argument in &c.arguments {
                    self.argument(argument, scope, flow);
                }
            }
            Expression::NewExpression(n) => {
                let miss = self.name_type(n.callee.span(), scope);
                self.emit(scope, RefKind::Constructs, n.span, miss);
                for argument in &n.arguments {
                    self.argument(argument, scope, flow);
                }
            }
            Expression::StaticMemberExpression(m) => {
                let miss = self.name_member(
                    &m.object,
                    &m.property.name,
                    "StaticMemberExpression",
                    scope,
                    flow,
                );
                self.emit(scope, RefKind::Reads, m.span, miss);
                self.expression(&m.object, scope, flow);
            }
            // `this.#count`, read. The same shape as the static member above and
            // it had no arm at all, so 167 of these fell to the catch-all and
            // were dropped — and A2's counter shared the blind spot, so the
            // subtraction that is meant to catch a drop came out zero.
            Expression::PrivateFieldExpression(p) => {
                let miss = self.name_member(
                    &p.object,
                    &private_name(&p.field),
                    "PrivateFieldExpression",
                    scope,
                    flow,
                );
                self.emit(scope, RefKind::Reads, p.span, miss);
                self.expression(&p.object, scope, flow);
            }
            Expression::ComputedMemberExpression(m) => {
                // The member is an EXPRESSION, so which member is reached is not
                // knowable without running it. Named as a miss rather than
                // guessed at from a literal, because half of them are variables.
                let miss = Miss::because(
                    Reason::DynamicDispatch,
                    "ComputedMemberExpression",
                    self.text_of(m.expression.span()).trim(),
                    Reach::Field,
                    vec![Observation::Receiver(self.text_of(m.object.span()).to_string())],
                );
                self.emit(scope, RefKind::Reads, m.span, miss);
                self.expression(&m.object, scope, flow);
                self.expression(&m.expression, scope, flow);
            }
            Expression::AssignmentExpression(a) => self.assignment(a, scope, flow),
            Expression::ArrowFunctionExpression(a) => {
                let mut nested = self.captured(flow);
                self.bind_params(&a.params, &mut nested);
                self.param_defaults(&a.params, scope, &mut nested);
                for statement in &a.body.statements {
                    self.statement(statement, scope, &mut nested);
                }
            }
            Expression::FunctionExpression(f) => self.function_body(f, scope, flow),
            Expression::ClassExpression(c) => self.class(c, scope, flow),
            Expression::AwaitExpression(a) => self.expression(&a.argument, scope, flow),
            // `buckets[idx].c++` READS and WRITES its argument, and had no arm
            // at all — so the member and everything inside the index went
            // unseen.
            Expression::UpdateExpression(u) => match &u.argument {
                SimpleAssignmentTarget::StaticMemberExpression(m) => {
                    let miss = self.name_member(
                        &m.object,
                        &m.property.name,
                        "StaticMemberExpression",
                        scope,
                        flow,
                    );
                    self.emit(scope, RefKind::Writes, m.span, miss);
                    self.expression(&m.object, scope, flow);
                }
                SimpleAssignmentTarget::PrivateFieldExpression(p) => {
                    let miss = self.name_member(
                        &p.object,
                        &private_name(&p.field),
                        "PrivateFieldExpression",
                        scope,
                        flow,
                    );
                    self.emit(scope, RefKind::Writes, p.span, miss);
                    self.expression(&p.object, scope, flow);
                }
                SimpleAssignmentTarget::ComputedMemberExpression(m) => {
                    self.expression(&m.object, scope, flow);
                    self.expression(&m.expression, scope, flow);
                }
                _ => {}
            },
            Expression::ParenthesizedExpression(p) => self.expression(&p.expression, scope, flow),
            Expression::UnaryExpression(u) => self.expression(&u.argument, scope, flow),
            Expression::BinaryExpression(b) => {
                self.expression(&b.left, scope, flow);
                self.expression(&b.right, scope, flow);
            }
            Expression::LogicalExpression(l) => {
                self.expression(&l.left, scope, flow);
                self.expression(&l.right, scope, flow);
            }
            Expression::ConditionalExpression(c) => {
                self.expression(&c.test, scope, flow);
                self.expression(&c.consequent, scope, flow);
                self.expression(&c.alternate, scope, flow);
            }
            Expression::SequenceExpression(s) => {
                for e in &s.expressions {
                    self.expression(e, scope, flow);
                }
            }
            Expression::ArrayExpression(a) => {
                for element in &a.elements {
                    match element {
                        ArrayExpressionElement::SpreadElement(s) => {
                            self.expression(&s.argument, scope, flow)
                        }
                        other => {
                            if let Some(e) = other.as_expression() {
                                self.expression(e, scope, flow);
                            }
                        }
                    }
                }
            }
            Expression::ObjectExpression(o) => {
                for property in &o.properties {
                    match property {
                        ObjectPropertyKind::ObjectProperty(p) => {
                            // A COMPUTED key is an expression that runs:
                            // `{ [includeKey('company', 1)]: false }` carries a
                            // call in the key, not in the value.
                            if p.computed
                                && let Some(key) = p.key.as_expression()
                            {
                                self.expression(key, scope, flow);
                            }
                            self.expression(&p.value, scope, flow)
                        }
                        // `{ ...authHeaders(token) }` — a SPREAD is the same
                        // expression it would be without the dots, and skipping
                        // it drops the call inside. MEASURED: spread in all
                        // three positions was most of A2's 383 dropped
                        // references.
                        ObjectPropertyKind::SpreadProperty(s) => {
                            self.expression(&s.argument, scope, flow)
                        }
                    }
                }
            }
            Expression::TemplateLiteral(t) => {
                for e in &t.expressions {
                    self.expression(e, scope, flow);
                }
            }
            Expression::TSAsExpression(a) => self.expression(&a.expression, scope, flow),
            Expression::TSNonNullExpression(n) => self.expression(&n.expression, scope, flow),
            Expression::TSSatisfiesExpression(s) => self.expression(&s.expression, scope, flow),
            Expression::ChainExpression(c) => match &c.expression {
                ChainElement::CallExpression(call) => {
                    let miss = self.name_callee(&call.callee, scope, flow);
                    self.emit(scope, RefKind::Calls, call.span, miss);
                    if let Some(member) = as_member(&call.callee) {
                        self.expression(member_object(member), scope, flow);
                    }
                }
                ChainElement::StaticMemberExpression(m) => {
                    let miss = self.name_member(
                        &m.object,
                        &m.property.name,
                        "StaticMemberExpression",
                        scope,
                        flow,
                    );
                    self.emit(scope, RefKind::Reads, m.span, miss);
                    self.expression(&m.object, scope, flow);
                }
                // `parts[0]?.[0]` — optional chaining over a COMPUTED member.
                ChainElement::ComputedMemberExpression(m) => {
                    let miss = Miss::because(
                        Reason::DynamicDispatch,
                        "ComputedMemberExpression",
                        self.text_of(m.expression.span()).trim(),
                        Reach::Field,
                        vec![Observation::Receiver(self.text_of(m.object.span()).to_string())],
                    );
                    self.emit(scope, RefKind::Reads, m.span, miss);
                    self.expression(&m.object, scope, flow);
                    self.expression(&m.expression, scope, flow);
                }
                _ => {}
            },
            // A literal, a bare identifier, `this`, a regexp. None of these
            // NAMES a target this walk can place: a bare identifier read is a
            // local variable as often as it is an import, and emitting every one
            // would bury the graph in references that can never resolve. The
            // rung that could tell them apart needs scope analysis the ladder
            // does not have either.
            _ => {}
        }
    }

    /// One argument, spread or not. `f(...xs.map(g))` carries a call that a
    /// plain `as_expression()` returns `None` for.
    fn argument(&mut self, argument: &Argument<'_>, scope: &Scope, flow: &mut Flow) {
        match argument {
            Argument::SpreadElement(s) => self.expression(&s.argument, scope, flow),
            other => {
                if let Some(e) = other.as_expression() {
                    self.expression(e, scope, flow);
                }
            }
        }
    }

    /// `x = …`, which REASSIGNS rather than declares. This is S1's other half:
    /// the type after the statement is what the right-hand side states, and
    /// `None` CLEARS.
    fn assignment(&mut self, a: &AssignmentExpression<'_>, scope: &Scope, flow: &mut Flow) {
        self.expression(&a.right, scope, flow);
        match &a.left {
            AssignmentTarget::AssignmentTargetIdentifier(id) => {
                let ty = if a.operator == AssignmentOperator::Assign {
                    self.stated_type(&a.right)
                } else {
                    // A compound assignment (`x += y`) produces a primitive or a
                    // string, never the type `x` had.
                    None
                };
                flow.bind(&id.name, ty);
            }
            AssignmentTarget::StaticMemberExpression(m) => {
                let miss = self.name_member(
                    &m.object,
                    &m.property.name,
                    "StaticMemberExpression",
                    scope,
                    flow,
                );
                self.emit(scope, RefKind::Writes, m.span, miss);
                self.expression(&m.object, scope, flow);
            }
            AssignmentTarget::PrivateFieldExpression(p) => {
                let miss = self.name_member(
                    &p.object,
                    &private_name(&p.field),
                    "PrivateFieldExpression",
                    scope,
                    flow,
                );
                self.emit(scope, RefKind::Writes, p.span, miss);
                self.expression(&p.object, scope, flow);
            }
            AssignmentTarget::ComputedMemberExpression(m) => {
                // `byProject[r.projectId] = g`. Which member is written is not
                // knowable without running it — the same reason a computed READ
                // is `DynamicDispatch` — but the WRITE happened and the index
                // expression is a use site of its own. The catch-all below
                // emitted neither.
                let miss = Miss::because(
                    Reason::DynamicDispatch,
                    "ComputedMemberExpression",
                    self.text_of(m.expression.span()).trim(),
                    Reach::Field,
                    vec![Observation::Receiver(self.text_of(m.object.span()).to_string())],
                );
                self.emit(scope, RefKind::Writes, m.span, miss);
                self.expression(&m.object, scope, flow);
                self.expression(&m.expression, scope, flow);
            }
            other => {
                // A destructuring target binds several names; each is cleared.
                for name in assignment_target_names(other) {
                    flow.bind(&name, None);
                }
            }
        }
    }

    /// Whatever stands in callee position. Every shape either names something or
    /// says which shape defeated it — there is no path out of here that emits
    /// nothing, which is the defect this rewrite exists to remove (R2, S8).
    fn name_callee(&self, callee: &Expression<'_>, scope: &Scope, flow: &Flow) -> Miss {
        match callee {
            Expression::Identifier(id) => Miss::unplaced(
                "Identifier",
                &id.name,
                Reach::Item,
                considered(fqn::refer(&Form::Item {
                    lang: Language::TypeScript,
                    package: self.package,
                    module: self.module,
                    name: &id.name,
                    reach: Reach::Item,
                })),
            ),
            Expression::StaticMemberExpression(m) => {
                self.name_member_called(&m.object, &m.property.name, scope, flow)
            }
            // `this.#hydrate()`. A private method call is a member call in every
            // way that matters — the receiver types the same, the name is the
            // same one the declaration minted — and it reached the catch-all
            // below, where `DynamicDispatch` with no observations made it
            // terminal: the ladder climbs for `Unplaced` and there was no
            // candidate to climb to.
            Expression::PrivateFieldExpression(p) => {
                self.name_member_called(&p.object, &private_name(&p.field), scope, flow)
            }
            Expression::ParenthesizedExpression(p) => self.name_callee(&p.expression, scope, flow),
            Expression::TSNonNullExpression(n) => self.name_callee(&n.expression, scope, flow),
            // `f()()`, `(cond ? a : b)()`, `obj[k]()`. Which function runs is
            // chosen at run time and no static answer exists.
            other => Miss::because(
                Reason::DynamicDispatch,
                expression_kind(other),
                self.text_of(other.span()).trim(),
                Reach::Item,
                Vec::new(),
            ),
        }
    }

    /// A member in CALLEE position — `x.m()`. Reached like an item, because a
    /// method is.
    fn name_member_called(
        &self,
        object: &Expression<'_>,
        member: &str,
        scope: &Scope,
        flow: &Flow,
    ) -> Miss {
        self.member_of(object, member, "StaticMemberExpression", Reach::Item, scope, flow)
    }

    /// A member in READ or WRITE position — `x.y`. Reached through a dot with no
    /// call, which is [`Reach::Field`] and keeps a field apart from a same-named
    /// method.
    fn name_member(
        &self,
        object: &Expression<'_>,
        member: &str,
        node_kind: &str,
        scope: &Scope,
        flow: &Flow,
    ) -> Miss {
        self.member_of(object, member, node_kind, Reach::Field, scope, flow)
    }

    /// The one place a receiver is turned into a type, so the three STATED
    /// routes of 04b S3 are applied identically wherever a member is reached.
    fn member_of(
        &self,
        object: &Expression<'_>,
        member: &str,
        node_kind: &str,
        reach: Reach,
        scope: &Scope,
        flow: &Flow,
    ) -> Miss {
        // A NAMESPACE import binds a module, not a type. Its members are that
        // module's exports, reached through the import — so the candidate is a
        // path for the ladder to place, NOT a member of a type called `api`.
        // Conflating them mints members on a thing that has none (04b §3).
        if let Expression::Identifier(id) = object
            && self.namespaces.contains(&id.name.to_string())
        {
            return Miss::unplaced(
                node_kind,
                member,
                reach,
                vec![Observation::UnplacedType(format!("{}.{member}", id.name))],
            );
        }

        let Some(ty) = self.receiver_type(object, scope, flow) else {
            return Miss::because(
                Reason::ReceiverTypeUnknown,
                node_kind,
                member,
                reach,
                vec![Observation::Receiver(self.text_of(object.span()).to_string())],
            );
        };
        // The TYPE's home, not the reading file's. A member's identity carries
        // the module its type lives in, and the declaration side already names
        // it that way — the two sides agreeing is the merge contract (§2), so
        // filing a use site under the module that happens to be READING it is
        // two identities for one field.
        let saw = || vec![Observation::Receiver(self.text_of(object.span()).to_string())];
        match self.home_of(&ty) {
            Home::Ours { module } => Miss::unplaced(
                node_kind,
                member,
                reach,
                considered(fqn::refer(&Form::Member {
                    lang: Language::TypeScript,
                    package: self.package,
                    module,
                    ty: &ty,
                    member,
                    reach,
                })),
            ),
            // Two first-party types answer to this name, so there is no one
            // home and picking would be a coin toss recorded as a fact.
            Home::Ambiguous => {
                Miss::because(Reason::AmbiguousCandidates, node_kind, member, reach, saw())
            }
            // NOTHING OF OURS DECLARES THIS TYPE. The receiver IS known — it was
            // read off an annotation, an assertion or a `new` — and it is
            // outside, so this is the boundary and not a lookup that failed.
            //
            // `Record`, `HTMLElement`, `Partial`, `URL`: filing their members
            // under the reading file's module minted a first-party member of a
            // type this scan does not declare, which is the fabrication the Rust
            // walk stopped doing at 6,109 sites (R4). MEASURED here at 938.
            Home::NotOurs => {
                Miss::because(Reason::ExternalBoundary, node_kind, member, reach, saw())
            }
        }
    }

    /// What a receiver's type IS, when something states it.
    ///
    /// Exactly the routes 04b S3 sizes, and nothing more. A receiver the file
    /// does not type is reported as such: a guessed receiver type mints a wrong
    /// identity, which R4 ranks below no identity at all.
    fn receiver_type(&self, object: &Expression<'_>, scope: &Scope, flow: &Flow) -> Option<String> {
        match object {
            // `this` inside a class body is that class.
            Expression::ThisExpression(_) => match &scope.container {
                Container::Type { name, .. } => Some(name.clone()),
                Container::File => None,
            },
            Expression::Identifier(id) => flow.get(&id.name).map(str::to_string),
            Expression::ParenthesizedExpression(p) => {
                self.receiver_type(&p.expression, scope, flow)
            }
            Expression::TSNonNullExpression(n) => self.receiver_type(&n.expression, scope, flow),
            // `(x as T).m()` — the assertion STATES the type, which is the whole
            // point of writing it.
            Expression::TSAsExpression(a) => {
                type_segment(self.text_of(a.type_annotation.span())).ok()
            }
            Expression::NewExpression(n) => type_segment(self.text_of(n.callee.span())).ok(),
            // A chain (`a.b().c()`) needs the RETURN type of the inner call,
            // which is the cross-file lookup deferred to #174.
            _ => None,
        }
    }

    /// The type an initialiser STATES — 04b S3, route 1 and its assertion form.
    /// Everything else states none: `const x = helper()` names no type, and
    /// inferring one from the function's return type needs the graph rather
    /// than the file.
    fn stated_type(&self, expression: &Expression<'_>) -> Option<String> {
        match expression {
            Expression::NewExpression(n) => type_segment(self.text_of(n.callee.span())).ok(),
            Expression::TSAsExpression(a) => {
                type_segment(self.text_of(a.type_annotation.span())).ok()
            }
            Expression::ParenthesizedExpression(p) => self.stated_type(&p.expression),
            Expression::TSNonNullExpression(n) => self.stated_type(&n.expression),
            _ => None,
        }
    }

    /// A type named in an annotation, a bound, a construction.
    fn name_type(&self, at: OxcSpan, scope: &Scope) -> Miss {
        let raw = self.text_of(at);
        let Ok(name) = type_segment(raw) else {
            return Miss::unhandled("TSType", raw.trim(), Reach::Item);
        };
        let mut saw = considered(fqn::refer(&Form::Item {
            lang: Language::TypeScript,
            package: self.package,
            module: self.module,
            name: &name,
            reach: Reach::Item,
        }));
        // The name alone cannot say WHICH `Widget`; a qualified one can, and it
        // is the only place the head of `ns.Widget` survives the reduction.
        saw.push(Observation::UnplacedType(raw.trim().to_string()));
        let _ = scope;
        Miss::unplaced("TSType", &name, Reach::Item, saw)
    }

    fn type_use(&mut self, at: OxcSpan, scope: &Scope) {
        let miss = self.name_type(at, scope);
        self.emit(scope, RefKind::TypeUse, at, miss);
    }

    // ── imports (spec §2) ────────────────────────────────────────────────────

    fn import(&mut self, i: &ImportDeclaration<'_>) {
        let path = i.source.value.to_string();
        let at = self.span(i.span);
        let origin = import_origin(&path);
        let Some(specifiers) = &i.specifiers else {
            // `import './side-effect.js'` binds no name and is still an import:
            // it states a dependency, which is what the module graph is.
            self.found.imports.push(Import { path, binds: Binding::Glob, origin, at });
            return;
        };
        for specifier in specifiers {
            let binds = match specifier {
                // A NAMED import binds a member of the module, and the
                // specifier spells only the module — so which member is a fact
                // only the clause carries. `imported` and `local` differ under
                // an alias, and each answers a different question.
                ImportDeclarationSpecifier::ImportSpecifier(s) => Binding::MemberOf {
                    local: s.local.name.to_string(),
                    member: s.imported.name().to_string(),
                },
                // A DEFAULT import binds whatever the module exports as its
                // default, under a name this file chose. The module does not
                // declare that name, so the specifier — which names the module
                // — is the closest thing written down to the target.
                //
                // This corpus says the same: 317 of them, almost all
                // `import Widget from './Widget.svelte'`, where the component
                // IS the module and the module is the right target.
                ImportDeclarationSpecifier::ImportDefaultSpecifier(s) => {
                    Binding::Name(s.local.name.to_string())
                }
                ImportDeclarationSpecifier::ImportNamespaceSpecifier(s) => {
                    // A namespace binds a MODULE. Recorded so `member_of` above
                    // can tell `api.thing()` from a method call.
                    self.namespaces.insert(s.local.name.to_string());
                    Binding::Name(s.local.name.to_string())
                }
            };
            self.found.imports.push(Import {
                path: path.clone(),
                binds,
                origin: origin.clone(),
                at,
            });
        }
    }

    /// `const { go } = await import('./api')` — the same two bindings a static
    /// clause makes, from the other spelling.
    ///
    /// A test that has to register a mock before its subject loads has no other
    /// way to write the import, so without this the subject's whole suite
    /// reaches nothing. MEASURED: the five TypeScript functions a use site named
    /// by EXACT identity while the edge stayed missing are all this shape.
    ///
    /// An ARRAY pattern is not an import clause — `const [a] = await import(m)`
    /// destructures the module object by position, which names no export — and
    /// a nested pattern binds nothing the other module declares either. Both
    /// fall through rather than inventing a member name (R4).
    fn dynamic_import(&mut self, path: &str, id: &BindingPattern<'_>, at: OxcSpan) {
        let origin = import_origin(path);
        let at = self.span(at);
        match id {
            // Bound to ONE name, so the binding IS the module — which is what a
            // namespace is. Recorded as one so `member_of` keeps telling
            // `api.thing()` from a method call on a type called `api`.
            BindingPattern::BindingIdentifier(local) => {
                self.namespaces.insert(local.name.to_string());
                self.found.imports.push(Import {
                    path: path.to_string(),
                    binds: Binding::Name(local.name.to_string()),
                    origin,
                    at,
                });
            }
            BindingPattern::ObjectPattern(o) => {
                for property in &o.properties {
                    // A COMPUTED key names no export that can be read here, and
                    // only a plain identifier is a name this file then uses.
                    let (Some(member), BindingPattern::BindingIdentifier(local)) =
                        (property.key.static_name(), &property.value)
                    else {
                        continue;
                    };
                    self.found.imports.push(Import {
                        path: path.to_string(),
                        binds: Binding::MemberOf {
                            local: local.name.to_string(),
                            member: member.to_string(),
                        },
                        origin: origin.clone(),
                        at,
                    });
                }
            }
            BindingPattern::ArrayPattern(_) | BindingPattern::AssignmentPattern(_) => {}
        }
    }

    fn export_named(&mut self, e: &ExportNamedDeclaration<'_>, scope: &Scope, flow: &mut Flow) {
        if let Some(declaration) = &e.declaration {
            let before = self.found.symbols.len();
            self.declaration(declaration, scope, flow);
            // `export` is the visibility, stated by the statement rather than by
            // the declaration inside it — which is why the arms above do not
            // each guess at one.
            for symbol in &mut self.found.symbols[before..] {
                symbol.visibility = Visibility::Public;
            }
        }
        // `export { a } from './b'` re-exports, which binds `a` here from
        // another module — an import in every way that matters to the graph.
        if let Some(source) = &e.source {
            let path = source.value.to_string();
            let origin = import_origin(&path);
            let at = self.span(e.span);
            for specifier in &e.specifiers {
                // A re-export names a MEMBER of the other module for exactly
                // the reason a named import does. `local` is what that module
                // calls it and `exported` is what this one passes on, which is
                // the same two questions an alias asks.
                self.found.imports.push(Import {
                    path: path.clone(),
                    binds: Binding::MemberOf {
                        local: specifier.exported.name().to_string(),
                        member: specifier.local.name().to_string(),
                    },
                    origin: origin.clone(),
                    at,
                });
            }
        }
    }

    fn export_default(&mut self, e: &ExportDefaultDeclaration<'_>, scope: &Scope, flow: &mut Flow) {
        let before = self.found.symbols.len();
        match &e.declaration {
            ExportDefaultDeclarationKind::FunctionDeclaration(f) => self.function(f, scope, flow),
            ExportDefaultDeclarationKind::ClassDeclaration(c) => self.class(c, scope, flow),
            ExportDefaultDeclarationKind::TSInterfaceDeclaration(i) => {
                self.interface(i, scope, flow)
            }
            expression => {
                if let Some(e) = expression.as_expression() {
                    self.expression(e, scope, flow);
                }
            }
        }
        for symbol in &mut self.found.symbols[before..] {
            symbol.visibility = Visibility::Public;
        }
    }

    /// `export * from './b'` binds an UNKNOWN set of names, so a name that
    /// might have come from here is not proof that it did (R4).
    fn export_all(&mut self, e: &ExportAllDeclaration<'_>) {
        let path = e.source.value.to_string();
        let origin = import_origin(&path);
        self.found.imports.push(Import {
            path,
            binds: Binding::Glob,
            origin,
            at: self.span(e.span),
        });
    }

    fn declaration(&mut self, declaration: &Declaration<'_>, scope: &Scope, flow: &mut Flow) {
        match declaration {
            Declaration::VariableDeclaration(v) => self.variables(v, scope, flow),
            Declaration::FunctionDeclaration(f) => self.function(f, scope, flow),
            Declaration::ClassDeclaration(c) => self.class(c, scope, flow),
            Declaration::TSTypeAliasDeclaration(a) => {
                let symbol = self.symbol(
                    a.span,
                    scope,
                    &a.id.name,
                    SymbolKind::TypeAlias,
                    Reach::Item,
                    DeclaredType::Stated(self.text_of(a.type_annotation.span()).to_string()),
                );
                self.push(symbol, scope);
            }
            Declaration::TSInterfaceDeclaration(i) => self.interface(i, scope, flow),
            Declaration::TSEnumDeclaration(e) => self.enumeration(e, scope),
            Declaration::TSModuleDeclaration(m) => self.namespace(m, scope, flow),
            Declaration::TSImportEqualsDeclaration(_) | Declaration::TSGlobalDeclaration(_) => {}
        }
    }
}

// ── reading the shapes, without the walk's state ─────────────────────────────

/// Which side of the scanned source an import specifier points at (spec §2).
///
/// The specifier is the ONLY input, because absence is scan-order dependent and
/// resolution must not be (R6). A specifier starting with `.` or `/` is a path
/// inside this package; anything else names a package, and whether THAT package
/// is one this scan owns is the ladder's question, not this one's.
///
/// `$lib` and its neighbours are aliases a bundler config states, and this walk
/// is never handed that config — so they land here as external, which is a
/// dangling target rather than a wrong one (R4).
fn import_origin(path: &str) -> ImportOrigin {
    if path.starts_with('.') || path.starts_with('/') {
        return ImportOrigin::Local;
    }
    // A scoped package is `@scope/name`; everything after is a path inside it.
    let package = if path.starts_with('@') {
        path.splitn(3, '/').take(2).collect::<Vec<_>>().join("/")
    } else {
        path.split('/').next().unwrap_or(path).to_string()
    };
    if package.is_empty() {
        // Malformed rather than a package. Claiming External would mint a
        // library node out of nothing.
        return ImportOrigin::Local;
    }
    ImportOrigin::External { package }
}

/// Whether an expression IS a function — the shape that gets a scope of its own
/// and must therefore be walked exactly once.
fn is_a_function(expression: &Expression<'_>) -> bool {
    matches!(expression, Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_))
}

/// The name a property key STATES. A computed key states none — which member is
/// declared is not knowable without running it — so it declares nothing rather
/// than declaring something guessed.
fn property_name(key: &PropertyKey<'_>) -> Option<String> {
    match key {
        PropertyKey::StaticIdentifier(id) => Some(id.name.to_string()),
        PropertyKey::PrivateIdentifier(id) => Some(private_name(id)),
        PropertyKey::StringLiteral(s) => Some(s.value.to_string()),
        _ => None,
    }
}

/// What a `#private` member is CALLED, hash included.
///
/// The hash is part of the name and not decoration: `#n` and `n` are two
/// different members that a class may declare side by side, and dropping it
/// would merge them onto one node. One function, called by the DECLARATION side
/// through [`property_name`] and by every use-site arm, because the two sides
/// agreeing on this string is the whole of what makes a private member
/// reachable.
///
/// The fqn separator is a middle dot, not a hash, so a hashed segment is a legal
/// one and there is no encoding collision.
fn private_name(id: &PrivateIdentifier<'_>) -> String {
    format!("#{}", id.name)
}

/// Every plain name a binding pattern binds. A nested destructure binds several
/// and this names all of them, because each has to be CLEARED.
/// The specifier of a dynamic `import('./m')`, seen through any `await`.
///
/// Only a STRING LITERAL answers. `import(chosenAtRuntime)` names a module that
/// is not written down anywhere, and picking one would be exactly the
/// fabrication R4 forbids — so it is no answer, and the site stays a miss that
/// says so.
fn dynamic_import_of<'a>(init: &'a Expression<'a>) -> Option<&'a str> {
    match init {
        Expression::AwaitExpression(a) => dynamic_import_of(&a.argument),
        Expression::ImportExpression(i) => match &i.source {
            Expression::StringLiteral(s) => Some(s.value.as_str()),
            _ => None,
        },
        _ => None,
    }
}

fn bound_names(pattern: &BindingPattern<'_>) -> Vec<String> {
    let mut out = Vec::new();
    collect_bound(pattern, &mut out);
    out
}

fn collect_bound(pattern: &BindingPattern<'_>, out: &mut Vec<String>) {
    match pattern {
        BindingPattern::BindingIdentifier(id) => out.push(id.name.to_string()),
        BindingPattern::ObjectPattern(o) => {
            for property in &o.properties {
                collect_bound(&property.value, out);
            }
            if let Some(rest) = &o.rest {
                collect_bound(&rest.argument, out);
            }
        }
        BindingPattern::ArrayPattern(a) => {
            for element in a.elements.iter().flatten() {
                collect_bound(element, out);
            }
            if let Some(rest) = &a.rest {
                collect_bound(&rest.argument, out);
            }
        }
        BindingPattern::AssignmentPattern(a) => collect_bound(&a.left, out),
    }
}

/// Every plain name an assignment target binds, for the destructuring forms.
fn assignment_target_names(target: &AssignmentTarget<'_>) -> Vec<String> {
    match target {
        AssignmentTarget::AssignmentTargetIdentifier(id) => vec![id.name.to_string()],
        // A nested destructuring target's names are not enumerated here: every
        // one of them is cleared by the caller anyway, and the ones this misses
        // simply keep a type they may no longer have. That would be the stale
        // type S1 refuses, so the conservative answer is to say so — see the
        // test `a_destructuring_assignment_clears_every_name_it_touches`.
        _ => Vec::new(),
    }
}

/// Names a run of statements DECLARES at its own level.
fn declared_in(body: &[Statement<'_>]) -> Vec<String> {
    let mut out = Vec::new();
    for statement in body {
        if let Statement::VariableDeclaration(v) = statement {
            for declarator in &v.declarations {
                out.extend(bound_names(&declarator.id));
            }
        }
    }
    out
}

/// Every TYPE a run of statements declares, however deep — the three kinds a
/// member can hang off in this language.
///
/// A whole-tree pass and not a running set, for the reason the Rust walk reads
/// its struct fields the same way: a class may be declared AFTER the function
/// that constructs it, and a scope only flows downward. Depth matters too — a
/// `namespace`, an `export` clause and a conditional block all wrap a
/// declaration that still names its members under this file's module.
fn types_declared_in(body: &[Statement<'_>]) -> Vec<String> {
    struct Declared {
        names: Vec<String>,
    }
    impl<'a> oxc_ast_visit::Visit<'a> for Declared {
        fn visit_class(&mut self, class: &Class<'a>) {
            if let Some(id) = &class.id {
                self.names.push(id.name.to_string());
            }
            oxc_ast_visit::walk::walk_class(self, class);
        }
        fn visit_ts_interface_declaration(&mut self, i: &TSInterfaceDeclaration<'a>) {
            self.names.push(i.id.name.to_string());
            oxc_ast_visit::walk::walk_ts_interface_declaration(self, i);
        }
        fn visit_ts_enum_declaration(&mut self, e: &TSEnumDeclaration<'a>) {
            self.names.push(e.id.name.to_string());
            oxc_ast_visit::walk::walk_ts_enum_declaration(self, e);
        }
    }
    let mut found = Declared { names: Vec::new() };
    for statement in body {
        oxc_ast_visit::Visit::visit_statement(&mut found, statement);
    }
    found.names
}

/// Names a run of statements ASSIGNS anywhere inside it, however deep.
///
/// What a loop body has to clear before it is read: on the second pass the body
/// sees what the first pass left, and a single forward walk cannot model that.
fn assigned_in(body: &[Statement<'_>]) -> Vec<String> {
    struct Assigned {
        names: Vec<String>,
    }
    impl<'a> oxc_ast_visit::Visit<'a> for Assigned {
        fn visit_assignment_target(&mut self, target: &AssignmentTarget<'a>) {
            if let AssignmentTarget::AssignmentTargetIdentifier(id) = target {
                self.names.push(id.name.to_string());
            }
            oxc_ast_visit::walk::walk_assignment_target(self, target);
        }
        fn visit_update_expression(&mut self, update: &UpdateExpression<'a>) {
            if let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = &update.argument {
                self.names.push(id.name.to_string());
            }
        }
    }
    let mut found = Assigned { names: Vec::new() };
    for statement in body {
        oxc_ast_visit::Visit::visit_statement(&mut found, statement);
    }
    found.names
}

/// The member expression a callee is, if it is one.
fn as_member<'a, 'b>(callee: &'b Expression<'a>) -> Option<&'b Expression<'a>> {
    matches!(
        callee,
        Expression::StaticMemberExpression(_)
            | Expression::ComputedMemberExpression(_)
            | Expression::PrivateFieldExpression(_)
    )
    .then_some(callee)
}

/// The object half of a member expression.
fn member_object<'a, 'b>(member: &'b Expression<'a>) -> &'b Expression<'a> {
    match member {
        Expression::StaticMemberExpression(m) => &m.object,
        Expression::ComputedMemberExpression(m) => &m.object,
        Expression::PrivateFieldExpression(m) => &m.object,
        other => other,
    }
}

/// The label a statement shape carries in the histogram. oxc's node names, so a
/// reader can look the shape up in the grammar it came from.
fn statement_kind(statement: &Statement<'_>) -> &'static str {
    match statement {
        Statement::WithStatement(_) => "WithStatement",
        Statement::TSTypeAliasDeclaration(_) => "TSTypeAliasDeclaration",
        _ => "Statement",
    }
}

/// The same, for an expression.
fn expression_kind(expression: &Expression<'_>) -> &'static str {
    match expression {
        Expression::CallExpression(_) => "CallExpression",
        Expression::ComputedMemberExpression(_) => "ComputedMemberExpression",
        Expression::PrivateFieldExpression(_) => "PrivateFieldExpression",
        Expression::ConditionalExpression(_) => "ConditionalExpression",
        Expression::ArrowFunctionExpression(_) => "ArrowFunctionExpression",
        Expression::FunctionExpression(_) => "FunctionExpression",
        Expression::AwaitExpression(_) => "AwaitExpression",
        Expression::ChainExpression(_) => "ChainExpression",
        Expression::ThisExpression(_) => "ThisExpression",
        Expression::Super(_) => "Super",
        _ => "Expression",
    }
}

/// Byte offset -> (line, column), for a whole file.
///
/// Built once per read. Columns are BYTES within the line, which is what
/// tree-sitter reports on the Rust side, so a consumer comparing spans across
/// languages compares like with like.
pub(super) struct LineIndex {
    starts: Vec<u32>,
}

impl LineIndex {
    pub(super) fn of(text: &str) -> Self {
        let mut starts = vec![0u32];
        starts.extend(
            text.bytes().enumerate().filter(|(_, b)| *b == b'\n').map(|(i, _)| i as u32 + 1),
        );
        Self { starts }
    }

    fn at(&self, offset: u32) -> (u32, u32) {
        let line = self.starts.partition_point(|start| *start <= offset).max(1);
        (line as u32, offset - self.starts[line - 1])
    }

    pub(super) fn locate(&self, start: u32, end: u32) -> Span {
        let (start_line, start_col) = self.at(start);
        let (end_line, end_col) = self.at(end);
        Span { start_line, start_col, end_line, end_col }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(text: &str) -> FileFacts {
        read(
            &Source { package: "pkg", module: "lib/fixture", path: "src/lib/fixture.ts", text },
            &TypeHomes::unknown(),
        )
        .expect("the fixture parses")
    }

    fn js_facts(text: &str) -> FileFacts {
        read(
            &Source { package: "pkg", module: "lib/fixture", path: "src/lib/fixture.js", text },
            &TypeHomes::unknown(),
        )
        .expect("the fixture parses")
    }

    /// One file of a scan that has already run its type barrier — the module,
    /// the path and the table the walk is TOLD.
    ///
    /// Through the ADAPTER rather than through `read`, because the table is a
    /// [`LanguageAdapter::read`] argument and a helper that side-stepped the
    /// trait could keep passing while the real pipeline ignored what it was
    /// handed. That is the defect these tests exist for.
    fn read_in(module: &str, path: &str, text: &str, types: &TypeHomes) -> FileFacts {
        TypeScriptAdapter
            .read(&Source { package: "pkg", module, path, text }, types)
            .expect("the fixture parses")
    }

    /// Every reference's target, as `(name, resolution)`.
    fn targets(facts: &FileFacts) -> Vec<(String, String)> {
        facts
            .references
            .iter()
            .map(|r| match &r.target {
                Resolution::Resolved { fqn, .. } => ("?".to_string(), fqn.to_string()),
                Resolution::Unresolved { reason, evidence } => {
                    let candidate = evidence.saw.iter().find_map(|o| match o {
                        Observation::Candidate(fqn) => Some(fqn.to_string()),
                        _ => None,
                    });
                    (evidence.name.clone(), candidate.unwrap_or_else(|| format!("{reason:?}")))
                }
            })
            .collect()
    }

    /// What the walk decided the receiver of each member call was, as
    /// `name -> candidate identity or reason`.
    fn member_targets(facts: &FileFacts) -> Vec<(String, String)> {
        targets(facts)
    }

    /// A private member is a use site in every position a public one is.
    ///
    /// The hash is the ONLY difference between each pair below, so the assertion
    /// is a property rather than a hand-counted number — and three of the four
    /// pairs were 0 against 1 before this, because `PrivateFieldExpression` had
    /// no arm in `expression`, in `assignment`, or in the update arm. The fourth
    /// reached the walk and came out `DynamicDispatch` with nothing to climb.
    ///
    /// MUTATION: delete any one of the four new arms and that row goes to 0.
    #[test]
    fn a_private_field_is_a_use_site_in_every_position_a_public_one_is() {
        // READ, WRITE, UPDATE and CALL, each with its public twin, so the
        // assertion is "the hash changes nothing" rather than a hand-counted
        // number. Three of these four emitted NOTHING AT ALL before: only the
        // call reached the walk, and it reached it as a shape with no arm.
        let shapes = [
            ("read", "return this.#n;", "return this.n;"),
            ("write", "this.#n = 3;", "this.n = 3;"),
            ("update", "this.#n++;", "this.n++;"),
            ("call", "this.#m();", "this.m();"),
        ];
        for (what, private, public) in shapes {
            let of = |body: &str, members: &str| {
                let facts = js_facts(&format!("class T {{ {members}\n  go() {{ {body} }} }}\n"));
                facts
                    .references
                    .iter()
                    .filter(|r| matches!(r.kind, RefKind::Calls | RefKind::Reads | RefKind::Writes))
                    .count()
            };
            let hashed = of(private, "#n = 0;\n  #m() {}");
            let plain = of(public, "n = 0;\n  m() {}");
            assert_eq!(
                hashed, plain,
                "`{private}` produced {hashed} references and `{public}` produced {plain} — \
                 a {what} of a private member is the same use site as a {what} of a public one"
            );
            assert!(hashed > 0, "`{private}` produced no reference at all");
        }
    }

    /// The RECEIVER of a private access is an expression in its own right.
    ///
    /// `this.boxes[0].#hydrate(api)` is one call plus a computed member plus a
    /// static member, and the walk saw one of the three: the call arm recursed
    /// into a callee it had no arm for, so the receiver was never reached.
    ///
    /// MUTATION, and its symptom is not the one to expect: drop the
    /// `PrivateFieldExpression` arm from `member_object` and this goes to FOUR,
    /// not to one. The call arm then recurses into the whole private expression
    /// instead of into its object, and the read arm — which now exists — emits
    /// `#m` a second time. A duplicate edge, which is why the assertion is an
    /// exact count rather than a lower bound.
    #[test]
    fn the_receiver_of_a_private_access_is_walked_like_any_other() {
        let facts = js_facts("class T { #m() {} go() { this.boxes[0].#m(); } }\n");
        let sites = facts
            .references
            .iter()
            .filter(|r| matches!(r.kind, RefKind::Calls | RefKind::Reads | RefKind::Writes))
            .count();
        assert_eq!(
            sites,
            3,
            "expected the call, the index and the property; got {:?}",
            targets(&facts)
        );
    }

    /// A function initialiser is walked ONCE, whichever pattern binds it.
    ///
    /// It was walked twice: once as the declarator's initialiser and again by
    /// the typed arm that gives its body the declaration's own scope. Every
    /// symbol inside came out twice — MEASURED at 375 duplicate identities,
    /// about a third of everything A7 was reporting, and each one a declaration
    /// colliding with itself.
    ///
    /// The destructuring cases are here because the obvious fix breaks them: a
    /// pattern that is not a plain name `continue`s before reaching the typed
    /// arm, so skipping the outer walk for one loses the whole body rather than
    /// de-duplicating it.
    ///
    /// MUTATION: drop the `BindingIdentifier` half of `re_walked_below` — the
    /// two destructuring fixtures fall to zero references.
    #[test]
    fn a_function_initialiser_is_walked_exactly_once_whatever_the_binding_pattern() {
        for binding in ["const a =", "const { a } =", "const [a] ="] {
            for body in
                ["() => { const t = new T(); t.m(); }", "function () { const t = new T(); t.m(); }"]
            {
                let facts = js_facts(&format!("class T {{ m() {{}} }}\n{binding} {body};\n"));
                let calls: Vec<String> = member_targets(&facts)
                    .into_iter()
                    .filter(|(name, _)| name == "m")
                    .map(|(_, t)| t)
                    .collect();
                assert_eq!(
                    calls.len(),
                    1,
                    "`{binding} {body}` produced {} references to `m`, not one",
                    calls.len()
                );
                let declared: Vec<&str> = facts
                    .symbols
                    .iter()
                    .filter(|s| s.name == "t")
                    .map(|s| s.fqn.as_str())
                    .collect();
                assert_eq!(
                    declared.len(),
                    1,
                    "`{binding} {body}` declared `t` {} times: {declared:?}",
                    declared.len()
                );
            }
        }
    }

    /// A declaration inside a function body is named under that function.
    ///
    /// The same defect the Rust walk had, and the same cause: oxc carries the
    /// parent, `Scope::from` was already set from it, and only the NAMING side
    /// was never told it had gone inside a body. Two `const deadline` in two
    /// functions of one file minted one identity.
    ///
    /// MUTATION: drop `body_scope.fn_scope.push(..)` — the two collapse.
    #[test]
    fn a_declaration_inside_a_function_body_is_named_under_that_function() {
        let facts = js_facts(
            "export function a() { const deadline = 1; return deadline; }\n\
             export function b() { const deadline = 2; return deadline; }\n\
             export const deadline = 3;\n",
        );
        let minted: Vec<&str> =
            facts.symbols.iter().filter(|s| s.name == "deadline").map(|s| s.fqn.as_str()).collect();
        assert_eq!(minted.len(), 3, "three declarations: {minted:?}");
        let distinct: std::collections::BTreeSet<&&str> = minted.iter().collect();
        assert_eq!(distinct.len(), 3, "three declarations, three identities: {minted:?}");
        assert!(
            minted.iter().any(|f| f.contains("fixture/fn/a")),
            "the local is named under its function: {minted:?}"
        );
        assert!(
            minted.contains(&"typescript·pkg·lib/fixture·deadline·item"),
            "and the module-level one is untouched: {minted:?}"
        );
    }

    /// A Svelte 5 RUNE is in scope with nothing written, and names `svelte`.
    ///
    /// MEASURED: `$derived` 466, `$state` 316, `$props` 256 sat in
    /// `NoImportInScope` because no table could name them. They are not
    /// ECMAScript, which is why a prelude entry carries its own package
    /// rather than the language having one.
    ///
    /// MUTATION: file them under `ecmascript` — the identity then names a
    /// package that does not define them.
    #[test]
    fn a_svelte_rune_is_in_scope_with_nothing_written_and_belongs_to_svelte() {
        let runes: Vec<&str> =
            GRAMMAR.prelude.iter().filter(|(_, p, _)| *p == "svelte").map(|(n, _, _)| *n).collect();
        for rune in ["$state", "$derived", "$props", "$effect"] {
            assert!(runes.contains(&rune), "{rune} is a rune and is not in the prelude: {runes:?}");
        }
        assert!(
            GRAMMAR.prelude.iter().any(|(n, p, _)| *n == "console" && *p == "ecmascript"),
            "a real ECMAScript global still names ecmascript"
        );
    }

    // ── 04b S1: the most recent assignment wins ──────────────────────────────

    /// The test 04b §5 names first, and the reason `Scope::bindings` could not
    /// be reused. MUTATION: record at the declaration and read it anywhere in
    /// the block, Rust-style — the second call then types as `Foo`, which is a
    /// wrong identity, and R4 ranks that below no identity.
    #[test]
    fn a_reassignment_changes_the_type_at_the_next_use_site() {
        let facts = js_facts(
            "class Foo { m() {} }\n\
             class Bar { m() {} }\n\
             export function go(other) {\n\
             \x20 let x = new Foo();\n\
             \x20 x.m();\n\
             \x20 x = new Bar();\n\
             \x20 x.m();\n\
             }\n",
        );
        let found = member_targets(&facts);
        let calls: Vec<&String> =
            found.iter().filter(|(name, _)| name == "m").map(|(_, t)| t).collect();
        assert_eq!(calls.len(), 2, "two member calls, two references: {found:?}");
        assert!(calls[0].ends_with("Foo·m·item"), "the first call is Foo's: {calls:?}");
        assert!(calls[1].ends_with("Bar·m·item"), "the second call is Bar's: {calls:?}");
    }

    /// **S1.** MUTATION: leave the previous type in place when an assignment
    /// cannot be typed. The second call then claims `Foo::m` for a receiver
    /// whose type nothing states.
    #[test]
    fn an_untypable_reassignment_clears_the_binding_rather_than_keeping_it() {
        let facts = js_facts(
            "class Foo { m() {} }\n\
             export function go(make) {\n\
             \x20 let x = new Foo();\n\
             \x20 x.m();\n\
             \x20 x = make();\n\
             \x20 x.m();\n\
             }\n",
        );
        let calls: Vec<String> = member_targets(&facts)
            .into_iter()
            .filter(|(name, _)| name == "m")
            .map(|(_, t)| t)
            .collect();
        assert_eq!(calls.len(), 2, "two member calls, two references");
        assert!(calls[0].ends_with("Foo·m·item"), "the first call is Foo's: {calls:?}");
        assert_eq!(
            calls[1], "ReceiverTypeUnknown",
            "a call's return type is not stated here, so the binding is cleared"
        );
    }

    /// **S2.** MUTATION: take the last arm. The call after the join then claims
    /// `Bar::m` on a receiver that is a `Foo` whenever the other branch ran.
    #[test]
    fn two_branches_assigning_different_types_leave_the_binding_unknown() {
        let facts = js_facts(
            "class Foo { m() {} }\n\
             class Bar { m() {} }\n\
             export function go(c) {\n\
             \x20 let x = new Foo();\n\
             \x20 if (c) { x = new Bar(); } else { x = new Foo(); }\n\
             \x20 x.m();\n\
             }\n",
        );
        let after = member_targets(&facts)
            .into_iter()
            .filter(|(name, _)| name == "m")
            .map(|(_, t)| t)
            .next_back()
            .expect("the call after the join is a reference");
        assert_eq!(after, "ReceiverTypeUnknown", "the arms disagree, so the join knows nothing");
    }

    /// The other half of S2, and the half a too-eager "clear on any branch"
    /// would break: when every arm assigns the SAME type, the join keeps it.
    #[test]
    fn two_branches_assigning_one_type_keep_it_after_the_join() {
        let facts = js_facts(
            "class Foo { m() {} }\n\
             export function go(c) {\n\
             \x20 let x = null;\n\
             \x20 if (c) { x = new Foo(); } else { x = new Foo(); }\n\
             \x20 x.m();\n\
             }\n",
        );
        let after = member_targets(&facts)
            .into_iter()
            .filter(|(name, _)| name == "m")
            .map(|(_, t)| t)
            .next_back()
            .expect("the call after the join is a reference");
        assert!(after.ends_with("Foo·m·item"), "every arm said Foo, so the join says Foo: {after}");
    }

    /// A branch that assigns in ONE arm only joins against the value BEFORE the
    /// branch, which is the same intersection rule with the implicit empty arm
    /// written out.
    #[test]
    fn an_if_with_no_else_joins_against_what_was_true_before_it() {
        let facts = js_facts(
            "class Foo { m() {} }\n\
             class Bar { m() {} }\n\
             export function go(c) {\n\
             \x20 let x = new Foo();\n\
             \x20 if (c) { x = new Bar(); }\n\
             \x20 x.m();\n\
             }\n",
        );
        let after = member_targets(&facts)
            .into_iter()
            .filter(|(name, _)| name == "m")
            .map(|(_, t)| t)
            .next_back()
            .expect("the call after the branch is a reference");
        assert_eq!(after, "ReceiverTypeUnknown", "the branch may or may not have run");
    }

    // ── 04b S3: the three STATED routes ──────────────────────────────────────

    /// Route 1 of 3, and the only in-file route that works in a `.js` file
    /// at all. MUTATION: gate the route on TypeScript.
    #[test]
    fn a_constructor_names_the_type_even_with_no_annotations_in_the_file() {
        for facts in [
            js_facts("class T { m() {} }\nexport function go() { const x = new T(); x.m(); }\n"),
            facts("class T { m() {} }\nexport function go() { const x = new T(); x.m(); }\n"),
        ] {
            let call = member_targets(&facts)
                .into_iter()
                .find(|(name, _)| name == "m")
                .expect("the member call is a reference");
            assert!(call.1.ends_with("T·m·item"), "new T() types the binding: {call:?}");
        }
    }

    /// Route 2 of 3 — a TypeScript annotation on the binding.
    #[test]
    fn a_type_annotation_gives_the_receiver_its_type() {
        let facts = facts(
            "class T { m() {} }\n\
             export function go(make: () => T) { const x: T = make(); x.m(); }\n",
        );
        let call = member_targets(&facts)
            .into_iter()
            .find(|(name, _)| name == "m")
            .expect("the member call is a reference");
        assert!(call.1.ends_with("T·m·item"), "the annotation types the binding: {call:?}");
    }

    /// Route 3 of 3 — a TypeScript signature. The parameter is in scope from
    /// the body's first statement, unlike a `const`.
    #[test]
    fn a_signature_gives_a_parameter_its_type() {
        let facts = facts("class T { m() {} }\nexport function go(x: T) { x.m(); }\n");
        let call = member_targets(&facts)
            .into_iter()
            .find(|(name, _)| name == "m")
            .expect("the member call is a reference");
        assert!(call.1.ends_with("T·m·item"), "the signature types the parameter: {call:?}");
    }

    /// `this` inside a class body is that class, which is the one receiver the
    /// Rust walk could type before any of this existed.
    #[test]
    fn this_inside_a_class_is_that_class() {
        let facts = js_facts("class T { m() {} go() { this.m(); } }\n");
        let call = member_targets(&facts)
            .into_iter()
            .find(|(name, _)| name == "m")
            .expect("the member call is a reference");
        assert!(call.1.ends_with("T·m·item"), "`this` is the enclosing class: {call:?}");
    }

    /// A union states that the value is sometimes something else, so it types
    /// nothing — except the `| null` shape, which states one type and its
    /// absence.
    #[test]
    fn a_union_of_two_real_types_names_neither_and_a_nullable_names_one() {
        assert_eq!(type_segment("Widget | null").as_deref(), Ok("Widget"));
        assert_eq!(type_segment("Widget | undefined").as_deref(), Ok("Widget"));
        assert_eq!(type_segment("Widget<T>").as_deref(), Ok("Widget"));
        assert_eq!(type_segment("ns.Widget").as_deref(), Ok("Widget"));
        assert_eq!(type_segment("readonly Widget").as_deref(), Ok("Widget"));
        for raw in ["Widget | Gadget", "Widget[]", "{ a: number }", "'literal'", "", "[A, B]"] {
            assert!(
                matches!(type_segment(raw), Err(FqnError::NotATypeName { .. })),
                "`{raw}` names no single type"
            );
        }
    }

    // ── 04b §3: what JavaScript has that Rust does not ───────────────────────

    /// A namespace import binds a MODULE, not a type, so `api.thing()` reaches
    /// one of that module's exports. Minting `api·thing` as a type member would
    /// put a member on a thing that has none.
    ///
    /// MUTATION: resolve it as a method — the candidate then carries the
    /// `Member` form and points at a type called `api` that nothing declares.
    #[test]
    fn a_namespace_imports_member_is_an_export_and_not_a_type_member() {
        let facts = facts("import * as api from './api';\nexport function go() { api.thing(); }\n");
        let call = member_targets(&facts)
            .into_iter()
            .find(|(name, _)| name == "thing")
            .expect("the member call is a reference");
        assert_eq!(
            call.1, "Unplaced",
            "a namespace member is placed by the ladder through the import, not typed here"
        );
        let namespace = facts.imports.iter().find(|i| i.path == "./api").expect("the import");
        assert_eq!(namespace.binds, Binding::Name("api".to_string()));
    }

    /// `const { go } = await import('./api')` is an IMPORT, and binds exactly
    /// what the static clause binds.
    ///
    /// The specifier is a literal written in the source, so this asks nothing
    /// of the ladder that a static import does not — externality still comes
    /// from the string, never from absence (R6). What makes it worth reading is
    /// that nothing else can: the test that needs a module mocked before it
    /// loads has no other way to spell the import, so the subject's whole test
    /// suite goes dark.
    ///
    /// MEASURED over this repository: 195 dynamic imports in 89 first-party
    /// files, and the five TypeScript functions whose declaration a use site
    /// named by EXACT identity while the edge stayed missing are all this one
    /// shape — `resolveTenantAccess`, `principalIdForSession`, `listUserOrgs`,
    /// `getUserOrg`, `provisionOnSignIn`.
    #[test]
    fn a_destructured_dynamic_import_binds_its_members_like_a_static_clause() {
        let facts = facts(
            "const { go, stop: halt } = await import('./api');\nexport function run() { go(); }\n",
        );
        let bound: Vec<&Binding> =
            facts.imports.iter().filter(|i| i.path == "./api").map(|i| &i.binds).collect();
        assert_eq!(
            bound,
            vec![
                &Binding::MemberOf { local: "go".to_string(), member: "go".to_string() },
                // The ALIAS keeps both halves, for the reason `Binding::MemberOf`
                // records: the two names answer different questions.
                &Binding::MemberOf { local: "halt".to_string(), member: "stop".to_string() },
            ],
            "a dynamic import binds members of the module it names, exactly as the static \
             clause does"
        );
    }

    /// And a dynamic import bound to ONE name binds a module, not a member.
    ///
    /// The same distinction `import * as api` draws, reached by the other
    /// spelling — so `api.thing()` has to stay an export of that module rather
    /// than becoming a member of a type called `api`.
    #[test]
    fn a_dynamic_import_bound_to_one_name_is_a_namespace() {
        let facts =
            facts("const api = await import('./api');\nexport function go() { api.thing(); }\n");
        let whole = facts.imports.iter().find(|i| i.path == "./api").expect("the import");
        assert_eq!(whole.binds, Binding::Name("api".to_string()));
        let call = member_targets(&facts)
            .into_iter()
            .find(|(name, _)| name == "thing")
            .expect("the member call is a reference");
        assert_eq!(
            call.1, "Unplaced",
            "the ladder places it through the import; it is not a member of a type called `api`"
        );
    }

    /// The module path keeps its trailing `index`, and the doc on
    /// [`module_path`] says why: dropping it collides two real files onto one
    /// identity.
    #[test]
    fn a_module_path_drops_the_extension_and_nothing_else() {
        assert_eq!(module_path("src/lib/store.ts", "."), "lib/store");
        assert_eq!(module_path("src/index.ts", "."), "index");
        assert_eq!(module_path("src/lib/store/index.ts", "."), "lib/store/index");
        assert_eq!(module_path("src/routes/+page.svelte", "."), "routes/+page");
        assert_eq!(module_path("src/types.d.ts", "."), "types");
        assert_ne!(
            module_path("src/index.ts", "."),
            module_path("src/index/index.ts", "."),
            "two real files must not reduce to one identity"
        );
    }

    /// The reader, over a codebase NOBODY HERE WROTE.
    ///
    /// `app/`, `dojo/` and `website/` were written alongside this indexer, so
    /// agreement with them is weaker evidence than it looks: a grammar shape
    /// none of our authors happens to use is a shape the reader has never been
    /// asked about. Every defect A2 found — spread, computed writes, parameter
    /// defaults — was a shape that WAS in our code; what is not there cannot be
    /// found there.
    ///
    /// Point it at someone else's:
    ///
    /// ```text
    /// SENSEI_CORPUS=~/Work/Dayamed cargo test -p senseid --bin senseid -- \
    ///   --ignored --nocapture indexer::lang::javascript::tests::a_foreign_corpus
    /// ```
    ///
    /// SKIPPED, not failed, when the variable is unset: this is a probe a
    /// person runs against a codebase they have, and a test that demanded one
    /// would fail on every machine that does not.
    #[test]
    #[ignore]
    fn a_foreign_corpus_reads_and_its_misses_are_all_named() {
        let Ok(root) = std::env::var("SENSEI_CORPUS") else {
            println!("SENSEI_CORPUS unset — nothing to read. See this test's docs.");
            return;
        };
        let root = std::path::Path::new(&root);
        let tops: Vec<String> = std::fs::read_dir(root)
            .expect("SENSEI_CORPUS names a readable directory")
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|name| !name.starts_with('.'))
            .collect();
        let tops: Vec<&str> = tops.iter().map(String::as_str).collect();
        let sources = crate::indexer::web_sources_under(root, &tops, 1);

        let mut files = 0usize;
        let mut unreadable = 0usize;
        let mut references = 0usize;
        let mut symbols = 0usize;
        let mut walked = 0usize;
        let mut counted = 0usize;
        let mut reasons: BTreeMap<String, usize> = BTreeMap::new();

        for (path, text) in &sources {
            let ext = format!(".{}", path.rsplit('.').next().unwrap_or(""));
            let Some(adapter) = crate::indexer::lang::adapter_for_ext(&ext) else { continue };
            files += 1;
            let module = adapter.module_path(path, ".");
            let source = Source { package: "foreign", module: &module, path, text };
            let Ok(facts) = adapter.read(&source, &crate::indexer::lang::TypeHomes::unknown())
            else {
                unreadable += 1;
                continue;
            };
            symbols += facts.symbols.len();
            references += facts.references.len();
            for reference in &facts.references {
                if let Resolution::Unresolved { reason, evidence } = &reference.target {
                    *reasons.entry(format!("{reason:?}")).or_default() += 1;
                    assert!(
                        !evidence.name.is_empty(),
                        "{path}: a miss with nothing in it is one nobody can act on"
                    );
                }
            }
            // A2 over foreign code, which is the point: our own corpus cannot
            // contain a shape our own authors never wrote.
            if !path.ends_with(".svelte") {
                walked += facts
                    .references
                    .iter()
                    .filter(|r| {
                        matches!(
                            r.kind,
                            RefKind::Calls | RefKind::Constructs | RefKind::Reads | RefKind::Writes
                        )
                    })
                    .count();
                counted += count_use_sites(text, path).len();
            }
        }

        println!("\n## A foreign corpus: {}\n", root.display());
        println!("files {files} | unreadable {unreadable}");
        println!("symbols {symbols} | references {references}");
        println!(
            "A2: walk {walked} | independent {counted} | dropped {}",
            counted.saturating_sub(walked)
        );
        for (reason, n) in &reasons {
            println!("  {n:>7}  {reason}");
        }

        assert!(files > 0, "no file under {} was claimed by an adapter", root.display());
        assert!(
            unreadable * 20 < files.max(1),
            "{unreadable} of {files} did not parse — over 5%, which is the reader being wrong \
             about the language rather than about a file"
        );
    }

    // ── A2: zero references dropped ──────────────────────────────────────

    /// **A2.** The count of `Reference` values equals the count of use sites in
    /// the AST, verified by a walk that counts INDEPENDENTLY.
    ///
    /// Rust has had this; the oxc side has not, so every TypeScript reference
    /// figure reported so far has been an unchecked total. A counter that
    /// called the walk would prove nothing, so this one is a
    /// [`oxc_ast_visit::Visit`] written from the other side against the same
    /// definitions.
    ///
    /// WHAT IT DELIBERATELY DOES NOT COUNT, matching the walk:
    ///
    /// - a member in CALLEE position. `a.b()` is ONE reference — the call —
    ///   because `name_callee` reads the member itself and the walk then
    ///   descends only into the OBJECT. Counting the member separately would
    ///   report a drop that is not one.
    /// - a member that is the LEFT side of an assignment, which the walk emits
    ///   as a Write from `assignment`, not twice.
    struct UseSites {
        calls: usize,
        constructs: usize,
        members: usize,
        /// Where each site was. A count says a reference was dropped; the span
        /// says which SYNTAX drops it, which is the difference between knowing
        /// there is a defect and being able to fix one.
        seen: Vec<(u32, u32)>,
        /// Spans of members the walk reads as part of something else, so they
        /// are not counted a second time.
        absorbed: BTreeSet<(u32, u32)>,
    }

    impl<'a> oxc_ast_visit::Visit<'a> for UseSites {
        fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
            self.calls += 1;
            self.seen.push((call.span.start, call.span.end));
            // The callee is READ BY the call, whatever shape it is.
            if let Some(member) = a_member_expression(&call.callee) {
                self.absorbed.insert((member.span().start, member.span().end));
            }
            oxc_ast_visit::walk::walk_call_expression(self, call);
        }

        fn visit_new_expression(&mut self, new: &NewExpression<'a>) {
            self.constructs += 1;
            self.seen.push((new.span.start, new.span.end));
            oxc_ast_visit::walk::walk_new_expression(self, new);
        }

        fn visit_static_member_expression(&mut self, member: &StaticMemberExpression<'a>) {
            if !self.absorbed.contains(&(member.span.start, member.span.end)) {
                self.members += 1;
                self.seen.push((member.span.start, member.span.end));
            }
            oxc_ast_visit::walk::walk_static_member_expression(self, member);
        }

        fn visit_computed_member_expression(&mut self, member: &ComputedMemberExpression<'a>) {
            if !self.absorbed.contains(&(member.span.start, member.span.end)) {
                self.members += 1;
                self.seen.push((member.span.start, member.span.end));
            }
            oxc_ast_visit::walk::walk_computed_member_expression(self, member);
        }

        /// `this.#count`. The THIRD member shape the grammar has, and the one
        /// this counter did not enumerate — so 167 of them were dropped by the
        /// walk and counted zero by the check that exists to catch drops.
        ///
        /// That is the lesson worth more than the 167: a counter built by
        /// listing the shapes the WALK handles cannot find a shape the walk
        /// does not handle. It has to be built from the shapes the GRAMMAR has.
        /// `oxc_ast_visit::Visit` has one method per node kind, and the three
        /// member kinds are `StaticMemberExpression`, `ComputedMemberExpression`
        /// and this — so the list above was two thirds of a closed set.
        fn visit_private_field_expression(&mut self, member: &PrivateFieldExpression<'a>) {
            if !self.absorbed.contains(&(member.span.start, member.span.end)) {
                self.members += 1;
                self.seen.push((member.span.start, member.span.end));
            }
            oxc_ast_visit::walk::walk_private_field_expression(self, member);
        }
    }

    /// Whether an expression IS a member access, for the counter's own reading
    /// of the grammar.
    ///
    /// Deliberately NOT the walk's `as_member`. A2's value is that it is an
    /// independent count, and a counter that asks the walk what counts as a
    /// member inherits the walk's blind spots — which is exactly how a shape
    /// missing from both sides subtracted to zero and looked like agreement.
    fn a_member_expression<'a, 'b>(callee: &'b Expression<'a>) -> Option<&'b Expression<'a>> {
        matches!(
            callee,
            Expression::StaticMemberExpression(_)
                | Expression::ComputedMemberExpression(_)
                | Expression::PrivateFieldExpression(_)
        )
        .then_some(callee)
    }

    /// The independent count over one source, in the dialect its path states.
    fn count_use_sites(text: &str, path: &str) -> Vec<(u32, u32)> {
        let allocator = Allocator::default();
        let source_type = SourceType::from_path(path).expect("a claimed extension");
        let parsed = Parser::new(&allocator, text, source_type).parse();
        let mut sites = UseSites {
            calls: 0,
            constructs: 0,
            members: 0,
            seen: Vec::new(),
            absorbed: BTreeSet::new(),
        };
        oxc_ast_visit::Visit::visit_program(&mut sites, &parsed.program);
        sites.seen
    }

    /// A2 over the REAL corpus, not a fixture.
    ///
    /// The two counts are produced from different code against the same
    /// definitions, so agreement means something and a divergence names either
    /// a dropped reference or a counter that has drifted from the walk. The
    /// delta is REPORTED per file rather than summed, because one file off by
    /// twenty and twenty files off by one are different defects.
    #[test]
    #[ignore]
    fn the_reference_count_equals_an_independent_count_of_use_sites() {
        let mut walked = 0usize;
        let mut counted = 0usize;
        let mut disagreed: Vec<(String, usize, usize)> = Vec::new();
        let mut missed_shapes: BTreeMap<String, usize> = BTreeMap::new();

        for (path, text) in crate::indexer::corpus_web_sources() {
            // `.svelte` is markup wrapped around script and has no single oxc
            // program; its own tests cover it. This is the plain-file check.
            if path.ends_with(".svelte") {
                continue;
            }
            let module = module_path(&path, ".");
            let Ok(facts) = read(
                &Source { package: "web", module: &module, path: &path, text: &text },
                &TypeHomes::unknown(),
            ) else {
                continue;
            };
            let ours = facts
                .references
                .iter()
                .filter(|r| {
                    matches!(
                        r.kind,
                        RefKind::Calls | RefKind::Constructs | RefKind::Reads | RefKind::Writes
                    )
                })
                .count();
            let sites = count_use_sites(&text, &path);
            let theirs = sites.len();
            walked += ours;
            counted += theirs;
            if ours != theirs {
                // The LINES the counter saw a site on that the walk emitted
                // nothing for. A count says a reference was dropped; the source
                // line says which syntax drops it.
                // A MULTISET, not a set. A first version compared sets of
                // line numbers, so three sites on one line matched one walk
                // reference and the other two read as drops — it reported 115
                // phantom misses on `const { data, error } = await db` and
                // would have sent somebody chasing a defect that is not there.
                let mut ours_at: BTreeMap<u32, usize> = BTreeMap::new();
                for reference in &facts.references {
                    *ours_at.entry(reference.at.start_line).or_default() += 1;
                }
                let lines = LineIndex::of(&text);
                for (start, end) in sites {
                    let at = lines.locate(start, end).start_line;
                    let left = ours_at.entry(at).or_default();
                    if *left > 0 {
                        *left -= 1;
                    } else {
                        let source = text.lines().nth(at as usize - 1).unwrap_or("").trim();
                        *missed_shapes
                            .entry(source.chars().take(72).collect::<String>())
                            .or_default() += 1;
                    }
                }
                disagreed.push((path.clone(), ours, theirs));
            }
        }

        println!("\n## A2 (typescript): the walk against an independent count\n");
        println!("walk {walked} | independent {counted} | files disagreeing {}", disagreed.len());
        for (path, ours, theirs) in disagreed.iter().take(5) {
            println!("  {path}: walk {ours}, independent {theirs}");
        }
        let mut ranked: Vec<(&String, &usize)> = missed_shapes.iter().collect();
        ranked.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
        println!("\nthe SOURCE the walk emitted nothing for:");
        for (source, n) in ranked.iter().take(18) {
            println!("  {n:>4}  {source}");
        }
        assert!(walked > 1_000, "only {walked} references, so this proved nothing");

        // A2 says ZERO dropped, and this is not zero: the walk is 14 short
        // across 11 of 946 files. It was 383 across 99, in three passes:
        //
        //   383 -> 152  SPREAD, in all three positions it can appear
        //   152 ->  73  a COMPUTED member as an assignment target, `x++`,
        //               an optional-chained computed member
        //    73 ->  14  a PARAMETER DEFAULT (`now: Date = new Date()`) and a
        //               COMPUTED KEY (`{ [f(x)]: false }`)
        //
        // A FOURTH pass did not move this number and is the most important of
        // the four. `PrivateFieldExpression` was missing from the walk AND from
        // the counter above, so 167 dropped references subtracted to zero and
        // read as agreement. Teaching the counter alone took the drop 14 -> 181;
        // teaching the walk took it back to 14, with both totals up by 167
        // (walk 50,092 -> 50,259). A2 cannot find a shape neither side
        // enumerates, which is why the counter is now built from the visitor's
        // node kinds rather than from the walk's arm list.
        //
        // Every one was found by reading the printout below. None was found by
        // reasoning about the grammar, and the one time a shape was guessed at
        // from the top of this list it was a phantom the diagnostic had
        // invented — see the multiset note above. Stated as a RATCHET rather than as the target,
        // for §6's reason — a gate that fails on its first run gets waived, and
        // a waived gate is not a gate. It may fall; it may not rise.
        //
        // The delta is per-file above, not just summed, because one file off by
        // twenty and twenty files off by one are different defects. The head is
        // `health-state.spec.svelte.ts` at 24, which is where to look first.
        const KNOWN_DROPPED: usize = 14;
        let dropped = counted.saturating_sub(walked);
        assert!(
            dropped <= KNOWN_DROPPED,
            "the walk dropped {dropped} references the independent count saw (ratchet \
             {KNOWN_DROPPED}) across {} files — A2 is 'zero references dropped', and a \
             reference nobody emits is one no query can ever answer for",
            disagreed.len()
        );
        assert!(
            walked <= counted,
            "the walk emitted MORE references than the independent count saw, which means one \
             use site is being emitted twice — a duplicate edge, not a missing one"
        );
    }

    // ── the real corpus (A2) ─────────────────────────────────────────────

    /// Read every JavaScript, TypeScript and Svelte file this repository's three
    /// front ends contain, through the adapter the extension dispatches to.
    ///
    /// The load-bearing check, and it must run over the REAL corpus: a fixture
    /// proves the reader handles what its author thought of, and three shipping
    /// SvelteKit apps are where the unthought-of forms live. What it asserts is
    /// what R2 promises — every file either produces facts or names why it
    /// could not, and NOTHING panics on the way.
    #[test]
    fn every_real_file_reads_without_panicking_and_says_what_it_saw() {
        use crate::indexer::lang::adapter_for_ext;

        let mut files = 0usize;
        let mut symbols = 0usize;
        let mut references = 0usize;
        let mut relations = 0usize;
        let mut imports = 0usize;
        let mut unreadable: Vec<(String, ReadError)> = Vec::new();
        let mut by_reason: BTreeMap<String, usize> = BTreeMap::new();

        for (path, text) in crate::indexer::corpus_web_sources() {
            let ext = format!(".{}", path.rsplit('.').next().unwrap_or(""));
            let adapter =
                adapter_for_ext(&ext).unwrap_or_else(|| panic!("{path}: nothing claims {ext}"));
            let module = adapter.module_path(&path, ".");
            let source = Source { package: "web", module: &module, path: &path, text: &text };
            files += 1;
            match adapter.read(&source, &TypeHomes::unknown()) {
                Ok(facts) => {
                    symbols += facts.symbols.len();
                    references += facts.references.len();
                    relations += facts.relations.len();
                    imports += facts.imports.len();
                    for reference in &facts.references {
                        if let Resolution::Unresolved { reason, .. } = &reference.target {
                            *by_reason.entry(format!("{reason:?}")).or_default() += 1;
                        }
                    }
                }
                // A file the reader cannot open is a FACT, collected and
                // reported — never an empty `FileFacts` and never a panic.
                Err(e) => unreadable.push((path.clone(), e)),
            }
        }

        println!("\n── the JS/TS/Svelte corpus, {files} files ──");
        println!(
            "symbols {symbols} | references {references} | relations {relations} | imports {imports}"
        );
        println!("unreadable {}", unreadable.len());
        for (reason, n) in &by_reason {
            println!("  {n:>6}  {reason}");
        }

        assert!(symbols > 1_000, "three front ends declare more than {symbols} things");
        assert!(references > 1_000, "three front ends use more than {references} things");
        assert!(imports > 500, "three front ends import more than {imports} times");
        assert!(
            relations > 0,
            "`extends`/`implements` are emitted from the same walk (D5), and none appeared"
        );
        // A handful of files may genuinely not parse — a `.svelte` using syntax
        // this build's oxc does not know. What must not happen is a SHARE of
        // them, which would mean the reader is wrong about the language rather
        // than about one file.
        let share = unreadable.len() * 100 / files.max(1);
        assert!(
            share < 5,
            "{} of {files} files did not read ({share}%): {:?}",
            unreadable.len(),
            unreadable.iter().take(5).collect::<Vec<_>>()
        );
    }

    /// The receiver-typing routes, MEASURED over the corpus rather than
    /// asserted over a fixture.
    ///
    /// 04b S3 sized three routes at 1,736 of 6,406 member calls (27%) before
    /// any of them was built, and the lesson it records — paid for three times
    /// on the Rust side — is that a route built without sizing it first reaches
    /// almost nothing. This is the other end of that: what the routes ACTUALLY
    /// reached, printed, so the next route is chosen against a number.
    #[test]
    fn the_stated_routes_type_a_measurable_share_of_real_receivers() {
        use crate::indexer::lang::adapter_for_ext;

        let mut typed = 0usize;
        let mut unknown = 0usize;
        for (path, text) in crate::indexer::corpus_web_sources() {
            let ext = format!(".{}", path.rsplit('.').next().unwrap_or(""));
            let Some(adapter) = adapter_for_ext(&ext) else { continue };
            let module = adapter.module_path(&path, ".");
            let Ok(facts) = adapter.read(
                &Source { package: "web", module: &module, path: &path, text: &text },
                &TypeHomes::unknown(),
            ) else {
                continue;
            };
            for reference in &facts.references {
                match &reference.target {
                    Resolution::Unresolved { reason: Reason::ReceiverTypeUnknown, .. } => {
                        unknown += 1
                    }
                    // A member the walk NAMED carries the candidate it minted,
                    // which it can only mint once it has a receiver type.
                    Resolution::Unresolved { evidence, .. }
                        if evidence.reach == Reach::Field
                            && evidence
                                .saw
                                .iter()
                                .any(|o| matches!(o, Observation::Candidate(_))) =>
                    {
                        typed += 1
                    }
                    _ => {}
                }
            }
        }
        let receivers = typed + unknown;
        println!("\nreceivers typed by a STATED route: {typed} of {receivers}");
        assert!(receivers > 500, "only {receivers} member receivers in three front ends?");
        assert!(
            typed > 0,
            "not one receiver was typed over the whole corpus, so the routes reach nothing real"
        );
    }

    /// A file identity is the module it declares, minted the way a reference to
    /// the module would mint it.
    #[test]
    fn a_file_identity_is_the_module_it_declares() {
        assert_eq!(
            file_fqn("pkg", "lib/store", "src/lib/store.ts").map(|f| f.to_string()).as_deref(),
            Ok("typescript·pkg·lib·store·mod")
        );
        assert_eq!(
            file_fqn("pkg", "index", "src/index.ts").map(|f| f.to_string()).as_deref(),
            Ok("typescript·pkg·index·mod")
        );
        assert!(
            file_fqn("pkg", "", "src/index.ts").is_err(),
            "a file with no module path has no identity, and inventing one is fabrication"
        );
    }
    /// A member of a type NOTHING FIRST-PARTY DECLARES is the boundary, and the
    /// walk mints no identity for it.
    ///
    /// The receiver is known here — `Record<string, string>` is written down —
    /// and it is the language's, not ours. Filing `hooks` under the reading
    /// file's own module invents a first-party member of a type this scan does
    /// not declare, which is the exact fabrication the Rust walk stopped doing
    /// at 6,109 sites (R4). MEASURED in TypeScript: 938 field reads minted one,
    /// 644 of them on `Record` alone.
    ///
    /// MUTATION: answer `Home::NotOurs` with the use site's module and a
    /// candidate appears in place of the reason.
    #[test]
    fn a_receiver_of_a_type_no_first_party_declares_mints_no_member_on_it() {
        let facts = read_in(
            "lib/fixture",
            "src/lib/fixture.ts",
            "export function go(r: Record<string, string>): string { return r.hooks; }\n",
            &TypeHomes::unknown(),
        );
        let hooks: Vec<String> = targets(&facts)
            .into_iter()
            .filter(|(name, _)| name == "hooks")
            .map(|(_, t)| t)
            .collect();
        assert_eq!(
            hooks,
            vec!["ExternalBoundary".to_string()],
            "the receiver IS typed and the type is outside, so this is the boundary and not a \
             first-party member nobody declared"
        );
    }

    /// TWO first-party modules answering to one type name is not one home, and
    /// picking the first is a coin toss recorded as a fact.
    ///
    /// The negative half of the rung above: the table can say `Ours`, and it can
    /// also say it knows two places. A walk that collapses the second into the
    /// first mints a member on whichever module sorted earlier, and the edge
    /// points at a real node that is the wrong one (R4).
    ///
    /// MUTATION: fold `Home::Ambiguous` in with `Home::Ours` and a candidate
    /// naming one of the two `Widget`s appears here.
    #[test]
    fn two_first_party_modules_answering_to_one_type_name_mint_no_member() {
        let unknown = TypeHomes::unknown();
        let a = read_in("lib/a", "src/lib/a.ts", "export class Widget { m() {} }\n", &unknown);
        let b = read_in("lib/b", "src/lib/b.ts", "export class Widget { m() {} }\n", &unknown);
        let homes = TypeHomes::of(a.symbols.iter().chain(b.symbols.iter()).map(|s| ("pkg", s)));

        let caller = read_in(
            "lib/c",
            "src/lib/c.ts",
            "export function go(w: Widget): void { w.m(); }\n",
            &homes,
        );
        let m: Vec<String> =
            targets(&caller).into_iter().filter(|(name, _)| name == "m").map(|(_, t)| t).collect();
        assert_eq!(
            m,
            vec!["AmbiguousCandidates".to_string()],
            "two homes is a different answer from one home, and the walk must not resolve it \
             by picking"
        );
    }

    /// A type the file declares ITSELF is placed by the file, whatever a later
    /// barrier says — and that is what keeps every single-file fixture, and the
    /// first of the scan's two passes, working.
    ///
    /// The type table is a BARRIER artifact built from a completed pass, so the
    /// pass that builds it is handed an empty one. Without the file's own answer
    /// first, every locally declared type would read as external on that pass
    /// and the table would be built from a walk that had already given up.
    ///
    /// MUTATION: drop the file's own declarations from `home_of` and this goes
    /// red with `ExternalBoundary` while the table is empty.
    #[test]
    fn a_type_this_file_declares_is_its_own_home_even_with_no_table() {
        let facts = read_in(
            "lib/fixture",
            "src/lib/fixture.ts",
            "export function go(): void { const w = new Widget(); w.m(); }\n\
             export class Widget { m(): void {} }\n",
            &TypeHomes::unknown(),
        );
        let m: Vec<String> =
            targets(&facts).into_iter().filter(|(name, _)| name == "m").map(|(_, t)| t).collect();
        assert_eq!(
            m,
            vec!["typescript\u{b7}pkg\u{b7}lib/fixture\u{b7}Widget\u{b7}m\u{b7}item".to_string()],
            "the nearest home there is, and it is read before the table is consulted"
        );
    }
}
