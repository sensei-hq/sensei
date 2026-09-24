//! The C walk — one pass, every declaration and every use site (R1, R2).
//!
//! # Linkage decides the identity, so linkage is read first
//!
//! [`Linkage`] is the answer to "where does this live", and it comes off a
//! `storage_class_specifier` rather than off a path. `static` is the file;
//! anything else is the whole link unit. See this module's sibling for why
//! that is the design rather than a detail of it.
//!
//! # The declarator is a nest, not a name
//!
//! C wraps a name in as many layers as the type has: `char *(*handlers[4])(int)`
//! is an array of pointers to functions, and the identifier is four nodes deep.
//! [`Walk::declared_name`] unwraps them. Reading the declarator's TEXT instead
//! would put `*handlers[4])(int` in an fqn.
//!
//! # What resolves without the ladder
//!
//! A `static` declaration and every reference to it are in ONE FILE by
//! definition, so the walk resolves those itself and hands the ladder nothing.
//! Everything else is a bare identifier at external linkage, which the ladder
//! mints with an EMPTY module — the same string the definition minted, in
//! whichever file it sits. That is the linker's rule, and it is why C needs no
//! import machinery to connect a call to its definition.

use std::collections::{BTreeMap, BTreeSet};

use tree_sitter::Node;

use super::super::{LanguageAdapter, ReadError, Source, TypeHomes};
use crate::indexer::facts::{
    Binding, DeclaredType, Evidence, FileFacts, Fqn, Import, ImportOrigin, Language, Param, Reason,
    RefKind, Reference, Relation, RelationKind, Resolution, Rung, Span, Symbol, SymbolKind,
    Visibility,
};
use crate::indexer::fqn::{self, Form, FqnError, Reach};

/// Whether a `.h` is actually a C++ HEADER.
///
/// `.hpp` and `.cc` are refused by the registry, which never claims them. `.h`
/// cannot be: it is the ordinary extension for a C header AND the one grpc,
/// fmt and half of the C++ world use for theirs, so the only thing that tells
/// them apart is what is inside.
///
/// `tree-sitter-c` handed a class or a template recovers into `ERROR` nodes and
/// the walk reads structure out of the wreckage. MEASURED over 3,776 vendored
/// files: 606 intra-file collisions, and the six worst files were every one of
/// them a C++ header named `.h` — `grpcpp/impl/codegen/server_callback.h`,
/// `fmt/format-inl.h`, grpc's `metadata_batch.h`.
///
/// **LINE-ANCHORED, not a substring search.** `class` and `template` are legal
/// C identifiers, so `int template_id;` must not trip this — only a line that
/// BEGINS with one of these can be a C++ declaration. A comment line begins
/// with `*`, `/` or `#`, so the prose in a header does not reach the test
/// either.
fn is_cpp(text: &str) -> bool {
    // Each of these is syntactically impossible in C at the start of a line.
    const CPP_ONLY: &[&str] = &[
        "namespace ",
        "template<",
        "template <",
        "using namespace ",
        "public:",
        "private:",
        "protected:",
        "extern \"C++\"",
    ];
    text.lines().map(str::trim_start).any(|line| CPP_ONLY.iter().any(|kw| line.starts_with(kw)))
}

/// Read one C file.
pub fn read(source: &Source<'_>, _types: &TypeHomes) -> Result<FileFacts, ReadError> {
    // REFUSED, not approximated. Reading C++ with a C grammar means reading
    // structure out of a recovered parse, and a fact invented that way is
    // indistinguishable from one the source stated (R4).
    if is_cpp(source.text) {
        return Err(ReadError::GrammarUnavailable(format!(
            "{}: this is a C++ header, and tree-sitter-c parses C",
            source.path
        )));
    }
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_c::LANGUAGE.into())
        .map_err(|e| ReadError::GrammarUnavailable(e.to_string()))?;
    let tree = parser.parse(source.text, None).ok_or(ReadError::NotParsed)?;
    let root = tree.root_node();

    // THE ADAPTER'S OWN RULE, called rather than restated. The two sides of an
    // identity drifting apart is the failure `type_segment` is shared to avoid,
    // and a file's identity is no different.
    let file = super::CAdapter
        .file_fqn(source.package, source.module, source.path)
        .map_err(ReadError::NoFileIdentity)?;

    let mut walk = Walk {
        src: source.text,
        package: source.package,
        header: source.path.ends_with(".h"),
        module: source.module,
        symbols: Vec::new(),
        references: Vec::new(),
        relations: Vec::new(),
        imports: Vec::new(),
        conditional_macros: redefined_macros(source.text, root),
        defined_functions: defined_functions(source.text, root),
        file_scoped: BTreeMap::new(),
    };
    let its_own = file.clone();
    let scope =
        Scope { from: file, container: Container::File, locals: BTreeMap::new(), in_body: false };
    walk.children(&scope, root);

    walk.symbols
        .insert(0, super::super::common::file_module(its_own, stem_of(source.path), source.text));

    Ok(FileFacts {
        language: Language::C,
        package: source.package.to_string(),
        // The FILE PATH, which is the scope a `static` lives in. Carried on the
        // facts so the file's own identity can use it — an external-linkage
        // declaration takes an empty module regardless, which the walk applies
        // per declaration rather than reading from here.
        module: source.module.to_string(),
        path: source.path.to_string(),
        symbols: walk.symbols,
        references: walk.references,
        relations: walk.relations,
        imports: walk.imports,
    })
}

/// The macro names a file defines more than once.
///
/// `#if X / #define M .. / #else / #define M .. / #endif` is C's spelling of
/// the shape Rust writes with `cfg` and C# with `#if` — two BODIES of one name,
/// exactly one of which survives preprocessing. Both are in the codebase, which
/// is what this indexer describes.
fn redefined_macros(src: &str, root: Node<'_>) -> BTreeSet<String> {
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if matches!(node.kind(), "preproc_def" | "preproc_function_def")
            && let Some(name) = node.child_by_field_name("name")
        {
            *seen.entry(src[name.byte_range()].to_string()).or_default() += 1;
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }
    seen.into_iter().filter(|(_, n)| *n > 1).map(|(name, _)| name).collect()
}

/// Every name a file DEFINES as a function — the ones with a body.
fn defined_functions(src: &str, root: Node<'_>) -> BTreeSet<String> {
    fn name_of<'t>(node: Node<'t>) -> Option<Node<'t>> {
        let mut at = node;
        loop {
            match at.kind() {
                "identifier" => return Some(at),
                _ => at = at.child_by_field_name("declarator")?,
            }
        }
    }
    let mut out = BTreeSet::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if node.kind() == "function_definition"
            && let Some(declarator) = node.child_by_field_name("declarator")
            && let Some(name) = name_of(declarator)
        {
            out.insert(src[name.byte_range()].to_string());
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }
    out
}

fn stem_of(path: &str) -> &str {
    let file = path.rsplit('/').next().unwrap_or(path);
    file.rsplit_once('.').map_or(file, |(head, _)| head)
}

fn span(node: Node<'_>) -> Span {
    let start = node.start_position();
    let end = node.end_position();
    Span {
        start_line: start.row as u32 + 1,
        start_col: start.column as u32 + 1,
        end_line: end.row as u32 + 1,
        end_col: end.column as u32 + 1,
    }
}

/// Where a declaration lives, which in C is what the source says about its
/// LINKAGE rather than where the file sits.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Linkage {
    /// `static` — the declaration belongs to this file and nothing outside it
    /// can name it.
    File,
    /// The default. One definition across the whole link unit, reachable from
    /// any file of the package with nothing written to bring it in.
    Unit,
}

impl Linkage {
    /// C's only visibility IS its linkage, and it is the same fact the module
    /// segment already carries. `static` is the one thing the language hides.
    fn visibility(self) -> Visibility {
        match self {
            Self::File => Visibility::Private,
            Self::Unit => Visibility::Public,
        }
    }
}

/// One declaration, ready to be emitted.
///
/// A STRUCT rather than seven positional arguments: the walk emits eight kinds
/// of declaration and every call site passed the same seven things in the same
/// order, which is a signature nobody reads at a glance and which clippy
/// refuses outright at nine.
struct Declared<'t> {
    name: &'t str,
    kind: SymbolKind,
    linkage: Linkage,
    reach: Reach,
    declared_type: DeclaredType,
    params: Vec<Param>,
    /// The node the declaration's SPAN comes from — the DECLARATOR rather than
    /// the whole statement when one statement declares several.
    at: Node<'t>,
}

/// What a declaration is nested inside. C nests members in a struct or union
/// and nothing else — there is no type inside a type, and no namespace.
#[derive(Clone)]
enum Container {
    File,
    Type { name: String },
}

#[derive(Clone)]
struct Scope {
    /// The symbol a use site inside this scope belongs to.
    from: Fqn,
    container: Container,
    /// Name -> the type WRITTEN for it: a parameter, a local with a stated
    /// type. C states a type at every binding, so this is a lookup and never an
    /// inference.
    locals: BTreeMap<String, String>,
    /// Whether this scope is inside a function BODY.
    ///
    /// A declaration in a body is a LOCAL, and a local is not a node — the rule
    /// every adapter here has needed. `barrier::a_node` already excludes its
    /// kind from every coverage measurement, so the row was one nothing read.
    /// It is still recorded in `locals`, because that is what types the
    /// receiver of `p.x`.
    in_body: bool,
}

struct Walk<'a> {
    src: &'a str,
    package: &'a str,
    /// Whether this file is a HEADER.
    ///
    /// The answer to "does a no-linkage declaration belong to this file alone".
    /// A typedef, a tag, an enum constant and a macro are private to the
    /// TRANSLATION UNIT — and a translation unit is a `.c` plus everything it
    /// includes, so a header's are shared with every includer and a `.c`'s are
    /// not. See this module's sibling.
    header: bool,
    /// The file's own path, minus its extension. The module a `static`
    /// declaration is minted under.
    module: &'a str,
    symbols: Vec<Symbol>,
    references: Vec<Reference>,
    relations: Vec<Relation>,
    imports: Vec<Import>,
    /// The macro names this file defines MORE THAN ONCE.
    ///
    /// Whether a definition is one arm of a conditional is a property of its
    /// SIBLINGS, and the walk meets them one at a time — so the file is scanned
    /// once, up front. A set of one is not a conditional: splitting every macro
    /// into a callable plus a single arm would double the node count and say
    /// nothing.
    conditional_macros: BTreeSet<String>,
    /// Every name this file DEFINES as a function.
    ///
    /// C's own precedence rule, which the grammar alone cannot give: a
    /// `declaration` of a name this file also defines is a PROMISE, whatever
    /// shape its declarator takes. `static JNI_ContextLoaderUpdater _noop;`
    /// forward-declares a function through a typedef'd function TYPE, so its
    /// declarator is a bare identifier and `declares_a_function` cannot see it
    /// — the definition further down the file is what says so.
    defined_functions: BTreeSet<String>,
    /// Every name this file declares `static`, and the identity it was minted
    /// at.
    ///
    /// THE WHOLE of C's cross-reference problem that the ladder cannot help
    /// with. A `static` and every reference to it are in one file by the
    /// language's own rule, so the walk answers for them and hands the ladder
    /// nothing — and a reference to a name NOT in here is external by
    /// elimination, which is the default the ladder already mints.
    file_scoped: BTreeMap<String, Fqn>,
}

impl<'a> Walk<'a> {
    fn text(&self, node: Node<'_>) -> &'a str {
        &self.src[node.byte_range()]
    }

    fn field_text(&self, node: Node<'_>, name: &str) -> Option<&'a str> {
        node.child_by_field_name(name).map(|c| self.text(c))
    }

    /// Where a declaration with NO LINKAGE lives.
    ///
    /// A typedef, a tag, an enum constant and a macro are private to the
    /// translation unit. A header's translation unit is every file that
    /// includes it; a `.c`'s is itself.
    fn no_linkage(&self) -> Linkage {
        if self.header { Linkage::Unit } else { Linkage::File }
    }

    /// `static`, or not.
    ///
    /// Read off the declaration's own `storage_class_specifier` children.
    /// `extern` and `auto` and `register` all mean external or automatic, and
    /// neither is file scope — only `static` is.
    fn linkage(&self, node: Node<'_>) -> Linkage {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "storage_class_specifier" && self.text(child).trim() == "static" {
                return Linkage::File;
            }
        }
        Linkage::Unit
    }

    /// Whether a declaration is a PROMISE rather than a definition.
    ///
    /// `extern int errno;` says a definition exists elsewhere, exactly as a
    /// function prototype does. Minting a symbol for it would put two
    /// declarations on one identity for every `extern` in every header.
    fn is_a_promise(&self, node: Node<'_>) -> bool {
        let mut cursor = node.walk();
        node.children(&mut cursor)
            .any(|c| c.kind() == "storage_class_specifier" && self.text(c).trim() == "extern")
    }

    /// The identifier a C declarator finally names.
    ///
    /// C wraps the name in one layer per level of the type — pointer, array,
    /// function, parenthesis — so this unwraps rather than reads text. The
    /// `declarator` FIELD is the way down through every wrapper the grammar
    /// has.
    fn declared_name<'t>(&self, declarator: Node<'t>) -> Option<Node<'t>> {
        let mut node = declarator;
        loop {
            match node.kind() {
                "identifier" | "type_identifier" | "field_identifier" => return Some(node),
                _ => match node.child_by_field_name("declarator") {
                    Some(inner) => node = inner,
                    // A PARENTHESISED declarator states no field: `(*fp)(int)`
                    // wraps the pointer in plain parentheses. Falling through to
                    // the first named child is what reaches the name inside.
                    None => {
                        let mut cursor = node.walk();
                        let inner = node.named_children(&mut cursor).find(|c| {
                            matches!(
                                c.kind(),
                                "identifier"
                                    | "type_identifier"
                                    | "field_identifier"
                                    | "parenthesized_declarator"
                                    | "pointer_declarator"
                                    | "array_declarator"
                                    | "function_declarator"
                            )
                        })?;
                        node = inner;
                    }
                },
            }
        }
    }

    /// Whether a declarator declares a FUNCTION — a prototype rather than a
    /// variable.
    fn declares_a_function(&self, declarator: Node<'_>) -> bool {
        let mut node = declarator;
        loop {
            match node.kind() {
                "function_declarator" => return true,
                "identifier" | "type_identifier" | "field_identifier" => return false,
                _ => match node.child_by_field_name("declarator") {
                    Some(inner) => node = inner,
                    None => return false,
                },
            }
        }
    }

    /// Mint the identity of a declaration, with LINKAGE deciding the module.
    fn declare(
        &self,
        scope: &Scope,
        name: &str,
        linkage: Linkage,
        reach: Reach,
    ) -> Result<Fqn, FqnError> {
        let lang = Language::C;
        let package = self.package;
        // THE ONE PLACE the design lives. `static` is the file; everything else
        // is the link unit, which is the empty module the ladder also mints for
        // a bare name.
        let module = match linkage {
            Linkage::File => self.module,
            Linkage::Unit => "",
        };
        match &scope.container {
            Container::File => fqn::define(&Form::Item { lang, package, module, name, reach }),
            // A struct's members are reached THROUGH it, so they hang off the
            // type wherever the type lives.
            Container::Type { name: ty } => {
                fqn::define(&Form::Member { lang, package, module, ty, member: name, reach })
            }
        }
    }

    fn owned_by_the_enclosing_type(&mut self, scope: &Scope, child: &Fqn, at: Span) {
        let kind = match &scope.container {
            Container::Type { .. } => RelationKind::Owns,
            Container::File => RelationKind::Contains,
        };
        self.relations.push(Relation {
            kind,
            child: child.clone(),
            parent: Resolution::Resolved { fqn: scope.from.clone(), via: Rung::DeclaredHere },
            at,
        });
    }

    /// A `Result`, NOT an `Option`: the only way this fails is
    /// `fqn::define` refusing a segment, and that is a typed error the caller
    /// can report. `Option` would throw the reason away, and a walk that
    /// cannot say WHY it declined to declare something is a walk nobody can
    /// debug (R2).
    fn push(&mut self, scope: &Scope, declared: Declared<'_>) -> Result<Fqn, FqnError> {
        let Declared { name, kind, linkage, reach, declared_type, params, at } = declared;
        let fqn = self.declare(scope, name, linkage, reach)?;
        if linkage == Linkage::File {
            self.file_scoped.insert(name.to_string(), fqn.clone());
        }
        self.symbols.push(Symbol {
            fqn: fqn.clone(),
            kind,
            name: name.to_string(),
            span: span(at),
            visibility: linkage.visibility(),
            docstring: None,
            declared_type,
            params,
        });
        self.owned_by_the_enclosing_type(scope, &fqn, span(at));
        Ok(fqn)
    }

    // ── includes ─────────────────────────────────────────────────────────────

    /// `#include "x.h"` and `#include <stdio.h>`.
    ///
    /// Both bind a GLOB, because that is what textual inclusion does: every
    /// name the header declares becomes visible, and nothing in the including
    /// file says which ones. The quoted form is LOCAL and resolved against this
    /// file's directory; the angled form names a library.
    fn include(&mut self, node: Node<'_>) {
        let Some(path) = node.child_by_field_name("path") else { return };
        let raw = self.text(path);
        let system = path.kind() == "system_lib_string";
        let spelled = raw.trim_matches(['"', '<', '>'].as_ref()).to_string();
        self.imports.push(Import {
            path: spelled,
            binds: Binding::Glob,
            origin: if system {
                // `libc` names the C standard library, which is what an angled
                // include of a standard header reaches. A project's own angled
                // include (`-I` on the command line) is not distinguishable
                // here, and naming it `libc` is the honest read of the only
                // thing the source says.
                ImportOrigin::External { package: "libc".to_string() }
            } else {
                ImportOrigin::Local
            },
            at: span(node),
        });
    }

    // ── declarations ─────────────────────────────────────────────────────────

    fn children(&mut self, scope: &Scope, node: Node<'_>) {
        let mut cursor = node.walk();
        let kids: Vec<Node<'_>> = node.children(&mut cursor).collect();
        for child in kids {
            self.node(scope, child);
        }
    }

    fn node(&mut self, scope: &Scope, node: Node<'_>) {
        match node.kind() {
            "preproc_include" => self.include(node),
            "preproc_def" | "preproc_function_def" => self.macro_definition(scope, node),
            "function_definition" => self.function_definition(scope, node),
            "declaration" => self.declaration(scope, node),
            "type_definition" => self.type_definition(scope, node),
            // A UNION IS A STRUCT whose members share storage. `SymbolKind`
            // has no variant for the distinction and inventing one would put a
            // C-only kind on every language's vocabulary; the overlap is a
            // property of the type, not a different kind of declaration.
            "struct_specifier" | "union_specifier" => {
                self.record_specifier(scope, node, SymbolKind::Struct)
            }
            "enum_specifier" => self.record_specifier(scope, node, SymbolKind::Enum),
            "field_declaration" => self.field_declaration(scope, node),
            // A BLOCK, walked with a scope that GROWS as each declaration is
            // met — a local declared on line 3 types a receiver on line 4 and
            // not one on line 2.
            "compound_statement" => self.body(scope, node),
            _ => {
                self.expression(scope, node);
                self.children(scope, node);
            }
        }
    }

    /// The preprocessor condition a node sits under, as the SOURCE wrote it.
    ///
    /// Climbed rather than threaded, because a `#define` may sit any number of
    /// blocks deep and only the innermost one discriminates it from its
    /// sibling. The `#else` branch is the negation of its own `#if`, which is
    /// the only way the two arms of a two-arm conditional can be told apart —
    /// the `#else` states no condition of its own.
    fn preproc_condition(&self, node: Node<'_>) -> Option<String> {
        let mut at = node.parent();
        while let Some(parent) = at {
            let spelled = match parent.kind() {
                "preproc_ifdef" => parent.child_by_field_name("name").map(|n| {
                    // `#ifndef` and `#ifdef` are the same node kind, told apart
                    // by the directive TEXT. Reading only the name would give
                    // two arms one condition, which is the collision this is
                    // here to remove.
                    let negated = self.text(parent).trim_start().starts_with("#ifndef");
                    let name = self.text(n);
                    if negated { format!("!defined {name}") } else { format!("defined {name}") }
                }),
                "preproc_if" | "preproc_elif" => parent
                    .child_by_field_name("condition")
                    .map(|c| self.text(c).split_whitespace().collect::<Vec<_>>().join(" ")),
                // An `#else` is the NEGATION of the branch it follows, and the
                // grammar nests it INSIDE that branch — so the condition is one
                // more climb up.
                "preproc_else" => {
                    let owner = parent.parent()?;
                    let inner = self.preproc_condition_of(owner)?;
                    Some(format!("!({inner})"))
                }
                _ => None,
            };
            if let Some(spelled) = spelled {
                return Some(format!("if:{spelled}"));
            }
            at = parent.parent();
        }
        None
    }

    /// The condition a conditional node states about ITSELF, unprefixed.
    fn preproc_condition_of(&self, node: Node<'_>) -> Option<String> {
        match node.kind() {
            "preproc_ifdef" => node.child_by_field_name("name").map(|n| {
                let negated = self.text(node).trim_start().starts_with("#ifndef");
                let name = self.text(n);
                if negated { format!("!defined {name}") } else { format!("defined {name}") }
            }),
            "preproc_if" | "preproc_elif" => node
                .child_by_field_name("condition")
                .map(|c| self.text(c).split_whitespace().collect::<Vec<_>>().join(" ")),
            _ => None,
        }
    }

    /// Split a CONDITIONALLY REDEFINED macro into the callable a use site mints
    /// and the arm that this branch defines.
    ///
    /// The same shape `rust::walk::split_into_variant` gives a `cfg`-gated
    /// declaration, and for the same reason: two bodies of one name, told apart
    /// by the thing the SOURCE wrote. There it is the `cfg`; here it is the
    /// preprocessor condition.
    ///
    /// A FREE ITEM'S ARM NEEDS NO NEW FORM — `Form::Member { ty: <name>,
    /// member: <condition> }` uses the slot the item form leaves spare, which
    /// is exactly what rust does for a gated free `fn`.
    ///
    /// Returned UNTOUCHED when the name is defined once, or when the source
    /// states no condition: a `#define`/`#undef`/`#define` sequence has two
    /// bodies and nothing to tell them apart, and inventing a discriminator
    /// would be a fact the file does not carry.
    fn split_into_variant(
        &mut self,
        scope: &Scope,
        node: Node<'_>,
        name: &str,
        linkage: Linkage,
    ) -> Option<String> {
        if !self.conditional_macros.contains(name) {
            return None;
        }
        if !matches!(scope.container, Container::File) {
            return None;
        }
        let condition = self.preproc_condition(node)?;
        let module = match linkage {
            Linkage::File => self.module,
            Linkage::Unit => "",
        };
        let arm = fqn::define(&Form::Member {
            lang: Language::C,
            package: self.package,
            module,
            ty: name,
            member: &condition,
            reach: Reach::Item,
        })
        .ok()?;
        // THE CALLABLE, once. A second arm finds it already emitted and adds
        // only its own relation — which is what takes the set off A7's list.
        let callable = self.declare(scope, name, linkage, Reach::Item).ok()?;
        if !self.symbols.iter().any(|s| s.fqn == callable) {
            self.symbols.push(Symbol {
                fqn: callable.clone(),
                kind: SymbolKind::Macro,
                name: name.to_string(),
                span: span(node),
                visibility: linkage.visibility(),
                docstring: None,
                declared_type: DeclaredType::Unstated,
                params: Vec::new(),
            });
            self.owned_by_the_enclosing_type(scope, &callable, span(node));
        }
        self.relations.push(Relation {
            kind: RelationKind::Variant,
            child: arm.clone(),
            parent: Resolution::Resolved { fqn: callable, via: Rung::DeclaredHere },
            at: span(node),
        });
        Some(condition)
    }

    /// `#define MAX 10` and `#define MIN(a, b) ...`.
    ///
    /// A macro is visible from its definition to the end of the translation
    /// unit AND into every file that includes the header, so it has unit
    /// linkage — the same as a non-`static` function, and for the same reason.
    fn macro_definition(&mut self, scope: &Scope, node: Node<'_>) {
        let Some(name) = self.field_text(node, "name") else { return };
        let linkage = self.no_linkage();
        // A CONDITIONALLY REDEFINED macro is a callable plus one arm per
        // branch. `#if X / #define M .. / #else / #define M .. / #endif` is two
        // bodies of one name, and merging them would lose one.
        if let Some(condition) = self.split_into_variant(scope, node, name, linkage) {
            let module = match linkage {
                Linkage::File => self.module,
                Linkage::Unit => "",
            };
            if let Ok(arm) = fqn::define(&Form::Member {
                lang: Language::C,
                package: self.package,
                module,
                ty: name,
                member: &condition,
                reach: Reach::Item,
            }) {
                self.symbols.push(Symbol {
                    fqn: arm,
                    kind: SymbolKind::Macro,
                    name: name.to_string(),
                    span: span(node),
                    visibility: match linkage {
                        Linkage::File => Visibility::Private,
                        Linkage::Unit => Visibility::Public,
                    },
                    docstring: None,
                    declared_type: DeclaredType::Unstated,
                    params: Vec::new(),
                });
            }
            return;
        }
        // AN EXPLICIT DISCARD, and it says what an error means: `push` fails
        // only when `fqn::define` refuses a segment, so the declaration has no
        // identity and cannot be a node. Skipping it is the same answer every
        // other adapter gives — what must not happen is the `Result` vanishing
        // without a reader knowing the case exists.
        let _ = self.push(
            scope,
            Declared {
                name,
                kind: SymbolKind::Macro,
                linkage,
                reach: Reach::Item,
                declared_type: DeclaredType::Unstated,
                params: Vec::new(),
                at: node,
            },
        );
        // The BODY is not walked. A macro's replacement list is tokens, not
        // an expression — `#define LOG(x) fprintf(stderr, x)` has no call in
        // it until somebody writes `LOG(..)`, and recording one here would
        // claim a call the file never makes.
    }

    fn function_definition(&mut self, scope: &Scope, node: Node<'_>) {
        let Some(declarator) = node.child_by_field_name("declarator") else { return };
        let Some(name_node) = self.declared_name(declarator) else { return };
        let name = self.text(name_node);
        let linkage = self.linkage(node);
        let returns = match self.field_text(node, "type") {
            Some(t) => DeclaredType::Stated(t.to_string()),
            None => DeclaredType::Unstated,
        };

        let mut params = Vec::new();
        let mut inner = scope.clone();
        if let Some(list) = self.parameter_list(declarator) {
            let mut cursor = list.walk();
            let formals: Vec<Node<'_>> = list
                .children(&mut cursor)
                .filter(|p| p.kind() == "parameter_declaration")
                .collect();
            for (position, p) in formals.into_iter().enumerate() {
                let declared_type = match self.field_text(p, "type") {
                    Some(t) => DeclaredType::Stated(t.to_string()),
                    None => DeclaredType::Unstated,
                };
                self.type_use(scope, p, "type");
                // `void f(int)` states a type and NO name. The type is a real
                // use site and is recorded above; the PARAMETER is not, because
                // `Param::name` is a `String` and an empty one is a name a
                // caller cannot tell from a read that failed (R4).
                //
                // THE POSITION IS THE SOURCE'S, counted over every formal
                // including the skipped ones — deriving it from `params.len()`
                // would renumber `void f(int a, int, int c)` so that `c`
                // reported position 1.
                let Some(pname) =
                    p.child_by_field_name("declarator").and_then(|d| self.declared_name(d))
                else {
                    continue;
                };
                let pname = self.text(pname);
                if let DeclaredType::Stated(t) = &declared_type {
                    inner.locals.insert(pname.to_string(), t.clone());
                }
                params.push(Param {
                    name: pname.to_string(),
                    position: position as u32,
                    declared_type,
                });
            }
        }

        let Ok(fqn) = self.push(
            scope,
            Declared {
                name,
                kind: SymbolKind::Function,
                linkage,
                reach: Reach::Item,
                declared_type: returns,
                params,
                at: node,
            },
        ) else {
            return;
        };
        self.type_use(scope, node, "type");
        inner.from = fqn;
        if let Some(body) = node.child_by_field_name("body") {
            // `body`, NOT `children`: the block is where `in_body` turns on and
            // where locals accumulate. Dispatching the statements directly
            // skipped both, so every local in every function was a node.
            self.body(&inner, body);
        }
    }

    /// The `parameter_list` under a declarator, however many wrappers are
    /// between.
    fn parameter_list<'t>(&self, declarator: Node<'t>) -> Option<Node<'t>> {
        let mut node = declarator;
        loop {
            if node.kind() == "function_declarator" {
                return node.child_by_field_name("parameters");
            }
            node = node.child_by_field_name("declarator")?;
        }
    }

    /// A function body, walked with a scope that grows.
    ///
    /// The declarations come first because C requires them to: a name is in
    /// scope from its declaration to the end of the block, so reading them as
    /// the walk goes is the language's own rule rather than a convenience.
    fn body(&mut self, scope: &Scope, node: Node<'_>) {
        let mut inner = scope.clone();
        inner.in_body = true;
        let mut cursor = node.walk();
        let kids: Vec<Node<'_>> = node.children(&mut cursor).collect();
        for child in kids {
            if child.kind() == "declaration" {
                self.note_locals(&mut inner, child);
            }
            self.node(&inner, child);
        }
    }

    /// Record what a block's `declaration` BINDS, without declaring anything.
    fn note_locals(&self, scope: &mut Scope, node: Node<'_>) {
        let Some(ty) = self.field_text(node, "type") else { return };
        let mut cursor = node.walk();
        let declarators: Vec<Node<'_>> =
            node.children_by_field_name("declarator", &mut cursor).collect();
        for declarator in declarators {
            // A FUNCTION POINTER binds a callable, not a value with members, so
            // it is deliberately not typed here — `fp->x` would be nonsense.
            if self.declares_a_function(declarator) {
                continue;
            }
            if let Some(name) = self.declared_name(declarator) {
                scope.locals.insert(self.text(name).to_string(), ty.to_string());
            }
        }
    }

    /// A `declaration` — a prototype, an `extern`, a global, or a local.
    fn declaration(&mut self, scope: &Scope, node: Node<'_>) {
        // THE TYPE FIRST, always: `struct point { int x; } p;` declares the
        // struct AND a variable, and the struct is in the `type` field.
        let anonymous = node.child_by_field_name("type").and_then(|t| self.anonymous_aggregate(t));
        if let Some(body) = anonymous {
            let mut cursor = node.walk();
            let bound: Vec<Node<'_>> =
                node.children_by_field_name("declarator", &mut cursor).collect();
            if let Some(name) = bound.first().and_then(|d| self.declared_name(*d)) {
                let bound_to = self.text(name).to_string();
                self.anonymous_members(scope, body, &bound_to, node);
            }
        } else if let Some(ty) = node.child_by_field_name("type") {
            self.node(scope, ty);
        }
        self.type_use(scope, node, "type");

        let linkage = self.linkage(node);
        let promise = self.is_a_promise(node);
        let declared_type = match self.field_text(node, "type") {
            Some(t) => DeclaredType::Stated(t.to_string()),
            None => DeclaredType::Unstated,
        };

        let mut cursor = node.walk();
        let declarators: Vec<Node<'_>> =
            node.children_by_field_name("declarator", &mut cursor).collect();
        for declarator in declarators {
            // An `init_declarator` holds a VALUE, and the value is an
            // expression with calls in it.
            if let Some(value) = declarator.child_by_field_name("value") {
                self.node(scope, value);
            }
            let Some(name_node) = self.declared_name(declarator) else { continue };
            let name = self.text(name_node);

            if self.declares_a_function(declarator) {
                // A PROTOTYPE. `void parse(void);` promises a definition
                // elsewhere — see this module's sibling. Its parameter types are
                // still real use sites the header makes.
                if let Some(list) = self.parameter_list(declarator) {
                    let mut inner = list.walk();
                    let formals: Vec<Node<'_>> = list
                        .children(&mut inner)
                        .filter(|p| p.kind() == "parameter_declaration")
                        .collect();
                    for p in formals {
                        self.type_use(scope, p, "type");
                    }
                }
                continue;
            }
            if promise {
                // `extern int errno;` — a promise too, for the same reason.
                continue;
            }
            // A NAME THIS FILE ALSO DEFINES AS A FUNCTION. The definition is
            // the declaration; this is the forward one, however it is spelled.
            if self.defined_functions.contains(name) {
                continue;
            }

            // A LOCAL IS NOT A NODE. `Scope::in_body` says which, and the
            // binding it carries was recorded by `body` before this ran.
            if scope.in_body {
                continue;
            }

            // A TENTATIVE DEFINITION may be written more than once in one
            // file. `static PgObjectClass s_EntryClass;` at line 51 and again
            // at line 197 declares ONE object — C 6.9.2 — so one identity is
            // the correct answer and the repeat is not a second declaration.
            //
            // Only when there is NO INITIALISER. Two initialised definitions of
            // one name is an error the compiler reports, and collapsing those
            // would hide it.
            if declarator.child_by_field_name("value").is_none()
                && let Ok(already) = self.declare(scope, name, linkage, Reach::Item)
                && self.symbols.iter().any(|s| s.fqn == already)
            {
                continue;
            }

            // A file-scope variable HAS AN ADDRESS AND A LIFETIME, which is
            // exactly the distinction `SymbolKind::Static` exists for — and it
            // is true of a C global whether or not `static` is written on it.
            // The keyword decides the MODULE, above; it does not decide the
            // kind.
            if self
                .push(
                    scope,
                    Declared {
                        name,
                        kind: SymbolKind::Static,
                        linkage,
                        reach: Reach::Item,
                        declared_type: declared_type.clone(),
                        params: Vec::new(),
                        at: declarator,
                    },
                )
                .is_err()
            {
                continue;
            }
        }
    }

    /// `typedef struct node node_t;` — the alias is a declaration, and the
    /// struct inside it may be one too.
    fn type_definition(&mut self, scope: &Scope, node: Node<'_>) {
        // THE ANONYMOUS CASE FIRST, because the typedef's name is what names
        // the aggregate and only this caller has it.
        let anonymous = node.child_by_field_name("type").and_then(|t| self.anonymous_aggregate(t));
        if let Some(body) = anonymous {
            let mut cursor = node.walk();
            let bound: Vec<Node<'_>> =
                node.children_by_field_name("declarator", &mut cursor).collect();
            if let Some(name) = bound.first().and_then(|d| self.declared_name(*d)) {
                let bound_to = self.text(name).to_string();
                self.anonymous_members(scope, body, &bound_to, node);
            }
        } else if let Some(ty) = node.child_by_field_name("type") {
            self.node(scope, ty);
        }
        self.type_use(scope, node, "type");
        let mut cursor = node.walk();
        let declarators: Vec<Node<'_>> =
            node.children_by_field_name("declarator", &mut cursor).collect();
        for declarator in declarators {
            let Some(name_node) = self.declared_name(declarator) else { continue };
            let name = self.text(name_node);
            let _ = self.push(
                scope,
                Declared {
                    name,
                    kind: SymbolKind::TypeAlias,
                    linkage: self.no_linkage(),
                    reach: Reach::Item,
                    declared_type: match self.field_text(node, "type") {
                        Some(t) => DeclaredType::Stated(t.to_string()),
                        None => DeclaredType::Unstated,
                    },
                    params: Vec::new(),
                    at: declarator,
                },
            );
        }
    }

    /// The BODY of an aggregate that states no name of its own.
    ///
    /// `typedef struct { .. } point_t;` and `struct { .. } udt;` both declare a
    /// type with no tag. The thing that BINDS it is its name — the typedef, or
    /// the field — which is the same rule TypeScript needed for an anonymous
    /// function under a property.
    ///
    /// Without it, the members land in the ENCLOSING scope: two typedef'd
    /// anonymous structs in one header each with a `time` field mint one
    /// identity, and so do the outer and inner `udt` of a nested anonymous
    /// struct. Both measured in pljava.
    fn anonymous_aggregate<'t>(&self, ty: Node<'t>) -> Option<Node<'t>> {
        if !matches!(ty.kind(), "struct_specifier" | "union_specifier" | "enum_specifier") {
            return None;
        }
        if ty.child_by_field_name("name").is_some() {
            return None;
        }
        ty.child_by_field_name("body")
    }

    /// Walk an anonymous aggregate's members under the name that binds it.
    ///
    /// An ENUM is the exception, and it is C's rule rather than an omission:
    /// its constants go in the ORDINARY namespace at file scope whether the
    /// enum is named or not, so they stay where the enclosing scope puts them.
    fn anonymous_members(&mut self, scope: &Scope, body: Node<'_>, bound_to: &str, at: Node<'_>) {
        if body.kind() == "enumerator_list" {
            self.children(scope, body);
            return;
        }
        let Ok(fqn) = self.declare(scope, bound_to, self.no_linkage(), Reach::Item) else {
            return;
        };
        let _ = at;
        let inner = Scope {
            from: fqn,
            container: Container::Type { name: bound_to.to_string() },
            locals: BTreeMap::new(),
            in_body: false,
        };
        self.children(&inner, body);
    }

    /// The KEYWORD a specifier is introduced by — `struct`, `union` or `enum`.
    ///
    /// Part of the name, not decoration on it. C 6.2.3 puts tags in their own
    /// namespace, so `struct node` and a `typedef` called `node` are two names
    /// and the keyword is the only thing that tells them apart.
    fn tag_word(&self, node: Node<'_>) -> &'static str {
        match node.kind() {
            "union_specifier" => "union",
            "enum_specifier" => "enum",
            _ => "struct",
        }
    }

    /// `struct node { ... }`, `union value { ... }`, `enum color { ... }`.
    ///
    /// Only when it has a BODY. `struct node *next;` names an already-declared
    /// struct and is a use site, not a declaration, and minting a symbol for it
    /// would put one identity on every forward reference in the file.
    fn record_specifier(&mut self, scope: &Scope, node: Node<'_>, kind: SymbolKind) {
        let tag = self.tag_word(node);
        let Some(body) = node.child_by_field_name("body") else {
            // A forward declaration or a use. Either way it NAMES the tag, and
            // the name includes the keyword.
            if let Some(name) = node.child_by_field_name("name") {
                let raw = format!("{tag} {}", self.text(name));
                self.references.push(Reference {
                    from: scope.from.clone(),
                    kind: RefKind::TypeUse,
                    at: span(name),
                    target: self.refer_to(&raw, name, Reach::Item),
                });
            }
            return;
        };
        // AN ANONYMOUS struct has no name to mint. `typedef struct { int x; }
        // point;` is the ordinary C idiom for one, and the NAME is on the
        // typedef — which `type_definition` above already declared. Emitting a
        // second symbol here under an invented name would be a node no use site
        // can reach.
        let Some(name) = self.field_text(node, "name") else {
            // AN ANONYMOUS aggregate is named by what BINDS it, and only the
            // caller holding that binding can say — see `anonymous_aggregate`.
            // Reaching here means nothing binds it at all (`struct { int x; };`,
            // which declares nothing), so the members are walked where they
            // sit rather than lost.
            self.children(scope, body);
            return;
        };
        let tagged = format!("{tag} {name}");
        let Ok(fqn) = self.push(
            scope,
            Declared {
                name: &tagged,
                kind,
                linkage: self.no_linkage(),
                reach: Reach::Item,
                declared_type: DeclaredType::Unstated,
                params: Vec::new(),
                at: node,
            },
        ) else {
            return;
        };

        if kind == SymbolKind::Enum && body.kind() == "enumerator_list" {
            // AN ENUM CONSTANT IS AN ITEM, not a member — the one place C
            // differs from every other language here. `RED` is written bare at
            // file scope, never `Color.RED`, so minting it under the enum would
            // produce an identity no use site can compose.
            let mut cursor = body.walk();
            let constants: Vec<Node<'_>> =
                body.children(&mut cursor).filter(|c| c.kind() == "enumerator").collect();
            for constant in constants {
                let Some(cname) = self.field_text(constant, "name") else { continue };
                let _ = self.push(
                    scope,
                    Declared {
                        name: cname,
                        kind: SymbolKind::EnumVariant,
                        linkage: self.no_linkage(),
                        reach: Reach::Item,
                        declared_type: DeclaredType::Unstated,
                        params: Vec::new(),
                        at: constant,
                    },
                );
                // `enum flag { A = 1, B = A << 1 }` puts a reference in the
                // value.
                if let Some(value) = constant.child_by_field_name("value") {
                    self.node(scope, value);
                }
            }
            return;
        }

        let inner = Scope {
            from: fqn,
            // THE TAGGED NAME, so a field's identity is the one a
            // `struct node`-typed receiver mints. Using the bare `node` here
            // would put the declaration and every `n->id` that reads it on two
            // different strings.
            container: Container::Type { name: tagged },
            locals: BTreeMap::new(),
            in_body: false,
        };
        self.children(&inner, body);
    }

    fn field_declaration(&mut self, scope: &Scope, node: Node<'_>) {
        let anonymous = node.child_by_field_name("type").and_then(|t| self.anonymous_aggregate(t));
        if let Some(body) = anonymous {
            let mut cursor = node.walk();
            let bound: Vec<Node<'_>> =
                node.children_by_field_name("declarator", &mut cursor).collect();
            if let Some(name) = bound.first().and_then(|d| self.declared_name(*d)) {
                let bound_to = self.text(name).to_string();
                self.anonymous_members(scope, body, &bound_to, node);
            }
        } else if let Some(ty) = node.child_by_field_name("type") {
            self.node(scope, ty);
        }
        self.type_use(scope, node, "type");
        let declared_type = match self.field_text(node, "type") {
            Some(t) => DeclaredType::Stated(t.to_string()),
            None => DeclaredType::Unstated,
        };
        let mut cursor = node.walk();
        let declarators: Vec<Node<'_>> =
            node.children_by_field_name("declarator", &mut cursor).collect();
        for declarator in declarators {
            let Some(name_node) = self.declared_name(declarator) else { continue };
            let name = self.text(name_node);
            // Field reach, because `p.x` is the only shape that reaches one.
            let _ = self.push(
                scope,
                Declared {
                    name,
                    kind: SymbolKind::Field,
                    // A FIELD takes its struct's scope, which the container
                    // already decided — a member of a `.c`-local struct is as
                    // local as the struct is.
                    linkage: self.no_linkage(),
                    // Field reach: `p.x` is the only shape that reaches one.
                    reach: Reach::Field,
                    declared_type: declared_type.clone(),
                    params: Vec::new(),
                    at: declarator,
                },
            );
        }
    }

    // ── use sites ────────────────────────────────────────────────────────────

    /// Every `type_identifier` at or under a node — the types a signature
    /// NAMES.
    ///
    /// A `primitive_type` (`int`, `char`) and a `sized_type_specifier`
    /// (`unsigned long`) name no declaration and are skipped; emitting one
    /// would put a reference on a node that cannot exist.
    fn type_names_under<'t>(&self, node: Node<'t>) -> Vec<(Node<'t>, String)> {
        let mut out = Vec::new();
        let mut stack = vec![node];
        while let Some(n) = stack.pop() {
            if n.kind() == "type_identifier" {
                // A TAG CARRIES ITS KEYWORD. The identifier inside a
                // `struct_specifier` names the tag `struct node`, and the same
                // identifier standing alone names the typedef `node`. The
                // PARENT is what says which, and it is the only thing that
                // does.
                let spelled = match n.parent().map(|p| p.kind()) {
                    Some("struct_specifier") => format!("struct {}", self.text(n)),
                    Some("union_specifier") => format!("union {}", self.text(n)),
                    Some("enum_specifier") => format!("enum {}", self.text(n)),
                    _ => self.text(n).to_string(),
                };
                out.push((n, spelled));
                continue;
            }
            let mut cursor = n.walk();
            for c in n.children(&mut cursor) {
                stack.push(c);
            }
        }
        out.sort_by_key(|(n, _)| n.start_byte());
        out
    }

    fn type_use(&mut self, scope: &Scope, node: Node<'_>, field: &str) {
        let Some(ty) = node.child_by_field_name(field) else { return };
        for (named, spelled) in self.type_names_under(ty) {
            self.references.push(Reference {
                from: scope.from.clone(),
                kind: RefKind::TypeUse,
                at: span(named),
                target: self.refer_to(&spelled, named, Reach::Item),
            });
        }
    }

    fn expression(&mut self, scope: &Scope, node: Node<'_>) {
        match node.kind() {
            "call_expression" => {
                let Some(callee) = node.child_by_field_name("function") else { return };
                let target = match callee.kind() {
                    "identifier" => self.refer_to(self.text(callee), callee, Reach::Item),
                    // `handlers[i](x)` and `(*fp)(x)` call through a VALUE, and
                    // nothing written says which function it holds.
                    _ => {
                        self.missed(self.text(callee), callee, Reason::DynamicDispatch, Reach::Item)
                    }
                };
                self.references.push(Reference {
                    from: scope.from.clone(),
                    kind: RefKind::Calls,
                    at: span(node),
                    target,
                });
            }
            "field_expression" => {
                let Some(field) = node.child_by_field_name("field") else { return };
                let member = self.text(field);
                let target = match node
                    .child_by_field_name("argument")
                    .and_then(|a| self.type_of(scope, a))
                {
                    Some(ty) => self.refer_to_member(&ty, member, node),
                    None => self.missed(member, node, Reason::ReceiverTypeUnknown, Reach::Field),
                };
                self.references.push(Reference {
                    from: scope.from.clone(),
                    kind: RefKind::Reads,
                    at: span(node),
                    target,
                });
            }
            _ => {}
        }
    }

    /// The type of a receiver, when this scope STATES one. A lookup, never an
    /// inference — C writes a type at every binding.
    fn type_of(&self, scope: &Scope, node: Node<'_>) -> Option<String> {
        match node.kind() {
            "identifier" => scope.locals.get(self.text(node)).cloned(),
            _ => None,
        }
    }

    // ── minting a reference ──────────────────────────────────────────────────

    /// A bare name.
    ///
    /// Resolved HERE when this file declares it `static`, because a `static`
    /// and every reference to it are in one file by the language's own rule and
    /// the ladder has no way to know which file that was. Otherwise it goes up
    /// unplaced, and the ladder mints the empty module that external linkage
    /// means — the same string the definition minted, wherever it sits.
    fn refer_to(&self, raw: &str, at: Node<'_>, reach: Reach) -> Resolution {
        match super::type_segment(raw) {
            Ok(segment) => {
                if let Some(fqn) = self.file_scoped.get(&segment) {
                    return Resolution::Resolved { fqn: fqn.clone(), via: Rung::DeclaredHere };
                }
                self.missed(&segment, at, Reason::Unplaced, reach)
            }
            Err(_) => self.missed(raw, at, Reason::UnhandledForm, reach),
        }
    }

    /// A member of a struct this scope named.
    ///
    /// A struct tag has unit linkage, so the member's identity carries the
    /// empty module and meets the declaration wherever the struct was defined.
    fn refer_to_member(&self, ty: &str, member: &str, at: Node<'_>) -> Resolution {
        let Ok(ty) = super::type_segment(ty) else {
            return self.missed(member, at, Reason::UnhandledForm, Reach::Field);
        };
        match fqn::refer(&Form::Member {
            lang: Language::C,
            package: self.package,
            module: "",
            ty: &ty,
            member,
            // THE USE SITE'S reach, and a field read is the only shape that
            // reaches a field. Minting `Item` here would differ from the
            // declaration in exactly one segment and could never meet it.
            reach: Reach::Field,
        }) {
            Ok(fqn) => Resolution::Resolved { fqn, via: Rung::DeclaredHere },
            Err(_) => self.missed(member, at, Reason::UnhandledForm, Reach::Field),
        }
    }

    fn missed(&self, name: &str, at: Node<'_>, reason: Reason, reach: Reach) -> Resolution {
        Resolution::Unresolved {
            reason,
            evidence: Evidence {
                name: name.to_string(),
                node_kind: at.kind().to_string(),
                reach,
                saw: Vec::new(),
            },
        }
    }
}
