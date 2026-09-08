//! Indexer v2 — reading the Rust grammar (spec §7 D4, step 3 and 4 of the plan).
//!
//! This module owns one thing: how a Rust source file is READ (R7). It mints no
//! identity of its own — every fqn comes back through `fqn::define`/`fqn::refer`
//! — and it resolves nothing across files, because resolution is a shared rule
//! and a shared rule does not live in a language module.
//!
//! Decisions this walk makes about what counts as a declaration and what counts
//! as a use site are recorded next to the code that makes them, and the
//! independent counters in the tests are written against the same definitions
//! from the other side.
//
// This module has no caller on purpose — see the note in `indexer/mod.rs`.
#![allow(dead_code)]

use tree_sitter::Node;

use crate::indexer::facts::{
    Binding, DeclaredType, Evidence, FileFacts, Fqn, Import, ImportOrigin, Language, Observation,
    Param, Reason, RefKind, Reference, Relation, Resolution, Symbol, SymbolKind, Visibility,
};
use crate::indexer::fqn::{self, Form, FqnError, Ns};

/// One file, plus the two things the file cannot know about itself: which
/// package owns it and where it sits in that package's module tree. Both are
/// supplied by the processor from the manifest and the path, because a source
/// file states neither.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Source<'a> {
    pub package: &'a str,
    /// Package-relative module path, `::`-joined, empty at the crate root.
    pub module: &'a str,
    pub text: &'a str,
}

/// Why a file produced no facts at all.
///
/// Only whole-file failures live here. A single declaration or use site the
/// walk cannot read is never an error — it is a fact carrying what was seen
/// (R2, R4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadError {
    /// The tree-sitter grammar could not be loaded — a build problem, not a
    /// property of the source.
    GrammarUnavailable(String),
    /// tree-sitter returned no tree. Distinct from a tree full of ERROR nodes,
    /// which is a normal thing to walk.
    NotParsed,
    /// The file's own identity could not be minted, so nothing inside it could
    /// be named either.
    NoFileIdentity(crate::indexer::fqn::FqnError),
}

/// The name standing for a file that IS a crate root, where the module path is
/// empty and there is no `mod` declaration anywhere that names this file. It is
/// a reserved word, so no declaration can ever mint the same identity.
const CRATE_ROOT: &str = "crate";

/// Parse one file once (R1) and return everything that parse saw (spec §3).
pub fn read(source: &Source<'_>) -> Result<FileFacts, ReadError> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .map_err(|e| ReadError::GrammarUnavailable(e.to_string()))?;
    let tree = parser.parse(source.text, None).ok_or(ReadError::NotParsed)?;

    let scope = Scope {
        module: source.module.to_string(),
        container: Container::File,
        from: file_fqn(source.package, source.module).map_err(ReadError::NoFileIdentity)?,
    };
    let mut walk = Walk {
        src: source.text,
        package: source.package,
        symbols: Vec::new(),
        references: Vec::new(),
        relations: Vec::new(),
        imports: Vec::new(),
    };
    walk.children(tree.root_node(), &scope);

    Ok(FileFacts {
        language: Language::Rust,
        package: source.package.to_string(),
        module: source.module.to_string(),
        symbols: walk.symbols,
        references: walk.references,
        relations: walk.relations,
        imports: walk.imports,
    })
}

/// The identity of the file itself, which is the identity of the module it
/// declares. Minted exactly the way the `mod x;` that names this file mints it —
/// last segment as the name, everything before it as the module — so the file
/// and its declaration are one node and not two.
fn file_fqn(package: &str, module: &str) -> Result<Fqn, FqnError> {
    let (parent, name) = match module.rsplit_once("::") {
        Some((parent, name)) => (parent, name),
        None if module.is_empty() => ("", CRATE_ROOT),
        None => ("", module),
    };
    fqn::define(&Form::Item { lang: Language::Rust, package, module: parent, name, ns: Ns::Ty })
}

/// Where the walk currently is, in the terms the fqn grammar needs.
#[derive(Debug, Clone)]
struct Scope {
    /// Module path of the current container, which an inline `mod` extends.
    module: String,
    container: Container,
    /// The symbol a use site found here sits inside — [`Reference::from`].
    from: Fqn,
}

/// What a declaration found here is a member OF. This is the only thing that
/// decides which fqn form a declaration takes, so the choice is made once.
#[derive(Debug, Clone)]
enum Container {
    /// A free item, at the file level or inside an inline `mod`.
    File,
    /// A struct/enum/union body, a trait body, or an inherent `impl`.
    Type(String),
    /// An `impl Trait for Type` body. The trait qualifier is what keeps
    /// `Display::fmt` and `Debug::fmt` on one type apart.
    TraitImpl { ty: String, tr: String },
    /// An `impl` on a type with no name — a tuple, a slice, a unit. Its members
    /// have no identity in this grammar, and naming them as free items of the
    /// module would mint identities no use site could ever mint. A wrong edge is
    /// worse than a missing one (R4), so this container names nothing and the
    /// body is walked for use sites all the same.
    Unnameable { raw: String },
}

struct Walk<'a> {
    src: &'a str,
    package: &'a str,
    symbols: Vec<Symbol>,
    references: Vec<Reference>,
    relations: Vec<Relation>,
    imports: Vec<Import>,
}

impl<'a> Walk<'a> {
    fn text(&self, node: Node<'_>) -> &'a str {
        &self.src[node.byte_range()]
    }

    fn field_text(&self, node: Node<'_>, field: &str) -> Option<&'a str> {
        node.child_by_field_name(field).map(|c| self.text(c))
    }

    /// Mint the identity of a declaration named `member` in the current
    /// container. One place, so a declaration and a use site of it cannot pick
    /// different forms.
    fn declare(&self, scope: &Scope, member: &str, ns: Ns) -> Result<Fqn, FqnError> {
        let lang = Language::Rust;
        let package = self.package;
        let module = scope.module.as_str();
        match &scope.container {
            Container::File => fqn::define(&Form::Item { lang, package, module, name: member, ns }),
            Container::Type(ty) => {
                fqn::define(&Form::Member { lang, package, module, ty, member, ns })
            }
            Container::TraitImpl { ty, tr } => {
                fqn::define(&Form::TraitMember { lang, package, module, ty, tr, member, ns })
            }
            Container::Unnameable { raw } => Err(FqnError::NotATypeName { value: raw.clone() }),
        }
    }

    fn children(&mut self, node: Node<'_>, scope: &Scope) {
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.node(child, scope);
        }
    }

    /// The one dispatch. Every arm either records a declaration and walks its
    /// body, or walks children unchanged; no arm returns early without walking.
    fn node(&mut self, node: Node<'_>, scope: &Scope) {
        match node.kind() {
            // An attribute's contents are an unparsed token tree, the same as a
            // macro body: nothing inside is an expression this walk can read, so
            // claiming facts from it would be invention. The independent
            // counters skip it on the same grounds.
            "attribute_item" | "inner_attribute_item" => {}
            "use_declaration" => self.import(node),
            "extern_crate_declaration" => self.extern_crate(node),

            "function_item" | "function_signature_item" => self.function(node, scope),
            "struct_item" | "union_item" => self.type_with_fields(node, scope, SymbolKind::Struct),
            "enum_item" => self.type_with_fields(node, scope, SymbolKind::Enum),
            "trait_item" => self.type_with_fields(node, scope, SymbolKind::Trait),
            "enum_variant" => self.enum_variant(node, scope),
            "field_declaration" => self.named_field(node, scope),
            "ordered_field_declaration_list" => self.positional_fields(node, scope),
            "type_item" | "associated_type" => {
                self.plain(node, scope, SymbolKind::TypeAlias, Ns::Ty)
            }
            "const_item" => self.plain(node, scope, SymbolKind::Const, Ns::Val),
            "static_item" => self.plain(node, scope, SymbolKind::Static, Ns::Val),
            "macro_definition" => self.plain(node, scope, SymbolKind::Macro, Ns::Macro),
            "mod_item" => self.module_item(node, scope),
            "impl_item" => self.impl_block(node, scope),

            _ => {
                self.use_site(node, scope);
                self.children(node, scope);
            }
        }
    }

    /// Push a symbol and return the scope its body is walked in. Returns the
    /// scope unchanged when the declaration could not be named, so the body is
    /// still walked and nothing below it is lost.
    fn push(&mut self, symbol: Result<Symbol, FqnError>, scope: &Scope) -> Scope {
        match symbol {
            Ok(symbol) => {
                let inner = Scope { from: symbol.fqn.clone(), ..scope.clone() };
                self.symbols.push(symbol);
                inner
            }
            Err(_) => scope.clone(),
        }
    }

    fn symbol(
        &self,
        node: Node<'_>,
        scope: &Scope,
        name: &str,
        kind: SymbolKind,
        ns: Ns,
        declared_type: DeclaredType,
    ) -> Result<Symbol, FqnError> {
        Ok(Symbol {
            fqn: self.declare(scope, name, ns)?,
            kind,
            name: name.to_string(),
            span: span(node),
            visibility: self.visibility(node),
            docstring: self.docstring(node),
            declared_type,
            params: Vec::new(),
        })
    }

    /// A declaration whose whole identity is a `name` field: type alias,
    /// associated type, const, static, macro definition.
    fn plain(&mut self, node: Node<'_>, scope: &Scope, kind: SymbolKind, ns: Ns) {
        let Some(name) = self.field_text(node, "name") else {
            self.children(node, scope);
            return;
        };
        let declared = self.declared_type(node, "type");
        let symbol = self.symbol(node, scope, name, kind, ns, declared);
        let inner = self.push(symbol, scope);
        self.children(node, &inner);
    }

    fn function(&mut self, node: Node<'_>, scope: &Scope) {
        let Some(name) = self.field_text(node, "name") else {
            self.children(node, scope);
            return;
        };
        // A free `fn` is a function; the same node inside a type's body is a
        // method. Nothing about the node itself says which — only the container.
        let kind = match scope.container {
            Container::File => SymbolKind::Function,
            _ => SymbolKind::Method,
        };
        let declared = self.declared_type(node, "return_type");
        let symbol = self.symbol(node, scope, name, kind, Ns::Val, declared).map(|mut s| {
            s.params = self.params(node);
            s
        });
        let inner = self.push(symbol, scope);
        self.children(node, &inner);
    }

    /// A struct, union, enum or trait: a named type whose body declares members
    /// of it. The body is walked with the type as the container, which is what
    /// makes its fields and methods members rather than free items.
    fn type_with_fields(&mut self, node: Node<'_>, scope: &Scope, kind: SymbolKind) {
        let Some(name) = self.field_text(node, "name") else {
            self.children(node, scope);
            return;
        };
        let symbol = self.symbol(node, scope, name, kind, Ns::Ty, DeclaredType::Unstated);
        let mut inner = self.push(symbol, scope);
        inner.container = Container::Type(name.to_string());
        self.children(node, &inner);
    }

    /// An enum variant is a member of its enum in the value namespace, and its
    /// own fields are members of the variant. `Enum::Variant` as the type
    /// segment is what keeps two variants' same-named fields apart.
    fn enum_variant(&mut self, node: Node<'_>, scope: &Scope) {
        let Some(name) = self.field_text(node, "name") else {
            self.children(node, scope);
            return;
        };
        let symbol = self.symbol(
            node,
            scope,
            name,
            SymbolKind::EnumVariant,
            Ns::Val,
            DeclaredType::Unstated,
        );
        let mut inner = self.push(symbol, scope);
        if let Container::Type(enum_name) = &scope.container {
            inner.container = Container::Type(format!("{enum_name}::{name}"));
        }
        self.children(node, &inner);
    }

    fn named_field(&mut self, node: Node<'_>, scope: &Scope) {
        let Some(name) = self.field_text(node, "name") else {
            self.children(node, scope);
            return;
        };
        let declared = self.declared_type(node, "type");
        let symbol = self.symbol(node, scope, name, SymbolKind::Field, Ns::Field, declared);
        let inner = self.push(symbol, scope);
        self.children(node, &inner);
    }

    /// A tuple-struct or tuple-variant field has no name, so its identity is its
    /// POSITION, spelled exactly the way a use site spells it (`pair.0`).
    fn positional_fields(&mut self, node: Node<'_>, scope: &Scope) {
        let mut cursor = node.walk();
        let types: Vec<Node<'_>> = node.children_by_field_name("type", &mut cursor).collect();
        for (position, ty) in types.into_iter().enumerate() {
            let name = position.to_string();
            let declared = DeclaredType::Stated(self.text(ty).to_string());
            let symbol = self.symbol(ty, scope, &name, SymbolKind::Field, Ns::Field, declared);
            let inner = self.push(symbol, scope);
            // The type node is dispatched, not descended into: the field's type
            // is itself a use site and skipping it would lose the edge.
            self.node(ty, &inner);
        }
    }

    /// An inline `mod` extends the module path, so a declaration inside one is
    /// named the same as if it lived in its own file. Two spellings of one
    /// symbol would never merge (spec §2).
    fn module_item(&mut self, node: Node<'_>, scope: &Scope) {
        let Some(name) = self.field_text(node, "name") else {
            self.children(node, scope);
            return;
        };
        let symbol =
            self.symbol(node, scope, name, SymbolKind::Module, Ns::Ty, DeclaredType::Unstated);
        let mut inner = self.push(symbol, scope);
        inner.module = if scope.module.is_empty() {
            name.to_string()
        } else {
            format!("{}::{name}", scope.module)
        };
        inner.container = Container::File;
        self.children(node, &inner);
    }

    /// An `impl` is not a declaration — it is a container for the ones inside
    /// it. Which container depends on whether a trait is named.
    fn impl_block(&mut self, node: Node<'_>, scope: &Scope) {
        let Some(ty) = node.child_by_field_name("type").map(|n| self.text(n)) else {
            self.children(node, scope);
            return;
        };
        let Ok(ty) = fqn::type_segment(ty) else {
            let mut inner = scope.clone();
            inner.container = Container::Unnameable { raw: ty.to_string() };
            self.children(node, &inner);
            return;
        };
        let tr = node
            .child_by_field_name("trait")
            .map(|n| self.text(n))
            .and_then(|raw| fqn::type_segment(raw).ok());

        let mut inner = scope.clone();
        inner.container = match tr {
            Some(tr) => Container::TraitImpl { ty: ty.clone(), tr },
            None => Container::Type(ty.clone()),
        };
        // A use site in the impl header sits inside no member, so it belongs to
        // the type the impl is about.
        if let Ok(owner) = fqn::define(&Form::Item {
            lang: Language::Rust,
            package: self.package,
            module: &scope.module,
            name: &ty,
            ns: Ns::Ty,
        }) {
            inner.from = owner;
        }
        self.children(node, &inner);
    }

    // ── use sites (spec §3.2, R2) ────────────────────────────────────────────

    /// Every use site the plan's step 4 names — call, member access, path use,
    /// construction — plus a macro invocation, which is a use of the macro.
    ///
    /// A bare `identifier` in expression position is deliberately not one.
    /// Without scope analysis the walk cannot tell a local variable read from a
    /// const read, and emitting every one of them would bury the graph in
    /// references that can never resolve. The rung that could tell them apart is
    /// step 5's.
    fn use_site(&mut self, node: Node<'_>, scope: &Scope) {
        if named_by_its_parent(node) || in_path_position(node) {
            return;
        }
        match node.kind() {
            "call_expression" => self.call(node, scope),
            "macro_invocation" => self.macro_use(node, scope),
            "struct_expression" => self.construct(node, scope),
            "field_expression" => self.member_access(node, scope),
            "scoped_identifier" => self.path_use(node, scope),
            "type_identifier" | "scoped_type_identifier" | "generic_type" => {
                self.type_use(node, scope)
            }
            // Not a use site. Distinct from a use site with no rule, which is
            // the `UnhandledForm` miss the arms above produce — nothing that
            // reaches here names anything.
            _ => {}
        }
    }

    fn call(&mut self, node: Node<'_>, scope: &Scope) {
        let miss = match node.child_by_field_name("function") {
            Some(callee) => self.name_callee(callee, scope, Ns::Val),
            None => Miss::unhandled(node, self.text(node)),
        };
        self.emit(scope, RefKind::Calls, node, miss);
    }

    /// Whatever stands in callee position. Every shape either names something or
    /// says which shape defeated it — there is no path out of here that emits
    /// nothing, which is the defect this rewrite exists to remove.
    fn name_callee(&self, callee: Node<'_>, scope: &Scope, ns: Ns) -> Miss {
        match callee.kind() {
            "identifier" | "scoped_identifier" => {
                let path = self.text(callee);
                Miss::unplaced(callee, path, self.considered_path(path, scope, ns))
            }
            // A turbofish wraps the real callee; the type arguments are use
            // sites in their own right and are counted as such.
            "generic_function" => match callee.child_by_field_name("function") {
                Some(inner) => self.name_callee(inner, scope, ns),
                None => Miss::unhandled(callee, self.text(callee)),
            },
            "field_expression" => self.name_member(callee, scope, Ns::Val),
            _ => Miss::unhandled(callee, self.text(callee)),
        }
    }

    /// `receiver.member`, in either callee or read position. The member's
    /// identity depends on the receiver's TYPE, which the walk only knows when
    /// the receiver is `self` inside a type's own body — anywhere else that is a
    /// real, permanent-until-inference miss and is reported as one.
    fn name_member(&self, node: Node<'_>, scope: &Scope, ns: Ns) -> Miss {
        // Both children are malformed-source cases, and an empty string in
        // either would name a member that is not there. Say what was seen.
        let (Some(member), Some(receiver)) =
            (self.field_text(node, "field"), self.field_text(node, "value"))
        else {
            return Miss::unhandled(node, self.text(node));
        };

        let self_type = match (&scope.container, receiver) {
            (Container::Type(ty), "self" | "Self") => Some(ty.as_str()),
            (Container::TraitImpl { ty, .. }, "self" | "Self") => Some(ty.as_str()),
            _ => None,
        };
        let Some(ty) = self_type else {
            return Miss {
                reason: Reason::ReceiverTypeUnknown,
                name: member.to_string(),
                node_kind: node.kind().to_string(),
                saw: vec![Observation::Receiver(receiver.to_string())],
            };
        };
        let considered = considered(fqn::refer(&Form::Member {
            lang: Language::Rust,
            package: self.package,
            module: &scope.module,
            ty,
            member,
            ns,
        }));
        Miss::unplaced(node, member, considered)
    }

    fn member_access(&mut self, node: Node<'_>, scope: &Scope) {
        // A read and a write of one field are different facts, and pattern
        // detection reads the difference (R8).
        let kind = if is_assignment_target(node) { RefKind::Writes } else { RefKind::Reads };
        let miss = self.name_member(node, scope, Ns::Field);
        self.emit(scope, kind, node, miss);
    }

    fn macro_use(&mut self, node: Node<'_>, scope: &Scope) {
        let miss = match node.child_by_field_name("macro") {
            Some(name) => {
                let path = self.text(name);
                Miss {
                    node_kind: node.kind().to_string(),
                    ..Miss::unplaced(name, path, self.considered_path(path, scope, Ns::Macro))
                }
            }
            None => Miss::unhandled(node, self.text(node)),
        };
        self.emit(scope, RefKind::MacroInvokes, node, miss);
    }

    fn construct(&mut self, node: Node<'_>, scope: &Scope) {
        let miss = match node.child_by_field_name("name") {
            Some(name) => self.name_type(name, scope),
            None => Miss::unhandled(node, self.text(node)),
        };
        self.emit(scope, RefKind::Constructs, node, miss);
    }

    fn path_use(&mut self, node: Node<'_>, scope: &Scope) {
        let path = self.text(node);
        let miss = Miss::unplaced(node, path, self.considered_path(path, scope, Ns::Val));
        self.emit(scope, RefKind::Reads, node, miss);
    }

    fn type_use(&mut self, node: Node<'_>, scope: &Scope) {
        let miss = self.name_type(node, scope);
        self.emit(scope, RefKind::TypeUse, node, miss);
    }

    /// A type named in a signature, a field, a bound or a construction. The one
    /// owner of type-text normalisation is `fqn::type_segment`, so the use side
    /// and the definition side reduce `Widget<T>` and `Widget` the same way.
    fn name_type(&self, node: Node<'_>, scope: &Scope) -> Miss {
        let raw = self.text(node);
        let Ok(name) = fqn::type_segment(raw) else {
            return Miss::unhandled(node, raw);
        };
        let considered = considered(fqn::refer(&Form::Item {
            lang: Language::Rust,
            package: self.package,
            module: &scope.module,
            name: &name,
            ns: Ns::Ty,
        }));
        Miss::unplaced(node, &name, considered)
    }

    /// The identity a path COULD name if it named something in this module —
    /// CONSIDERED, never proven, which is why it goes into the evidence and not
    /// into the resolution. Only the shapes whose reading is unambiguous get
    /// one; a longer path needs the import table, which is the ladder's.
    fn considered_path(&self, raw: &str, scope: &Scope, ns: Ns) -> Vec<Observation> {
        let lang = Language::Rust;
        let package = self.package;
        let module = scope.module.as_str();
        // Turbofish arguments decorate a path without naming a segment of it.
        let segments: Vec<&str> = raw
            .split("::")
            .map(str::trim)
            .filter(|s| !s.starts_with('<') && !s.is_empty())
            .collect();
        match segments.as_slice() {
            [name] => considered(fqn::refer(&Form::Item { lang, package, module, name, ns })),
            [ty, member] if !matches!(*ty, "crate" | "self" | "super") => {
                match fqn::type_segment(ty) {
                    Ok(ty) => considered(fqn::refer(&Form::Member {
                        lang,
                        package,
                        module,
                        ty: &ty,
                        member,
                        ns,
                    })),
                    Err(_) => Vec::new(),
                }
            }
            // A longer path is not a thing this walk can read on its own, and a
            // candidate it could not justify would be material a later pass
            // would mistake for an answer.
            _ => Vec::new(),
        }
    }

    fn emit(&mut self, scope: &Scope, kind: RefKind, at: Node<'_>, miss: Miss) {
        self.references.push(Reference {
            from: scope.from.clone(),
            kind,
            at: span(at),
            target: Resolution::Unresolved {
                reason: miss.reason,
                evidence: Evidence { name: miss.name, node_kind: miss.node_kind, saw: miss.saw },
            },
        });
    }

    // ── imports (spec §2) ────────────────────────────────────────────────────

    fn import(&mut self, node: Node<'_>) {
        if let Some(argument) = node.child_by_field_name("argument") {
            self.import_tree(argument, "", node);
        }
    }

    /// `extern crate foo;` binds `foo` from outside this source. Rare after the
    /// 2018 edition, but it is an import and dropping it would lose the binding.
    fn extern_crate(&mut self, node: Node<'_>) {
        let Some(name) = self.field_text(node, "name") else {
            return;
        };
        let bound = match self.field_text(node, "alias") {
            Some(alias) => alias,
            None => name,
        };
        self.imports.push(Import {
            path: name.to_string(),
            binds: Binding::Name(bound.to_string()),
            origin: import_origin(name),
            at: span(node),
        });
    }

    /// A `use` tree flattened into one import per bound name, so that
    /// `use a::{b, c as d}` says what it binds instead of leaving a caller to
    /// re-parse the specifier.
    fn import_tree(&mut self, node: Node<'_>, prefix: &str, at: Node<'_>) {
        let text = self.text(node);
        let joined = |segment: &str| {
            if prefix.is_empty() { segment.to_string() } else { format!("{prefix}::{segment}") }
        };
        match node.kind() {
            "scoped_use_list" => {
                let (Some(path), Some(list)) =
                    (self.field_text(node, "path"), node.child_by_field_name("list"))
                else {
                    // A half-parsed group binds something this walk cannot name.
                    return self.bind(joined(text), Binding::Glob, at);
                };
                let full = joined(path);
                let mut cursor = list.walk();
                for item in list.named_children(&mut cursor) {
                    self.import_tree(item, &full, at);
                }
            }
            "use_list" => {
                let mut cursor = node.walk();
                for item in node.named_children(&mut cursor) {
                    self.import_tree(item, prefix, at);
                }
            }
            "use_wildcard" => self.bind(joined(text), Binding::Glob, at),
            "use_as_clause" => match self.field_text(node, "alias") {
                Some(alias) => self.bind(joined(text), Binding::Name(alias.to_string()), at),
                // `use a::b as` with nothing after it binds an unknown name.
                None => self.bind(joined(text), Binding::Glob, at),
            },
            "scoped_identifier" | "identifier" | "crate" | "super" | "self" => {
                let full = joined(text);
                // `use a::{self}` binds `a`, not a name called `self`.
                let bound = if text == "self" { prefix } else { full.as_str() };
                let bound = bound.rsplit("::").next().unwrap_or(bound).to_string();
                self.bind(full, Binding::Name(bound), at);
            }
            // A specifier shape with no rule here binds SOMETHING, and the walk
            // cannot say what. `Glob` is exactly that statement: it can never
            // read as proof that a given name came from this import (R4).
            _ => self.bind(joined(text), Binding::Glob, at),
        }
    }

    fn bind(&mut self, path: String, binds: Binding, at: Node<'_>) {
        self.imports.push(Import { origin: import_origin(&path), path, binds, at: span(at) });
    }

    fn declared_type(&self, node: Node<'_>, field: &str) -> DeclaredType {
        // Total on purpose: "the language states no type here" is a fact the
        // walk read, not an absence it failed to read (R3, spec §5).
        match self.field_text(node, field) {
            Some(text) => DeclaredType::Stated(text.to_string()),
            None => DeclaredType::Unstated,
        }
    }

    /// Parameters are typed props on their function, never nodes (D2).
    fn params(&self, node: Node<'_>) -> Vec<Param> {
        let Some(list) = node.child_by_field_name("parameters") else {
            return Vec::new();
        };
        let mut cursor = list.walk();
        list.named_children(&mut cursor)
            .enumerate()
            .map(|(position, param)| {
                let position = position as u32;
                match param.kind() {
                    "self_parameter" => Param {
                        name: "self".to_string(),
                        position,
                        // Rust states no type for the receiver at this point; it
                        // comes from the impl. Reading it off the impl would be
                        // inference, which is a non-goal (spec §5).
                        declared_type: DeclaredType::Unstated,
                    },
                    _ => Param {
                        name: self
                            .field_text(param, "pattern")
                            .unwrap_or_else(|| self.text(param))
                            .to_string(),
                        position,
                        declared_type: self.declared_type(param, "type"),
                    },
                }
            })
            .collect()
    }

    fn visibility(&self, node: Node<'_>) -> Visibility {
        let mut cursor = node.walk();
        let modifier = node.named_children(&mut cursor).find(|c| c.kind() == "visibility_modifier");
        let Some(modifier) = modifier else {
            return Visibility::Private;
        };
        match self.text(modifier) {
            "pub" => Visibility::Public,
            "pub(crate)" => Visibility::Crate,
            // The scope is kept verbatim: narrowing `pub(in a::b)` to a flag
            // would discard which scope was meant.
            other => Visibility::Restricted(
                other.trim_start_matches("pub").trim_matches(['(', ')']).trim().to_string(),
            ),
        }
    }

    /// The doc comment attached to a declaration, if the source carries one.
    /// Attributes may sit between the comment and the item, so they are stepped
    /// over rather than treated as the end of the run.
    fn docstring(&self, node: Node<'_>) -> Option<String> {
        let mut lines: Vec<&str> = Vec::new();
        let mut sibling = node.prev_sibling();
        while let Some(current) = sibling {
            match current.kind() {
                "attribute_item" => {}
                "line_comment" | "block_comment" => {
                    // A comment node carries its own line ending, so it is
                    // trimmed before joining: keeping both would put a blank
                    // line between every two lines of every doc comment.
                    let text = self.text(current).trim_end();
                    if !text.starts_with("///") && !text.starts_with("/**") {
                        break;
                    }
                    lines.push(text);
                }
                _ => break,
            }
            sibling = current.prev_sibling();
        }
        if lines.is_empty() {
            return None;
        }
        lines.reverse();
        Some(lines.join("\n"))
    }
}

/// What the walk has to say about a use site it did not place: the bucket, the
/// name, the node kind that names the bucket in the histogram, and the material
/// a later pass gets to work with.
///
/// Every use-site arm produces one of these and then emits, so there is no path
/// through this module on which a use site yields nothing.
struct Miss {
    reason: Reason,
    name: String,
    /// The kind of the node that DEFEATED the walk, which is the anchor's own
    /// kind when the walk read it fine and the inner node's kind when it did
    /// not. This is what a reader sees in the histogram.
    node_kind: String,
    saw: Vec<Observation>,
}

impl Miss {
    /// The walk read the use site and named it. Placing that name needs the
    /// shared ladder, which a language module is not (R7).
    fn unplaced(node: Node<'_>, name: &str, saw: Vec<Observation>) -> Self {
        Self {
            reason: Reason::Unplaced,
            name: readable(name, node),
            node_kind: node.kind().to_string(),
            saw,
        }
    }

    /// The walk has no rule for this shape. Named rather than dropped, so the
    /// histogram says what was not understood instead of saying nothing.
    fn unhandled(node: Node<'_>, name: &str) -> Self {
        Self {
            reason: Reason::UnhandledForm,
            name: readable(name, node),
            node_kind: node.kind().to_string(),
            saw: Vec::new(),
        }
    }
}

/// An identity the walk CONSIDERED, as evidence — never as a resolution. One
/// that could not even be minted leaves no observation behind rather than a
/// placeholder one.
fn considered(minted: Result<Fqn, FqnError>) -> Vec<Observation> {
    minted.map(Observation::Candidate).into_iter().collect()
}

/// Evidence must always name something. In a tree full of ERROR nodes a node's
/// text can be empty, and an unnameable miss is one nobody can act on, so the
/// node kind stands in.
fn readable(name: &str, node: Node<'_>) -> String {
    let name = name.trim();
    if name.is_empty() { node.kind().to_string() } else { name.to_string() }
}

/// True when a node is NAMED BY its parent rather than naming something itself:
/// the callee of a call, the macro of an invocation, the type of a construction,
/// the `name` of any declaration. The parent emits the reference; a second one
/// here would be the same edge twice.
fn named_by_its_parent(node: Node<'_>) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };
    let is_field =
        |field: &str| parent.child_by_field_name(field).is_some_and(|c| c.id() == node.id());
    match parent.kind() {
        "call_expression" | "generic_function" => is_field("function"),
        "macro_invocation" => is_field("macro"),
        _ => is_field("name"),
    }
}

/// True when a node is a SEGMENT of a longer path rather than a use site of its
/// own: `crate::db::PgStore::connect` names one thing, not four.
fn in_path_position(node: Node<'_>) -> bool {
    node.parent().is_some_and(|parent| {
        matches!(
            parent.kind(),
            "scoped_identifier"
                | "scoped_type_identifier"
                | "generic_type"
                | "qualified_type"
                | "bracketed_type"
        )
    })
}

fn is_assignment_target(node: Node<'_>) -> bool {
    node.parent().is_some_and(|parent| {
        matches!(parent.kind(), "assignment_expression" | "compound_assignment_expr")
            && parent.child_by_field_name("left").is_some_and(|c| c.id() == node.id())
    })
}

/// Which side of the scanned source an import specifier points at (spec §2).
///
/// The specifier is the ONLY input, because absence is scan-order dependent and
/// resolution must not be (R6). One consequence is recorded rather than hidden:
/// a sibling crate of the same workspace is rooted at its own name, so it lands
/// here as `External`. Correcting that needs the set of first-party packages,
/// which the walk is never handed and the processor owns — so the ladder must
/// consult that set before it turns an `External` import into a library target.
fn import_origin(path: &str) -> ImportOrigin {
    match path.split("::").next().map(str::trim) {
        Some("crate" | "self" | "super") => ImportOrigin::Local,
        // No head, or an empty one, is malformed source rather than a package.
        // Claiming External would mint a library node out of nothing.
        None | Some("") => ImportOrigin::Local,
        Some(package) => ImportOrigin::External { package: package.to_string() },
    }
}

/// Columns are tree-sitter's, which counts BYTES within the line rather than
/// characters. Recorded because two references on one line are told apart by
/// column, and a reader comparing these against a character offset would find
/// them shifted on any line carrying non-ASCII.
fn span(node: Node<'_>) -> crate::indexer::facts::Span {
    let start = node.start_position();
    let end = node.end_position();
    crate::indexer::facts::Span {
        start_line: start.row as u32 + 1,
        start_col: start.column as u32,
        end_line: end.row as u32 + 1,
        end_col: end.column as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexer::facts::{
        Binding, ImportOrigin, Observation, Reason, RefKind, Resolution, SymbolKind,
    };

    /// One of every declaration the plan's step 3 names, in one file, so the
    /// expected set can be written out by hand and compared whole.
    const ONE_OF_EACH: &str = r#"
use std::collections::HashMap;

pub mod inner;

pub const MAX: u32 = 10;
pub static NAME: &str = "x";
pub type Alias = HashMap<String, u32>;

macro_rules! shout { () => {} }

/// A widget.
pub struct Widget {
    pub width: u32,
    height: u32,
}

pub struct Pair(pub u32, String);

pub enum Shape {
    Circle,
    Rect { w: u32 },
}

pub trait Draw {
    type Canvas;
    const SIDES: u32;
    fn draw(&self) -> String;
}

impl Widget {
    pub fn new(width: u32) -> Widget { Widget { width, height: 0 } }
    fn width(&self) -> u32 { self.width }
}

impl Draw for Widget {
    type Canvas = u32;
    const SIDES: u32 = 4;
    fn draw(&self) -> String { String::new() }
}

pub fn free(w: &Widget) -> u32 { w.width }
"#;

    fn facts(module: &str, text: &str) -> FileFacts {
        read(&Source { package: "p", module, text }).expect("the fixture parses")
    }

    fn fqns(facts: &FileFacts) -> Vec<&str> {
        facts.symbols.iter().map(|s| s.fqn.as_str()).collect()
    }

    /// The whole expected symbol set, written out. A count alone would pass
    /// while naming the wrong things.
    #[test]
    fn every_declaration_in_the_fixture_becomes_exactly_one_symbol() {
        let facts = facts("m", ONE_OF_EACH);
        let mut got = fqns(&facts);
        got.sort_unstable();

        let mut expected = vec![
            "rust·p·m·inner·ty",
            "rust·p·m·MAX·val",
            "rust·p·m·NAME·val",
            "rust·p·m·Alias·ty",
            "rust·p·m·shout·macro",
            "rust·p·m·Widget·ty",
            "rust·p·m·Widget·width·field",
            "rust·p·m·Widget·height·field",
            "rust·p·m·Pair·ty",
            "rust·p·m·Pair·0·field",
            "rust·p·m·Pair·1·field",
            "rust·p·m·Shape·ty",
            "rust·p·m·Shape·Circle·val",
            "rust·p·m·Shape·Rect·val",
            "rust·p·m·Shape::Rect·w·field",
            "rust·p·m·Draw·ty",
            "rust·p·m·Draw·Canvas·ty",
            "rust·p·m·Draw·SIDES·val",
            "rust·p·m·Draw·draw·val",
            "rust·p·m·Widget·new·val",
            "rust·p·m·Widget·width·val",
            "rust·p·m·Widget·Draw·Canvas·ty",
            "rust·p·m·Widget·Draw·SIDES·val",
            "rust·p·m·Widget·Draw·draw·val",
            "rust·p·m·free·val",
        ];
        expected.sort_unstable();

        assert_eq!(got, expected);
    }

    /// The collision the fqn grammar's namespace segment exists to prevent,
    /// checked end to end through the walk rather than through the builder.
    /// `nodes_unique_identity` is `(folder, path, kind, name, parent, line)`, so
    /// two declarations sharing a name on one type must differ in `kind` AND in
    /// identity — one of them being overwritten is a wrong edge (R4).
    #[test]
    fn a_field_and_a_method_of_the_same_name_are_two_symbols_with_two_identities() {
        let facts = facts("m", ONE_OF_EACH);
        let width: Vec<_> = facts.symbols.iter().filter(|s| s.name == "width").collect();

        assert_eq!(width.len(), 2, "the field and the method are both declarations");
        assert_ne!(width[0].fqn, width[1].fqn, "one identity would merge them onto one node");
        assert_ne!(width[0].kind, width[1].kind, "kind is what `nodes_unique_identity` separates");
        let mut kinds = width.iter().map(|s| s.kind).collect::<Vec<_>>();
        kinds.sort_by_key(|k| format!("{k:?}"));
        assert_eq!(kinds, vec![SymbolKind::Field, SymbolKind::Method]);
    }

    /// A tuple-struct field has no name, so its identity is its POSITION, spelled
    /// the same way a use site spells it (`pair.0`). Written down here because
    /// the choice is arbitrary until it is recorded.
    #[test]
    fn a_tuple_struct_field_is_identified_by_its_position() {
        let facts = facts("m", ONE_OF_EACH);
        let positional: Vec<_> =
            facts.symbols.iter().filter(|s| s.fqn.as_str().contains("·Pair·")).collect();
        let positional: Vec<_> =
            positional.into_iter().filter(|s| s.kind == SymbolKind::Field).collect();

        assert_eq!(positional.len(), 2);
        for (index, symbol) in positional.iter().enumerate() {
            assert_eq!(symbol.name, index.to_string(), "the name is the position");
            assert_eq!(symbol.kind, SymbolKind::Field);
        }
        assert_eq!(positional[0].declared_type, DeclaredType::Stated("u32".to_string()));
        assert_eq!(positional[1].declared_type, DeclaredType::Stated("String".to_string()));
    }

    /// D2: a parameter is a typed prop on its function, never a node of its own.
    /// R3: the type the language states reaches the fact, verbatim.
    #[test]
    fn a_parameter_is_a_typed_prop_and_not_a_symbol() {
        let facts = facts("m", ONE_OF_EACH);
        assert!(
            !fqns(&facts)
                .iter()
                .any(|f| f.ends_with("·w·val") || f.ends_with("·width·val") && f.contains("free")),
            "no parameter may appear as a symbol"
        );

        let free = facts.symbols.iter().find(|s| s.name == "free").expect("the free fn");
        assert_eq!(free.kind, SymbolKind::Function);
        assert_eq!(free.declared_type, DeclaredType::Stated("u32".to_string()), "the return type");
        assert_eq!(free.params.len(), 1);
        assert_eq!(free.params[0].name, "w");
        assert_eq!(free.params[0].position, 0);
        assert_eq!(free.params[0].declared_type, DeclaredType::Stated("&Widget".to_string()));
    }

    /// R3 for the fact that lost data before: a field's declared type. And the
    /// other half of `DeclaredType` being total — a struct states no type of its
    /// own, and that is a reading, not a failure.
    #[test]
    fn a_declared_type_reaches_the_fact_and_an_absent_one_is_stated_as_absent() {
        let facts = facts("m", ONE_OF_EACH);
        let by = |fqn: &str| {
            facts.symbols.iter().find(|s| s.fqn.as_str() == fqn).unwrap_or_else(|| {
                panic!("{fqn} is missing from {:?}", fqns(&facts));
            })
        };

        assert_eq!(
            by("rust·p·m·Widget·width·field").declared_type,
            DeclaredType::Stated("u32".to_string())
        );
        assert_eq!(by("rust·p·m·MAX·val").declared_type, DeclaredType::Stated("u32".to_string()));
        assert_eq!(
            by("rust·p·m·Widget·ty").declared_type,
            DeclaredType::Unstated,
            "a struct states no type of its own; that is a fact, not a miss"
        );
    }

    /// An inline `mod` extends the module path, so a declaration inside one is
    /// named the same way whether it lives in `a/b.rs` or in `mod b { }` inside
    /// `a.rs`. Two spellings of one symbol would never merge (spec §2).
    #[test]
    fn an_inline_module_extends_the_module_path_of_what_it_contains() {
        let facts = facts("a", "pub mod b { pub fn f() {} pub mod c { pub fn g() {} } }");
        let mut got = fqns(&facts);
        got.sort_unstable();
        assert_eq!(
            got,
            vec!["rust·p·a::b::c·g·val", "rust·p·a::b·c·ty", "rust·p·a::b·f·val", "rust·p·a·b·ty",]
        );
    }

    /// At the crate root the module segment is empty, which the grammar drops.
    #[test]
    fn a_declaration_at_the_crate_root_carries_no_module_segment() {
        let facts = facts("", "pub fn main() {}");
        assert_eq!(fqns(&facts), vec!["rust·p·main·val"]);
    }

    /// `impl Trait for (A, B)` names no type, so its members have no identity in
    /// this grammar. A missing symbol is the correct outcome; naming them as if
    /// they were free items of the module would mint an identity no use site
    /// could ever mint, which is a wrong edge and worse than a missing one (R4).
    /// The body is still walked, so nothing inside it is lost.
    #[test]
    fn a_member_of_an_unnameable_impl_is_missing_rather_than_misnamed() {
        let facts = facts(
            "m",
            "pub trait Draw { fn draw(&self); }\nimpl Draw for (u32, u32) { fn draw(&self) { helper(); } }",
        );
        let got = fqns(&facts);
        assert!(
            !got.contains(&"rust\u{b7}p\u{b7}m\u{b7}draw\u{b7}val"),
            "the trait-impl method was named as a free item of the module: {got:?}"
        );
        assert_eq!(
            got.iter().filter(|f| f.ends_with("draw\u{b7}val")).count(),
            1,
            "only the trait's own declaration can be named; got {got:?}"
        );
        assert_eq!(
            facts.references.iter().filter(|r| r.kind == RefKind::Calls).count(),
            1,
            "the body is still walked, so `helper()` is not lost"
        );
    }

    /// Visibility and the doc comment are facts R8 and "what is this module's
    /// surface" read, and both are stated by the source rather than inferred.
    /// A restricted scope is kept verbatim, because narrowing `pub(in a::b)` to a
    /// flag would discard which scope was meant.
    #[test]
    fn a_declarations_visibility_and_docstring_are_read_as_the_source_states_them() {
        let facts = facts(
            "m",
            "/// One.\n/// Two.\n#[derive(Debug)]\npub struct A;\npub(crate) struct B;\npub(super) struct C;\nstruct D;\n",
        );
        let by = |name: &str| {
            facts.symbols.iter().find(|s| s.name == name).unwrap_or_else(|| panic!("{name}"))
        };

        assert_eq!(by("A").visibility, Visibility::Public);
        assert_eq!(by("B").visibility, Visibility::Crate);
        assert_eq!(by("C").visibility, Visibility::Restricted("super".to_string()));
        assert_eq!(by("D").visibility, Visibility::Private);
        assert_eq!(
            by("A").docstring.as_deref(),
            Some("/// One.\n/// Two."),
            "an attribute between the comment and the item does not end the run"
        );
        assert_eq!(by("B").docstring, None, "no comment is not an empty comment");
    }

    // ── independent counters ─────────────────────────────────────────────────
    //
    // These are written from the OTHER side: they walk the same tree and count
    // nodes by tree-sitter kind, knowing nothing about `Walk`, its scopes or its
    // fqns. A producer that checks its own output proves only that it is
    // self-consistent; this is what notices a dropped fact.

    fn parse(text: &str) -> tree_sitter::Tree {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_rust::LANGUAGE.into()).expect("the rust grammar loads");
        parser.parse(text, None).expect("tree-sitter returns a tree")
    }

    /// The node kinds that ARE a declaration, listed here and nowhere else.
    const DECLARATION_KINDS: &[&str] = &[
        "function_item",
        "function_signature_item",
        "struct_item",
        "union_item",
        "enum_item",
        "enum_variant",
        "trait_item",
        "type_item",
        "associated_type",
        "const_item",
        "static_item",
        "mod_item",
        "macro_definition",
        "field_declaration",
    ];

    /// Every source file of this repo's own Rust, which is the corpus the
    /// counting tests run over. A fixture proves the walk handles what the
    /// fixture's author thought of; the corpus proves it handles what is there.
    fn repo_rust_sources() -> Vec<(String, String)> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("the crate sits two levels below the workspace root")
            .join("crates");
        let mut out = Vec::new();
        for entry in walkdir::WalkDir::new(&root).into_iter().filter_entry(|e| {
            e.file_name() != "target" && !e.file_name().to_string_lossy().starts_with('.')
        }) {
            let entry = entry.expect("the workspace source tree must be readable");
            if entry.path().extension().is_none_or(|ext| ext != "rs") {
                continue;
            }
            let body = std::fs::read_to_string(entry.path())
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", entry.path().display()));
            out.push((entry.path().display().to_string(), body));
        }
        assert!(out.len() > 100, "the corpus is this repo's rust; {} files is not it", out.len());
        out
    }

    /// True for a node the walk deliberately does not read: an attribute's
    /// contents are an unparsed token tree.
    fn inside_an_attribute(node: tree_sitter::Node<'_>) -> bool {
        let mut current = node.parent();
        while let Some(n) = current {
            if matches!(n.kind(), "attribute_item" | "inner_attribute_item") {
                return true;
            }
            current = n.parent();
        }
        false
    }

    /// True for a declaration whose enclosing `impl` is on a type with no name
    /// — a tuple, a slice, a unit. There is no type segment to build a member
    /// identity from, so no such declaration is nameable. Stated here in
    /// tree-sitter terms rather than by asking the walk.
    fn inside_an_unnameable_impl(node: tree_sitter::Node<'_>) -> bool {
        let mut current = node.parent();
        while let Some(n) = current {
            if n.kind() == "impl_item" {
                return !n.child_by_field_name("type").is_some_and(|t| {
                    matches!(
                        t.kind(),
                        "type_identifier" | "scoped_type_identifier" | "generic_type"
                    )
                });
            }
            current = n.parent();
        }
        false
    }

    /// Declarations, counted by kind, with no knowledge of `Walk`.
    fn count_declarations(root: tree_sitter::Node<'_>) -> usize {
        let mut count = 0;
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            if !inside_an_attribute(node) && !inside_an_unnameable_impl(node) {
                if DECLARATION_KINDS.contains(&node.kind())
                    && node.child_by_field_name("name").is_some()
                {
                    count += 1;
                }
                if node.kind() == "ordered_field_declaration_list" {
                    let mut cursor = node.walk();
                    count += node.children_by_field_name("type", &mut cursor).count();
                }
            }
            let mut cursor = node.walk();
            stack.extend(node.named_children(&mut cursor));
        }
        count
    }

    /// The node kinds that ARE a use site, listed here and nowhere else.
    ///
    /// This is the plan's step-4 set — call, member access, path use,
    /// construction — plus macro invocation, which is a use of the macro. A bare
    /// `identifier` in expression position is NOT here: without scope analysis
    /// the walk cannot tell a local variable read from a const read, and the
    /// rung that could is step 5's. That is a scoping decision, not an
    /// oversight, and both sides of this count apply it.
    const USE_SITE_KINDS: &[&str] = &[
        "call_expression",
        "macro_invocation",
        "struct_expression",
        "field_expression",
        "scoped_identifier",
        "type_identifier",
        "scoped_type_identifier",
        "generic_type",
    ];

    /// Parent kinds that make a node a SEGMENT of a longer path rather than a
    /// use site of its own. `crate::db::PgStore::connect` names one thing, not
    /// four.
    const PATH_PARENTS: &[&str] = &[
        "scoped_identifier",
        "scoped_type_identifier",
        "generic_type",
        "qualified_type",
        "bracketed_type",
        "use_declaration",
        "use_as_clause",
        "use_list",
        "scoped_use_list",
        "use_wildcard",
    ];

    /// True when a node is named by its parent rather than naming something
    /// itself: the callee of a call, the macro of an invocation, the type of a
    /// construction, the `name` of any declaration.
    fn is_named_by_its_parent(node: tree_sitter::Node<'_>) -> bool {
        let Some(parent) = node.parent() else {
            return false;
        };
        let is_field =
            |field: &str| parent.child_by_field_name(field).is_some_and(|c| c.id() == node.id());
        match parent.kind() {
            "call_expression" | "generic_function" => is_field("function"),
            "macro_invocation" => is_field("macro"),
            _ => is_field("name"),
        }
    }

    /// True when a node sits inside something the walk does not read: an
    /// attribute's token tree, or a `use` path (which is an Import, not a
    /// reference).
    fn inside_unread_text(node: tree_sitter::Node<'_>) -> bool {
        let mut current = node.parent();
        while let Some(n) = current {
            if matches!(
                n.kind(),
                "attribute_item"
                    | "inner_attribute_item"
                    | "use_declaration"
                    | "extern_crate_declaration"
            ) {
                return true;
            }
            current = n.parent();
        }
        false
    }

    /// Use sites, counted by kind, with no knowledge of `Walk` and none at all
    /// of resolution.
    fn count_use_sites(root: tree_sitter::Node<'_>) -> usize {
        let mut count = 0;
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            let parent_kind = node.parent().map(|p| p.kind()).unwrap_or_default();
            if USE_SITE_KINDS.contains(&node.kind())
                && !is_named_by_its_parent(node)
                && !PATH_PARENTS.contains(&parent_kind)
                && !inside_unread_text(node)
            {
                count += 1;
            }
            let mut cursor = node.walk();
            stack.extend(node.named_children(&mut cursor));
        }
        count
    }

    /// Step 3's load-bearing check, over this repo's own rust rather than a
    /// fixture: if the walk emits fewer symbols than there are declaration nodes
    /// it is dropping declarations, and if it emits more it is inventing them.
    #[test]
    fn the_symbol_count_equals_an_independent_count_of_declaration_nodes() {
        let mut disagreements = Vec::new();
        for (path, text) in repo_rust_sources() {
            let facts = read(&Source { package: "p", module: "m", text: &text })
                .unwrap_or_else(|e| panic!("{path}: {e:?}"));
            let expected = count_declarations(parse(&text).root_node());
            if facts.symbols.len() != expected {
                disagreements.push(format!(
                    "{path}: walk produced {}, independent count says {}",
                    facts.symbols.len(),
                    expected
                ));
            }
        }
        assert!(
            disagreements.is_empty(),
            "{} of the corpus files disagree:\n{}",
            disagreements.len(),
            disagreements.join("\n")
        );
    }

    // ── step 4: every reference ──────────────────────────────────────────────

    /// A2, and the load-bearing check of the whole rewrite. v1's defect was a
    /// catch-all arm that returned early, so a use site it did not understand
    /// left no trace at all and the loss was invisible. This is what makes the
    /// loss visible: an independent count of use-site nodes over this repo's own
    /// rust, against the number of references the walk produced.
    #[test]
    fn the_reference_count_equals_an_independent_count_of_use_sites() {
        let mut disagreements = Vec::new();
        let mut total = 0usize;
        for (path, text) in repo_rust_sources() {
            let facts = read(&Source { package: "p", module: "m", text: &text })
                .unwrap_or_else(|e| panic!("{path}: {e:?}"));
            let expected = count_use_sites(parse(&text).root_node());
            total += expected;
            if facts.references.len() != expected {
                disagreements.push(format!(
                    "{path}: walk produced {}, independent count says {}",
                    facts.references.len(),
                    expected
                ));
            }
        }
        assert!(total > 10_000, "the corpus produced only {total} use sites; that is not it");
        assert!(
            disagreements.is_empty(),
            "{} of the corpus files disagree, so references are being dropped:\n{}",
            disagreements.len(),
            disagreements.join("\n")
        );
    }

    /// A3's precondition. Every miss names its cause and carries the node kind
    /// that produced it, so a form nobody thought about is a NAMED bucket in the
    /// histogram rather than an absence.
    #[test]
    fn every_unresolved_reference_carries_a_reason_and_the_node_kind_it_came_from() {
        for (path, text) in repo_rust_sources().into_iter().take(40) {
            let facts = read(&Source { package: "p", module: "m", text: &text })
                .unwrap_or_else(|e| panic!("{path}: {e:?}"));
            for reference in &facts.references {
                let Resolution::Unresolved { evidence, .. } = &reference.target else {
                    continue;
                };
                assert!(
                    !evidence.name.is_empty(),
                    "{path}: a miss with no name at {:?}",
                    reference.at
                );
                assert!(
                    !evidence.node_kind.is_empty(),
                    "{path}: a miss the histogram cannot name at {:?}",
                    reference.at
                );
            }
        }
    }

    /// The catch-all arm this rewrite exists to remove. A callee shape the walk
    /// has no rule for still yields a reference — one that says so, and says
    /// which node kind it was, rather than yielding nothing.
    #[test]
    fn a_callee_shape_with_no_rule_yields_a_named_miss_and_never_nothing() {
        let facts =
            facts("m", "fn f(t: (fn(), fn()), b: Box<dyn Fn()>) { (b)(); t.0(); [b][0](); }");
        let unhandled: Vec<&str> = facts
            .references
            .iter()
            .filter_map(|r| match &r.target {
                Resolution::Unresolved { reason: Reason::UnhandledForm, evidence } => {
                    Some(evidence.node_kind.as_str())
                }
                _ => None,
            })
            .collect();

        assert!(
            unhandled.contains(&"parenthesized_expression"),
            "`(b)()` must name what it did not understand; got {unhandled:?}"
        );
        assert!(
            unhandled.contains(&"index_expression"),
            "`[b][0]()` must name what it did not understand; got {unhandled:?}"
        );
        assert_eq!(
            facts.references.iter().filter(|r| r.kind == RefKind::Calls).count(),
            3,
            "three calls, three references, however strange the callees"
        );
    }

    /// Deliberately broken source. tree-sitter still returns a tree, full of
    /// ERROR nodes; the walk must read what is there without panicking and
    /// without quietly emitting less than is there.
    #[test]
    fn malformed_source_neither_panics_nor_drops() {
        for text in [
            "fn f() { x.(); }",
            "fn f() { ().0(); }",
            "fn f() { let x = ",
            "impl {",
            "struct S { a: }",
            "fn f() { a.b.c().d[0].e(); }",
            "",
        ] {
            let facts = read(&Source { package: "p", module: "m", text })
                .unwrap_or_else(|e| panic!("`{text}`: {e:?}"));
            let expected = count_use_sites(parse(text).root_node());
            assert_eq!(
                facts.references.len(),
                expected,
                "`{text}`: the walk and the independent count disagree"
            );
        }
    }

    /// THE MERGE CONTRACT, exercised rather than asserted about itself. The
    /// declaration side and the use side are written by different code reading
    /// different nodes; this checks that for a symbol declared and used in one
    /// file, the identity the use site MINTED is byte-identical to the one the
    /// declaration minted. If they differ the two never merge, and no amount of
    /// resolution afterwards can fix it.
    #[test]
    fn a_use_site_mints_the_same_identity_the_declaration_did() {
        let facts = facts(
            "m",
            r#"
pub struct Widget { pub width: u32 }
impl Widget {
    pub fn new() -> Widget { Widget { width: 0 } }
    fn wide(&self) -> u32 { self.width }
}
pub fn free() -> u32 { helper() }
fn helper() -> u32 { 0 }
"#,
        );
        let declared: Vec<&str> = facts.symbols.iter().map(|s| s.fqn.as_str()).collect();
        let considered: Vec<&str> = facts
            .references
            .iter()
            .filter_map(|r| match &r.target {
                Resolution::Unresolved { evidence, .. } => Some(evidence),
                Resolution::Resolved(_) => None,
            })
            .flat_map(|e| e.saw.iter())
            .filter_map(|o| match o {
                Observation::Candidate(fqn) => Some(fqn.as_str()),
                _ => None,
            })
            .collect();

        for expected in [
            // `self.width` inside an impl knows its receiver's type.
            "rust·p·m·Widget·width·field",
            // `Widget { .. }` is a construction of the declared struct.
            "rust·p·m·Widget·ty",
            // `helper()` is a free call in the same module.
            "rust·p·m·helper·val",
        ] {
            assert!(
                considered.contains(&expected),
                "no use site minted {expected}; the use side considered {considered:?}"
            );
            assert!(
                declared.contains(&expected),
                "{expected} was minted by a use site but never by the declaration side, \
                 so the two would never merge"
            );
        }
    }

    /// A macro invocation is a distinct tree-sitter kind from a call, and this
    /// is the recorded decision about it: it IS a reference, of its own kind,
    /// pointing at the macro. What the macro EXPANDS to is not parsed, so uses
    /// inside its token tree are not facts this walk has and none are claimed.
    #[test]
    fn a_macro_invocation_is_a_reference_of_its_own_kind_and_its_body_is_not_read() {
        let facts = facts("m", r#"fn f() { println!("{}", self.hidden().call()); }"#);
        assert_eq!(facts.references.len(), 1, "one invocation, one reference");
        assert_eq!(facts.references[0].kind, RefKind::MacroInvokes);
        let Resolution::Unresolved { evidence, .. } = &facts.references[0].target else {
            panic!("the walk resolves nothing; that is step 5's job");
        };
        assert_eq!(evidence.name, "println");
        assert_eq!(evidence.node_kind, "macro_invocation");
    }

    /// A member access that is read and one that is written are different facts,
    /// and pattern detection reads the difference (R8).
    #[test]
    fn a_member_access_records_whether_it_is_read_or_written() {
        let facts = facts("m", "fn f(w: &mut W) { w.width = 1; let _ = w.height; }");
        let kinds: Vec<RefKind> = facts
            .references
            .iter()
            .filter(|r| matches!(r.kind, RefKind::Reads | RefKind::Writes))
            .map(|r| r.kind)
            .collect();
        assert!(kinds.contains(&RefKind::Writes), "`w.width = 1` writes; got {kinds:?}");
        assert!(kinds.contains(&RefKind::Reads), "`w.height` reads; got {kinds:?}");
    }

    /// Externality comes from the import, never from a symbol being absent from
    /// what has been scanned (spec §2, R6). The walk records the import verbatim
    /// and says which side of the boundary the specifier puts it on; it does not
    /// look at any other file to decide.
    #[test]
    fn an_import_is_recorded_with_what_it_binds_and_a_glob_claims_nothing() {
        let facts = facts(
            "m",
            "use std::collections::HashMap;\nuse serde_json::Value as Json;\nuse crate::db::{PgStore, Row};\nuse super::helper::*;\n",
        );
        let by = |path: &str| {
            facts.imports.iter().find(|i| i.path == path).unwrap_or_else(|| {
                panic!("{path} is missing from {:?}", facts.imports);
            })
        };

        assert_eq!(by("std::collections::HashMap").binds, Binding::Name("HashMap".to_string()));
        assert_eq!(
            by("std::collections::HashMap").origin,
            ImportOrigin::External { package: "std".to_string() }
        );
        assert_eq!(
            by("serde_json::Value as Json").binds,
            Binding::Name("Json".to_string()),
            "an alias binds the alias, not the original name"
        );
        assert_eq!(by("crate::db::PgStore").origin, ImportOrigin::Local);
        assert_eq!(by("crate::db::Row").binds, Binding::Name("Row".to_string()));
        assert_eq!(
            by("super::helper::*").binds,
            Binding::Glob,
            "a glob binds an unknown set, and must never read as proof that it bound a name"
        );
    }

    /// A3's shape, one step early: every reference the corpus produces is
    /// accounted for by exactly one reason, so the histogram sums to the whole
    /// and nothing sits outside it.
    ///
    /// It also records where step 4 leaves things. The walk resolves NOTHING —
    /// placing a name is the shared ladder's job (R7), so every reference here
    /// is `Unresolved`, and `Unplaced` is the bucket that says exactly that.
    /// Step 5 must empty that bucket; if it does not, the ladder is not doing
    /// its job and this test is where that shows.
    #[test]
    fn every_reference_is_accounted_for_by_exactly_one_reason() {
        use std::collections::BTreeMap;

        let mut histogram: BTreeMap<String, usize> = BTreeMap::new();
        let mut total = 0usize;
        for (path, text) in repo_rust_sources() {
            let facts = read(&Source { package: "p", module: "m", text: &text })
                .unwrap_or_else(|e| panic!("{path}: {e:?}"));
            for reference in &facts.references {
                total += 1;
                match &reference.target {
                    Resolution::Resolved(fqn) => panic!(
                        "{path}: the walk resolved {fqn} — resolution is step 5's, not a \
                         language module's (R7)"
                    ),
                    Resolution::Unresolved { reason, evidence } => {
                        assert!(!evidence.name.is_empty(), "{path}: a miss with no name");
                        assert!(
                            !evidence.node_kind.is_empty(),
                            "{path}: a miss the histogram cannot name"
                        );
                        *histogram.entry(format!("{reason:?}")).or_default() += 1;
                    }
                }
            }
        }

        assert_eq!(
            histogram.values().sum::<usize>(),
            total,
            "the reasons must account for 100% of {total} references, not {histogram:?}"
        );
        assert!(total > 10_000, "the corpus produced only {total} references; that is not it");
    }
}
