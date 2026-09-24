//! The C# walk — one pass, every declaration and every use site (R1, R2).
//!
//! Java's walk, with four differences the language forces. The shape is
//! deliberately the same: a second structure for a language this close to Java
//! would be two places to fix one bug.
//!
//! # The file states its own namespace
//!
//! `namespace Ethico.Policy.Services` is authoritative over anything a directory
//! suggests, so this walk reads it and reports it back in [`FileFacts::package`]
//! rather than echoing what it was handed. TWO spellings, and both are read: the
//! block form, which nests, and C# 10's file-scoped `namespace X;`, which
//! applies to everything after it.
//!
//! # Properties are declarations, not fields
//!
//! `public int Count { get; set; }` is reached like a field and is CODE. The
//! fact vocabulary carries [`SymbolKind::Property`] for exactly this, and filing
//! one as a field would lose the distinction A5 measures.
//!
//! # Partial types are ONE type
//!
//! `partial class Order` may be declared across several files of a namespace and
//! every part is the same type. They mint one identity, which is correct — and
//! the A7 measurement must not read it as a collision, because it is the
//! language's own answer rather than two declarations fighting over a name.
//!
//! # The two rules built in from the start
//!
//! Every language in this tree has needed both, each discovered by measurement
//! after the fact. They are here from the first line instead:
//!
//! 1. A container is a PATH — `Outer.Inner`, which is also C#'s own spelling.
//! 2. A body does not declare members of the type enclosing it — a local in a
//!    method is not a field of the class.

use std::collections::{BTreeMap, BTreeSet};

use tree_sitter::Node;

use super::super::{Home, ReadError, Source, TypeHomes};
use crate::indexer::facts::{
    Binding, DeclaredType, Evidence, FileFacts, Fqn, Import, ImportOrigin, Language, Param, Reason,
    RefKind, Reference, Relation, RelationKind, Resolution, Rung, Span, Symbol, SymbolKind,
    Visibility,
};
use crate::indexer::fqn::{self, Form, FqnError, Reach};

/// Read one C# file.
pub fn read(source: &Source<'_>, types: &TypeHomes) -> Result<FileFacts, ReadError> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_c_sharp::LANGUAGE.into())
        .map_err(|e| ReadError::GrammarUnavailable(e.to_string()))?;
    let tree = parser.parse(source.text, None).ok_or(ReadError::NotParsed)?;
    let root = tree.root_node();

    let declared = declared_namespace(source.text, root);
    let package = declared.as_deref().unwrap_or(source.package);

    let stem = source.path.rsplit('/').next().unwrap_or(source.path).trim_end_matches(".cs");
    let file = fqn::define(&Form::Item {
        lang: Language::CSharp,
        package,
        module: "",
        name: stem,
        reach: Reach::Mod,
    })
    .map_err(ReadError::NoFileIdentity)?;

    let mut walk = Walk {
        src: source.text,
        package,
        types,
        symbols: Vec::new(),
        references: Vec::new(),
        relations: Vec::new(),
        imports: Vec::new(),
    };
    walk.usings_of(root);
    let its_own = file.clone();
    let scope = Scope {
        from: file,
        container: Container::File,
        locals: BTreeMap::new(),
        overloaded: BTreeSet::new(),
        fn_depth: 0,
        container_at: 0,
    };
    walk.children(&scope, root);

    // THE FILE DECLARES ITS OWN MODULE, so a top-level type hangs off something
    // and `nodes.parent_id` has a value for one.
    //
    // No import reference goes with it: every `using` names a NAMESPACE, and a
    // namespace is not a file this scan can point at.
    walk.symbols.insert(0, super::super::common::file_module(its_own, stem, source.text));

    Ok(FileFacts {
        language: Language::CSharp,
        package: package.to_string(),
        // EMPTY, always — the namespace is the whole of where a declaration
        // lives. See this module's sibling for why a second segment derived from
        // the directory would be one the source never wrote.
        module: String::new(),
        path: source.path.to_string(),
        symbols: walk.symbols,
        references: walk.references,
        relations: walk.relations,
        imports: walk.imports,
    })
}

/// The namespace this file declares, in either spelling.
///
/// The FIRST one, because a file with several block namespaces has no single
/// answer and the first is the one its types are most likely under. A file with
/// none is in the global namespace, which is legal and rare, and the caller's
/// fallback covers it.
fn declared_namespace(src: &str, root: Node<'_>) -> Option<String> {
    fn find(src: &str, node: Node<'_>) -> Option<String> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if matches!(child.kind(), "namespace_declaration" | "file_scoped_namespace_declaration")
                && let Some(name) = child.child_by_field_name("name")
            {
                return Some(src[name.byte_range()].to_string());
            }
        }
        None
    }
    find(src, root)
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

/// The method names a type body declares MORE THAN ONCE.
///
/// Whether a method is an overload is a property of its siblings and the walk
/// meets them one at a time, so the body is scanned once up front. Only the
/// body's OWN members count — a nested type's methods are its own overload set.
fn overloaded_in(src: &str, type_node: Node<'_>) -> BTreeSet<String> {
    let Some(body) = type_node.child_by_field_name("body") else { return BTreeSet::new() };
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    let mut cursor = body.walk();
    for child in body.children(&mut cursor) {
        if !matches!(child.kind(), "method_declaration" | "constructor_declaration") {
            continue;
        }
        if let Some(name) = child.child_by_field_name("name") {
            *seen.entry(src[name.byte_range()].to_string()).or_default() += 1;
        }
    }
    seen.into_iter().filter(|(_, n)| *n > 1).map(|(name, _)| name).collect()
}

/// What a declaration is nested inside. C# nests types in types, so one kind is
/// enough — the same shape Java needs.
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
    /// Name -> the type WRITTEN for it: fields, properties, parameters, locals.
    /// Never inferred — an entry here was read off a declaration.
    locals: BTreeMap<String, String>,
    /// The method names THIS type declares more than once.
    overloaded: BTreeSet<String>,
    /// How many method BODIES enclose this point.
    fn_depth: usize,
    /// The [`Scope::fn_depth`] at which [`Scope::container`] was established.
    /// A body does not declare members of the type it sits in.
    container_at: usize,
}

struct Walk<'a> {
    src: &'a str,
    package: &'a str,
    types: &'a TypeHomes,
    symbols: Vec<Symbol>,
    references: Vec<Reference>,
    relations: Vec<Relation>,
    imports: Vec<Import>,
}

impl<'a> Walk<'a> {
    fn text(&self, node: Node<'_>) -> &'a str {
        &self.src[node.byte_range()]
    }

    fn field_text(&self, node: Node<'_>, name: &str) -> Option<&'a str> {
        node.child_by_field_name(name).map(|c| self.text(c))
    }

    /// Mint the identity of a declaration in the current container.
    fn declare(&self, scope: &Scope, member: &str, reach: Reach) -> Result<Fqn, FqnError> {
        let lang = Language::CSharp;
        let package = self.package;
        // A BODY DOES NOT DECLARE MEMBERS OF THE TYPE IT SITS IN. A local
        // written inside a method is a local of that body; the class declares no
        // such thing and no use site can reach one through it.
        if scope.fn_depth > scope.container_at {
            return fqn::define(&Form::Item { lang, package, module: "", name: member, reach });
        }
        match &scope.container {
            Container::File => {
                fqn::define(&Form::Item { lang, package, module: "", name: member, reach })
            }
            Container::Type { name: ty } => {
                fqn::define(&Form::Member { lang, package, module: "", ty, member, reach })
            }
        }
    }

    /// Record that the enclosing type owns this declaration.
    fn owned_by_the_enclosing_type(&mut self, scope: &Scope, child: &Fqn, at: Span) {
        // Exactly one of the two, because both feed `nodes.parent_id` and a
        // child with two parents has none.
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

    // ── usings ───────────────────────────────────────────────────────────────

    /// Every `using`, as an [`Import`] the ladder binds names through.
    ///
    /// EVERY ONE is reported external, naming the namespace it came from. A C#
    /// source file states nothing about which side of the boundary a name is on,
    /// and `Ladder::owned_by_this_scan` flips the ones this scan declares.
    ///
    /// THREE SHAPES, and they bind different things:
    ///   - `using System.Text;` brings every type of a NAMESPACE into scope,
    ///     which is a glob over that namespace rather than one name.
    ///   - `using static System.Math;` brings a TYPE'S MEMBERS into scope.
    ///   - `using Sb = System.Text.StringBuilder;` binds ONE name to one type.
    ///
    /// Collapsing them would make the plain form claim to import a type called
    /// `Text`, which no declaration anywhere mints.
    fn usings_of(&mut self, root: Node<'_>) {
        let mut found = Vec::new();
        collect(root, &mut found);
        fn collect<'n>(node: Node<'n>, out: &mut Vec<Node<'n>>) {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "using_directive" {
                    out.push(child);
                } else if matches!(
                    child.kind(),
                    "namespace_declaration"
                        | "file_scoped_namespace_declaration"
                        | "declaration_list"
                ) {
                    collect(child, out);
                }
            }
        }

        for child in found {
            let at = span(child);
            let raw = self.text(child);
            // `name` is the ALIAS, not the path. The grammar gives the target as
            // a `type` child in every form, and the field only appears when the
            // source wrote `using X = ...` — so reading the field as the path
            // recorded the alias for one form and nothing for the other two.
            let alias = child.child_by_field_name("name").map(|c| self.text(c));
            let mut cursor = child.walk();
            let names: Vec<&str> = child
                .children(&mut cursor)
                .filter(|c| {
                    matches!(c.kind(), "identifier" | "qualified_name" | "alias_qualified_name")
                })
                .map(|c| self.text(c))
                .collect();
            // The LAST one is the target: an alias form puts the alias first.
            let Some(target) = names.last().copied() else { continue };
            let (package, binds) = if raw.contains("static") {
                // `using static System.Math;` — the members of a TYPE.
                match target.rsplit_once('.') {
                    Some((head, last)) => (
                        head.to_string(),
                        Binding::MemberOf { local: last.to_string(), member: String::new() },
                    ),
                    None => (String::new(), Binding::Name(target.to_string())),
                }
            } else if let Some(alias) = alias {
                // `using Sb = System.Text.StringBuilder;` binds ONE name.
                match target.rsplit_once('.') {
                    Some((head, _)) => (head.to_string(), Binding::Name(alias.to_string())),
                    None => (String::new(), Binding::Name(alias.to_string())),
                }
            } else {
                // The plain form: a whole namespace enters scope.
                (target.to_string(), Binding::Glob)
            };
            self.imports.push(Import {
                path: target.to_string(),
                binds,
                origin: ImportOrigin::External { package },
                at,
            });
        }
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
            "class_declaration" => self.type_declaration(scope, node, SymbolKind::Class),
            "interface_declaration" => self.type_declaration(scope, node, SymbolKind::Interface),
            "struct_declaration" | "record_declaration" | "record_struct_declaration" => {
                self.type_declaration(scope, node, SymbolKind::Struct)
            }
            "enum_declaration" => self.type_declaration(scope, node, SymbolKind::Enum),
            "method_declaration" | "constructor_declaration" => self.method(scope, node),
            "property_declaration" => self.property(scope, node),
            "field_declaration" => self.field_declaration(scope, node),
            "enum_member_declaration" => self.enum_member(scope, node),
            // A BLOCK threads its scope. A local declared in one statement is
            // in scope for every LATER sibling, and cloning the scope per
            // statement — which is what walking children uniformly does — made
            // `Widget w = new Widget(); w.Wide();` two unrelated statements with
            // an untypable receiver in the second.
            "block" => {
                let mut running = scope.clone();
                let mut cursor = node.walk();
                let kids: Vec<Node<'_>> = node.children(&mut cursor).collect();
                for child in kids {
                    if child.kind() == "local_declaration_statement" {
                        self.bind_locals(&mut running, child);
                    }
                    self.node(&running, child);
                }
            }
            "local_declaration_statement" => {
                let mut inner = scope.clone();
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() != "variable_declaration" {
                        continue;
                    }
                    // Only when the source WROTE a type. `var x = ...` leaves
                    // the name absent, which is what makes its receiver honestly
                    // unknown rather than guessed.
                    let declared = self.field_text(child, "type").filter(|t| *t != "var");
                    let mut vars = child.walk();
                    for v in child.children(&mut vars) {
                        if v.kind() == "variable_declarator"
                            && let Some(name) = v.child_by_field_name("name")
                            && let Some(ty) = declared
                        {
                            inner.locals.insert(self.text(name).to_string(), ty.to_string());
                        }
                    }
                    self.type_use(&inner, child, "type");
                }
                self.children(&inner, node);
            }
            _ => {
                self.expression(scope, node);
                self.children(scope, node);
            }
        }
    }

    /// Record the type the source WROTE for each name a declaration binds.
    ///
    /// Extracted so the block arm and the statement arm cannot disagree about
    /// what a declaration binds.
    fn bind_locals(&mut self, scope: &mut Scope, node: Node<'_>) {
        let mut cursor = node.walk();
        let decls: Vec<Node<'_>> =
            node.children(&mut cursor).filter(|c| c.kind() == "variable_declaration").collect();
        for decl in decls {
            // Only when the source WROTE a type. `var x = ...` leaves the name
            // absent, which is what makes its receiver honestly unknown rather
            // than guessed.
            let Some(ty) = self.field_text(decl, "type").filter(|t| *t != "var") else { continue };
            let mut vars = decl.walk();
            for v in decl.children(&mut vars) {
                if v.kind() == "variable_declarator"
                    && let Some(name) = v.child_by_field_name("name")
                {
                    scope.locals.insert(self.text(name).to_string(), ty.to_string());
                }
            }
        }
    }

    fn type_declaration(&mut self, scope: &Scope, node: Node<'_>, kind: SymbolKind) {
        let Some(name) = self.field_text(node, "name") else { return };
        let Ok(fqn) = self.declare(scope, name, Reach::Item) else { return };
        self.symbols.push(Symbol {
            fqn: fqn.clone(),
            kind,
            name: name.to_string(),
            span: span(node),
            visibility: self.visibility(node),
            docstring: None,
            declared_type: DeclaredType::Unstated,
            params: Vec::new(),
        });
        self.owned_by_the_enclosing_type(scope, &fqn, span(node));

        // **C# DOES NOT SAY WHICH BASE IS A CLASS.** `class Order : Entity,
        // IAudited` puts a base class and an interface in ONE list with one
        // syntax, and the grammar gives neither a field of its own. Java states
        // them in `superclass` and `interfaces`; C# states them in a `base_list`.
        //
        // So an INTERFACE's bases are all `Extends`, which the language makes
        // certain — an interface has no base class to confuse them with.
        //
        // A class's are all `Implements`, and that is a measured choice rather
        // than a reading of the syntax. C# allows at most ONE base class and it
        // must come first, so marking the first `Extends` would be right for a
        // class that has one and wrong for every class that implements only
        // interfaces — which is the common shape. `Implements` is wrong for at
        // most one entry per type, and never for a type with no base class.
        //
        // The ambiguity is RESOLVABLE and not resolved here: whether a target is
        // an interface is a fact about the target's own declaration, which the
        // scan holds and a single file does not (R7). That belongs to a later
        // pass, like every other cross-file question.
        let bases = match kind {
            SymbolKind::Interface => RelationKind::Extends,
            _ => RelationKind::Implements,
        };
        let mut cursor = node.walk();
        let list: Vec<Node<'_>> =
            node.children(&mut cursor).filter(|c| c.kind() == "base_list").collect();
        for clause in list {
            let mut inner = clause.walk();
            for base in clause.children(&mut inner) {
                if !matches!(
                    base.kind(),
                    "identifier" | "qualified_name" | "generic_name" | "predefined_type"
                ) {
                    continue;
                }
                let raw = self.text(base);
                self.relations.push(Relation {
                    kind: bases,
                    child: fqn.clone(),
                    parent: self.refer_to_type(raw, base),
                    at: span(base),
                });
            }
        }

        // A NESTED type extends the enclosing type's name rather than replacing
        // it. C# spells it `Outer.Inner`, which is what the language calls it
        // and what a `using` writes.
        let nested = match &scope.container {
            Container::Type { name: outer } => format!("{outer}.{name}"),
            Container::File => name.to_string(),
        };
        let inner = Scope {
            from: fqn,
            container: Container::Type { name: nested },
            locals: self.members_of(node),
            overloaded: overloaded_in(self.src, node),
            fn_depth: scope.fn_depth,
            container_at: scope.fn_depth,
        };
        if let Some(body) = node.child_by_field_name("body") {
            self.children(&inner, body);
        }
    }

    /// Every field and property this type declares, with the type WRITTEN for
    /// it. This is what makes a C# receiver typable where a JavaScript one is
    /// not.
    fn members_of(&self, node: Node<'_>) -> BTreeMap<String, String> {
        let mut out = BTreeMap::new();
        let Some(body) = node.child_by_field_name("body") else { return out };
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            match child.kind() {
                "property_declaration" => {
                    if let (Some(name), Some(ty)) =
                        (self.field_text(child, "name"), self.field_text(child, "type"))
                    {
                        out.insert(name.to_string(), ty.to_string());
                    }
                }
                "field_declaration" => {
                    let mut inner = child.walk();
                    for decl in child.children(&mut inner) {
                        if decl.kind() != "variable_declaration" {
                            continue;
                        }
                        let Some(ty) = self.field_text(decl, "type") else { continue };
                        let mut vars = decl.walk();
                        for v in decl.children(&mut vars) {
                            if v.kind() == "variable_declarator"
                                && let Some(name) = v.child_by_field_name("name")
                            {
                                out.insert(self.text(name).to_string(), ty.to_string());
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        out
    }

    /// Split an OVERLOADED method into the callable a use site mints and the arm
    /// that implements this signature.
    ///
    /// The same shape `java::walk` and `rust::walk` use, for the same reason:
    /// two bodies of one name, told apart by what the SOURCE wrote. The
    /// signature does NOT go in the method's own identity — a call site `Msg(x)`
    /// cannot compose `Msg(String)` without type inference, so that would trade
    /// a lost declaration for an unresolvable call.
    fn split_into_variant(&mut self, scope: &Scope, node: Node<'_>, symbol: Symbol) -> Symbol {
        if !scope.overloaded.contains(&symbol.name) {
            return symbol;
        }
        let Container::Type { name: ty } = &scope.container else { return symbol };
        let signature = self.signature_of(node);
        let Ok(arm) = fqn::define(&Form::MemberVariant {
            lang: Language::CSharp,
            package: self.package,
            module: "",
            ty,
            member: &symbol.name,
            condition: &signature,
            reach: Reach::Item,
        }) else {
            return symbol;
        };
        if !self.symbols.iter().any(|s| s.fqn == symbol.fqn) {
            self.symbols.push(Symbol { fqn: symbol.fqn.clone(), ..symbol.clone() });
        }
        self.relations.push(Relation {
            kind: RelationKind::Variant,
            child: arm.clone(),
            parent: Resolution::Resolved { fqn: symbol.fqn, via: Rung::DeclaredHere },
            at: symbol.span,
        });
        Symbol { fqn: arm, ..symbol }
    }

    /// The parameter list, as ONE fqn segment: `(string,int)`.
    ///
    /// The types the source WROTE, in order, whitespace removed because it is
    /// formatting. `()` for a no-argument overload, which is a real signature
    /// and not an absent one.
    fn signature_of(&self, node: Node<'_>) -> String {
        let Some(list) = node.child_by_field_name("parameters") else { return "()".to_string() };
        let mut cursor = list.walk();
        let types: Vec<String> = list
            .children(&mut cursor)
            .filter(|p| p.kind() == "parameter")
            .map(|p| match self.field_text(p, "type") {
                Some(t) => t.split_whitespace().collect::<String>(),
                None => "?".to_string(),
            })
            .collect();
        format!("({})", types.join(","))
    }

    fn method(&mut self, scope: &Scope, node: Node<'_>) {
        let Some(name) = self.field_text(node, "name") else { return };
        let Ok(fqn) = self.declare(scope, name, Reach::Item) else { return };
        let returns = match self.field_text(node, "returns") {
            Some(t) => DeclaredType::Stated(t.to_string()),
            // A constructor states no return type. Unstated is what the source
            // says, not a value standing in for a failed read.
            None => DeclaredType::Unstated,
        };

        let mut params = Vec::new();
        let mut inner = scope.clone();
        inner.from = fqn.clone();
        // The BODY is one level inside this method.
        inner.fn_depth = scope.fn_depth + 1;
        if let Some(list) = node.child_by_field_name("parameters") {
            let mut cursor = list.walk();
            let formals: Vec<Node<'_>> =
                list.children(&mut cursor).filter(|p| p.kind() == "parameter").collect();
            for p in formals {
                let Some(pname) = self.field_text(p, "name") else { continue };
                let declared_type = match self.field_text(p, "type") {
                    Some(t) => {
                        inner.locals.insert(pname.to_string(), t.to_string());
                        DeclaredType::Stated(t.to_string())
                    }
                    None => DeclaredType::Unstated,
                };
                params.push(Param {
                    name: pname.to_string(),
                    position: params.len() as u32,
                    declared_type,
                });
                self.type_use(scope, p, "type");
            }
        }

        let symbol = Symbol {
            fqn: fqn.clone(),
            kind: SymbolKind::Method,
            name: name.to_string(),
            span: span(node),
            visibility: self.visibility(node),
            docstring: None,
            declared_type: returns,
            params,
        };
        let symbol = self.split_into_variant(scope, node, symbol);
        let fqn = symbol.fqn.clone();
        inner.from = fqn.clone();
        self.symbols.push(symbol);
        self.owned_by_the_enclosing_type(scope, &fqn, span(node));
        self.type_use(scope, node, "returns");
        if let Some(body) = node.child_by_field_name("body") {
            // `node`, not `children` — the body IS the block, and the block arm
            // is what threads a local's type to the statements after it.
            // `children` would iterate the statements directly and skip it.
            self.node(&inner, body);
        }
    }

    /// `public int Count { get; set; }` — reached like a field, and CODE.
    ///
    /// [`SymbolKind::Property`] rather than `Field`, which is the distinction
    /// the fact vocabulary carries for exactly this shape and which A5 measures.
    /// Its accessors are walked, because `get => _count * 2;` holds use sites.
    fn property(&mut self, scope: &Scope, node: Node<'_>) {
        let Some(name) = self.field_text(node, "name") else { return };
        let Ok(fqn) = self.declare(scope, name, Reach::Field) else { return };
        let declared = match self.field_text(node, "type") {
            Some(t) => DeclaredType::Stated(t.to_string()),
            None => DeclaredType::Unstated,
        };
        self.symbols.push(Symbol {
            fqn: fqn.clone(),
            kind: SymbolKind::Property,
            name: name.to_string(),
            span: span(node),
            visibility: self.visibility(node),
            docstring: None,
            declared_type: declared,
            params: Vec::new(),
        });
        self.owned_by_the_enclosing_type(scope, &fqn, span(node));
        self.type_use(scope, node, "type");

        let mut inner = scope.clone();
        inner.from = fqn;
        // An accessor is a BODY, so what it declares is local to it.
        inner.fn_depth = scope.fn_depth + 1;
        if let Some(accessors) = node.child_by_field_name("accessors") {
            self.children(&inner, accessors);
        }
        if let Some(value) = node.child_by_field_name("value") {
            self.children(&inner, value);
        }
    }

    fn field_declaration(&mut self, scope: &Scope, node: Node<'_>) {
        let mut cursor = node.walk();
        let decls: Vec<Node<'_>> =
            node.children(&mut cursor).filter(|c| c.kind() == "variable_declaration").collect();
        for decl in decls {
            let declared = match self.field_text(decl, "type") {
                Some(t) => DeclaredType::Stated(t.to_string()),
                None => DeclaredType::Unstated,
            };
            self.type_use(scope, decl, "type");
            let mut vars = decl.walk();
            let names: Vec<Node<'_>> =
                decl.children(&mut vars).filter(|v| v.kind() == "variable_declarator").collect();
            for v in names {
                let Some(name) = v.child_by_field_name("name").map(|n| self.text(n)) else {
                    continue;
                };
                let Ok(fqn) = self.declare(scope, name, Reach::Field) else { continue };
                self.symbols.push(Symbol {
                    fqn: fqn.clone(),
                    kind: SymbolKind::Field,
                    name: name.to_string(),
                    span: span(v),
                    visibility: self.visibility(node),
                    docstring: None,
                    declared_type: declared.clone(),
                    params: Vec::new(),
                });
                self.owned_by_the_enclosing_type(scope, &fqn, span(v));
                // An initialiser holds use sites like any other expression.
                self.children(scope, v);
            }
        }
    }

    fn enum_member(&mut self, scope: &Scope, node: Node<'_>) {
        let Some(name) = self.field_text(node, "name") else { return };
        let Ok(fqn) = self.declare(scope, name, Reach::Item) else { return };
        self.symbols.push(Symbol {
            fqn: fqn.clone(),
            kind: SymbolKind::EnumVariant,
            name: name.to_string(),
            span: span(node),
            visibility: Visibility::Public,
            docstring: None,
            declared_type: DeclaredType::Unstated,
            params: Vec::new(),
        });
        self.owned_by_the_enclosing_type(scope, &fqn, span(node));
    }

    /// What the modifiers say. C#'s default is `private` for a member and
    /// `internal` for a type, and neither is written — so an absent modifier is
    /// read as private rather than guessed as public.
    fn visibility(&self, node: Node<'_>) -> Visibility {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() != "modifier" {
                continue;
            }
            match self.text(child) {
                "public" => return Visibility::Public,
                "protected" | "internal" => return Visibility::Crate,
                "private" => return Visibility::Private,
                _ => {}
            }
        }
        Visibility::Private
    }

    // ── use sites ────────────────────────────────────────────────────────────

    fn type_use(&mut self, scope: &Scope, node: Node<'_>, field: &str) {
        let Some(ty) = node.child_by_field_name(field) else { return };
        for named in self.type_names_under(ty) {
            let raw = self.text(named);
            self.references.push(Reference {
                from: scope.from.clone(),
                kind: RefKind::TypeUse,
                at: span(named),
                target: self.refer_to_type(raw, named),
            });
        }
    }

    /// Every type NAME inside a type expression, including the arguments of a
    /// generic — `Dictionary<string, Order>` names three.
    fn type_names_under(&self, node: Node<'a>) -> Vec<Node<'a>> {
        let mut out = Vec::new();
        fn visit<'n>(node: Node<'n>, out: &mut Vec<Node<'n>>) {
            match node.kind() {
                "identifier" | "qualified_name" | "predefined_type" => out.push(node),
                "generic_name" => {
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        match child.kind() {
                            "identifier" => out.push(child),
                            _ => visit(child, out),
                        }
                    }
                }
                _ => {
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        visit(child, out);
                    }
                }
            }
        }
        visit(node, &mut out);
        out
    }

    fn expression(&mut self, scope: &Scope, node: Node<'_>) {
        match node.kind() {
            "invocation_expression" => {
                let Some(function) = node.child_by_field_name("function") else { return };
                let target = match function.kind() {
                    // `Foo()` — a call on `this` or a static using.
                    "identifier" => self.unplaced(self.text(function), function, Reach::Item),
                    "member_access_expression" => {
                        let Some(name) = self.field_text(function, "name") else { return };
                        match function
                            .child_by_field_name("expression")
                            .and_then(|o| self.type_of(scope, o))
                        {
                            Some(ty) => self.refer_to_member(&ty, name, function, Reach::Item),
                            None => self.missed(
                                name,
                                function,
                                Reason::ReceiverTypeUnknown,
                                Reach::Item,
                            ),
                        }
                    }
                    _ => return,
                };
                self.references.push(Reference {
                    from: scope.from.clone(),
                    kind: RefKind::Calls,
                    at: span(node),
                    target,
                });
            }
            "object_creation_expression" => {
                let Some(ty) = node.child_by_field_name("type") else { return };
                let raw = self.text(ty);
                self.references.push(Reference {
                    from: scope.from.clone(),
                    kind: RefKind::Constructs,
                    at: span(node),
                    target: self.refer_to_type(raw, ty),
                });
            }
            // A member READ, which is the same shape as a call without the
            // parentheses. The call arm above handles the invoked case, and this
            // one must not fire for it — `children` walks the access inside an
            // invocation, so the guard is the parent's kind.
            "member_access_expression" => {
                if node.parent().is_some_and(|p| {
                    p.kind() == "invocation_expression"
                        && p.child_by_field_name("function").is_some_and(|f| f.id() == node.id())
                }) {
                    return;
                }
                let Some(name) = self.field_text(node, "name") else { return };
                let target = match node
                    .child_by_field_name("expression")
                    .and_then(|o| self.type_of(scope, o))
                {
                    Some(ty) => self.refer_to_member(&ty, name, node, Reach::Field),
                    None => self.missed(name, node, Reason::ReceiverTypeUnknown, Reach::Field),
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
    /// inference — C# writes the type at nearly every binding.
    fn type_of(&self, scope: &Scope, node: Node<'_>) -> Option<String> {
        match node.kind() {
            "this_expression" | "this" => match &scope.container {
                Container::Type { name } => Some(name.clone()),
                Container::File => None,
            },
            "identifier" => {
                let name = self.text(node);
                scope.locals.get(name).cloned()
            }
            _ => None,
        }
    }

    // ── minting a reference ──────────────────────────────────────────────────

    /// A name the walk read as a type. `Unplaced`, because the ladder knows the
    /// usings and this does not (R7).
    fn refer_to_type(&self, raw: &str, at: Node<'_>) -> Resolution {
        match super::type_segment(raw) {
            Ok(segment) => self.unplaced(&segment, at, Reach::Item),
            Err(_) => self.missed(raw, at, Reason::UnhandledForm, Reach::Item),
        }
    }

    /// A member of a type this scope named. Placed only when the scan declares
    /// that type; otherwise the whole path goes to the ladder.
    ///
    /// THE REACH IS THE USE SITE'S, not a constant — a property read arrives at
    /// `Reach::Field` and a method call at `Reach::Item`, and a declaration's
    /// identity ends in the reach its own form minted. Minting one fixed reach
    /// here would mean a first-party property read could never meet its own
    /// declaration: the two strings would differ in exactly one segment. Java
    /// measured that as 22,316 field declarations against ZERO field edges.
    fn refer_to_member(&self, ty: &str, member: &str, at: Node<'_>, reach: Reach) -> Resolution {
        let Ok(ty) = super::type_segment(ty) else {
            return self.missed(member, at, Reason::UnhandledForm, Reach::Item);
        };
        if matches!(self.types.lookup(self.package, &ty), Home::Tabled { .. })
            && let Ok(fqn) = fqn::refer(&Form::Member {
                lang: Language::CSharp,
                package: self.package,
                module: "",
                ty: &ty,
                member,
                reach,
            })
        {
            return Resolution::Resolved { fqn, via: Rung::DeclaredHere };
        }
        // NOT OURS — hand the ladder the WHOLE PATH, not just the member.
        // `Math.Max(..)` dropped to a bare `Max` is a name no using binds; with
        // the path, the ladder finds `Math` in scope and names the member in the
        // framework. Safe here for the reason it is safe in Java: C# has no path
        // roots at all (`Grammar::roots` is empty), so an unbound path falls
        // through to the same miss rather than to a wrong first-party edge.
        Resolution::Unresolved {
            reason: Reason::Unplaced,
            evidence: Evidence {
                name: member.to_string(),
                node_kind: at.kind().to_string(),
                reach: Reach::Item,
                saw: vec![crate::indexer::facts::Observation::UnplacedType(format!(
                    "{ty}.{member}"
                ))],
            },
        }
    }

    fn unplaced(&self, name: &str, at: Node<'_>, reach: Reach) -> Resolution {
        self.missed(name, at, Reason::Unplaced, reach)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Read a fixture the way the corpus does: once to learn where the types
    /// live, then again with those homes, because a member is placed only when
    /// the scan DECLARES its type.
    fn twice(text: &str) -> FileFacts {
        let source = Source { package: "unnamed", module: "", path: "src/F.cs", text };
        let first = read(&source, &TypeHomes::unknown()).expect("the fixture parses");
        let declared = first.package.clone();
        let homes = TypeHomes::of(first.symbols.iter().map(|s| (declared.as_str(), s)));
        read(&source, &homes).expect("the fixture parses")
    }

    fn named(facts: &FileFacts, name: &str) -> Vec<String> {
        facts.symbols.iter().filter(|s| s.name == name).map(|s| s.fqn.to_string()).collect()
    }

    /// A FILE-SCOPED namespace is the file's namespace, exactly as the block
    /// form is. C# 10 made it the default in new projects, so a walk that read
    /// only the block form would file every modern file under the caller's
    /// fallback package.
    ///
    /// MUTATION: drop `file_scoped_namespace_declaration` from
    /// `declared_namespace` — every declaration moves to the fallback package
    /// and nothing in the file resolves to it.
    #[test]
    fn a_file_scoped_namespace_is_the_files_namespace() {
        let facts = twice("namespace Ethico.Policy;\npublic class Order {}\n");
        assert_eq!(facts.package, "Ethico.Policy");
        assert!(
            named(&facts, "Order")[0].contains("Ethico.Policy"),
            "{:?}",
            named(&facts, "Order")
        );

        let block = twice("namespace Ethico.Policy { public class Order {} }\n");
        assert_eq!(block.package, "Ethico.Policy", "the block form reads the same");
    }

    /// A PROPERTY is its own kind, not a field.
    ///
    /// It is reached like a field and is CODE, which is the distinction
    /// `SymbolKind::Property` exists for and that A5 measures. Filing one as a
    /// field would make "how many properties does this type expose" unanswerable.
    ///
    /// MUTATION: emit `SymbolKind::Field` — the property and the backing field
    /// become indistinguishable and the count A5 reports is wrong by the number
    /// of properties in the corpus.
    #[test]
    fn a_property_is_a_property_and_a_field_is_a_field() {
        let facts = twice(
            "namespace P;\npublic class Order {\n  private int _count;\n  public int Count { get; set; }\n}\n",
        );
        let kind = |n: &str| {
            facts.symbols.iter().find(|s| s.name == n).map(|s| s.kind).unwrap_or(SymbolKind::Module)
        };
        assert_eq!(kind("Count"), SymbolKind::Property, "a property");
        assert_eq!(kind("_count"), SymbolKind::Field, "and a field is still a field");
    }

    /// **RULE ONE, built in.** A nested type is named under the type enclosing
    /// it, as `Outer.Inner` — which is also what C# calls it.
    ///
    /// Java and Python each needed this fixed after measurement found it. Here
    /// it is a test written before the corpus was ever read.
    ///
    /// MUTATION: push the leaf name — the two `Value` members collapse onto one
    /// identity.
    #[test]
    fn a_nested_type_is_named_under_the_type_that_encloses_it() {
        let facts = twice(
            "namespace P;\n\
             public class A { public class Options { public int Value; } }\n\
             public class B { public class Options { public int Value; } }\n",
        );
        let values = named(&facts, "Value");
        assert_eq!(values.len(), 2, "two declarations: {values:?}");
        let distinct: std::collections::BTreeSet<&String> = values.iter().collect();
        assert_eq!(distinct.len(), 2, "two declarations, two identities: {values:?}");
        assert!(
            values.iter().any(|f| f.contains("A.Options")),
            "a nested type carries its enclosing type: {values:?}"
        );
    }

    /// **RULE TWO, built in.** A local inside a method is not a member of the
    /// class — the class declares no such thing and no use site can reach one
    /// through it.
    ///
    /// MUTATION: drop the `fn_depth > container_at` guard in `declare` — the two
    /// locals are filed as members of `Repo` and collide on one identity.
    #[test]
    fn a_local_in_a_method_is_not_a_member_of_the_class() {
        let facts = twice(
            "namespace P;\n\
             public class Repo {\n\
             \x20 public int Find() { int row = 1; return row; }\n\
             \x20 public int Save() { int row = 2; return row; }\n\
             }\n",
        );
        let rows = named(&facts, "row");
        let distinct: std::collections::BTreeSet<&String> = rows.iter().collect();
        assert_eq!(distinct.len(), rows.len(), "no two locals share an identity: {rows:?}");
        assert!(
            rows.iter().all(|f| !f.contains("Repo·")),
            "a local is not a member of the class: {rows:?}"
        );
    }

    /// An OVERLOAD SET is one callable and one arm per signature — the shape
    /// rust's `cfg` arms and Java's overloads already take.
    ///
    /// MUTATION: return early from `split_into_variant` — the two `Msg` collapse
    /// onto one identity, which is what A7 reports.
    #[test]
    fn an_overload_set_is_one_callable_and_an_arm_per_signature() {
        let facts = twice(
            "namespace P;\n\
             public class C {\n\
             \x20 public void Msg(string a) {}\n\
             \x20 public void Msg(int a, int b) {}\n\
             \x20 public void Alone(string a) {}\n\
             }\n",
        );
        let msg = named(&facts, "Msg");
        let distinct: std::collections::BTreeSet<&String> = msg.iter().collect();
        assert_eq!(distinct.len(), 3, "the callable plus one arm each: {msg:?}");
        assert_eq!(
            facts.relations.iter().filter(|r| r.kind == RelationKind::Variant).count(),
            2,
            "one Variant relation per arm"
        );
        let alone = named(&facts, "Alone");
        assert_eq!(alone.len(), 1, "an unambiguous method stays one node: {alone:?}");
        assert!(!alone[0].contains('('), "and carries no signature: {alone:?}");
    }

    /// A PARTIAL type is ONE type, and two files declaring it mint one identity
    /// — which is the language's own answer, not a collision.
    ///
    /// Stated as a test because the A7 measurement must not read it as a defect:
    /// it is the one shape where two declarations SHOULD share a name.
    #[test]
    fn two_parts_of_a_partial_type_mint_one_identity() {
        let a = twice("namespace P;\npublic partial class Order { public int Id; }\n");
        let b = twice("namespace P;\npublic partial class Order { public int Total; }\n");
        assert_eq!(named(&a, "Order"), named(&b, "Order"), "both parts are the same type");
    }

    /// The three `using` shapes bind different things, and collapsing them would
    /// make the plain form claim to import a type called `Text`.
    ///
    /// MUTATION: emit `Binding::Name` for the plain form — every `using` then
    /// binds the namespace's last segment as if it were a type, and no
    /// declaration anywhere mints that name.
    #[test]
    fn a_using_binds_what_its_shape_says_it_binds() {
        let facts = twice(
            "using System.Text;\nusing static System.Math;\nusing Sb = System.Text.StringBuilder;\nnamespace P;\npublic class C {}\n",
        );
        let by_path = |p: &str| facts.imports.iter().find(|i| i.path == p).map(|i| i.binds.clone());
        assert!(
            matches!(by_path("System.Text"), Some(Binding::Glob)),
            "a plain using brings a whole NAMESPACE into scope: {:?}",
            by_path("System.Text")
        );
        assert!(
            matches!(by_path("System.Math"), Some(Binding::MemberOf { .. })),
            "`using static` brings a TYPE's members: {:?}",
            by_path("System.Math")
        );
        assert!(
            matches!(by_path("System.Text.StringBuilder"), Some(Binding::Name(ref n)) if n == "Sb"),
            "an alias binds ONE name: {:?}",
            by_path("System.Text.StringBuilder")
        );
    }

    /// An interface's bases are `Extends`; a class's are `Implements`.
    ///
    /// C# gives both ONE syntax, so this is a decision the module header
    /// records rather than a reading of the grammar: at most one base of a class
    /// is a class, it must come first, and nothing in the syntax says whether
    /// there is one. `Implements` is wrong for at most one entry per type and
    /// never for a type with no base class, which is the common shape.
    #[test]
    fn a_base_list_is_read_by_what_declares_it() {
        let facts = twice(
            "namespace P;\npublic interface IAudited {}\npublic interface ITimed : IAudited {}\npublic class Order : IAudited {}\n",
        );
        let kinds: Vec<RelationKind> = facts
            .relations
            .iter()
            .filter(|r| matches!(r.kind, RelationKind::Extends | RelationKind::Implements))
            .map(|r| r.kind)
            .collect();
        assert!(kinds.contains(&RelationKind::Extends), "the interface extends: {kinds:?}");
        assert!(kinds.contains(&RelationKind::Implements), "the class implements: {kinds:?}");
    }

    /// A receiver whose type the source WROTE is typable, which is what makes a
    /// C# call graph better than a JavaScript one.
    #[test]
    fn a_written_type_makes_its_receiver_resolvable() {
        let facts = twice(
            "namespace P;\n\
             public class Widget { public int Wide() { return 1; } }\n\
             public class Use { public int Go() { Widget w = new Widget(); return w.Wide(); } }\n",
        );
        let calls: Vec<String> = facts
            .references
            .iter()
            .filter(|r| r.kind == RefKind::Calls)
            .filter_map(|r| match &r.target {
                Resolution::Resolved { fqn, .. } => Some(fqn.to_string()),
                Resolution::Unresolved { .. } => None,
            })
            .collect();
        assert!(
            calls.iter().any(|c| c.contains("Widget") && c.contains("Wide")),
            "the written type places the call: {calls:?}"
        );
    }
}
