//! The Java walk — one pass, every declaration and every use site (R1, R2).
//!
//! # The file states its own package
//!
//! Every other adapter is TOLD its package by the caller, which derives it from
//! a manifest and a path. Java writes it down: `package com.sg.dayamed.service;`
//! is the first line, and it is authoritative over anything a directory
//! suggests. So this walk reads it and reports it back in
//! [`FileFacts::package`] rather than echoing what it was handed. The caller's
//! value is the fallback for the default package, which is legal and rare.
//!
//! # Why a Java receiver is usually typable
//!
//! JavaScript needed flow-sensitive binding because a JS binding states no type.
//! Java states one at nearly every binding — fields, parameters, locals — so
//! [`Scope::locals`] is a plain map, filled from the declarations in scope, and
//! `userDetailsService.loadNursePractitioner()` resolves where its JavaScript
//! equivalent would be `ReceiverTypeUnknown`.

use std::collections::BTreeMap;

use tree_sitter::Node;

use super::super::{Home, ReadError, Source, TypeHomes};
use crate::indexer::facts::{
    Binding, DeclaredType, Evidence, FileFacts, Fqn, Import, ImportOrigin, Language, Param, Reason,
    RefKind, Reference, Relation, RelationKind, Resolution, Rung, Span, Symbol, SymbolKind,
    Visibility,
};
use crate::indexer::fqn::{self, Form, FqnError, Reach};

/// Read one Java file.
pub fn read(source: &Source<'_>, types: &TypeHomes) -> Result<FileFacts, ReadError> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_java::LANGUAGE.into())
        .map_err(|e| ReadError::GrammarUnavailable(e.to_string()))?;
    let tree = parser.parse(source.text, None).ok_or(ReadError::NotParsed)?;
    let root = tree.root_node();

    let declared = declared_package(source.text, root);
    let package = declared.as_deref().unwrap_or(source.package);

    let stem = source.path.rsplit('/').next().unwrap_or(source.path).trim_end_matches(".java");
    let file = fqn::define(&Form::Item {
        lang: Language::Java,
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
    walk.imports_of(root);
    let its_own = file.clone();
    let scope = Scope { from: file, container: Container::File, locals: BTreeMap::new() };
    walk.children(&scope, root);

    // THE FILE DECLARES ITS OWN MODULE. The identity already existed and was
    // already the scope every use site here is filed under — it was simply
    // never emitted, so Java's top-level types hung off nothing and
    // `nodes.parent_id` had no value for one.
    //
    // NO IMPORT REFERENCE goes with it, and that is a fact about Java rather
    // than an omission: every Java import names a TYPE (`java.util.List`), so
    // there is no module for one to enter. See
    // `common::specifier_names_a_module`, which refuses Java's `Binding::Name`
    // for exactly this reason.
    walk.symbols.insert(0, super::super::common::file_module(its_own, stem, source.text));

    Ok(FileFacts {
        language: Language::Java,
        package: package.to_string(),
        // EMPTY, always — the package is the whole namespace. See this module's
        // sibling for why deriving a second segment from the directory would be
        // a segment the source never wrote.
        module: String::new(),
        path: source.path.to_string(),
        symbols: walk.symbols,
        references: walk.references,
        relations: walk.relations,
        imports: walk.imports,
    })
}

/// The `package` declaration, if the file has one.
fn declared_package(src: &str, root: Node<'_>) -> Option<String> {
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() == "package_declaration" {
            let mut inner = child.walk();
            for part in child.children(&mut inner) {
                if matches!(part.kind(), "scoped_identifier" | "identifier") {
                    return Some(src[part.byte_range()].to_string());
                }
            }
        }
    }
    None
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

/// What a declaration is nested inside. Java nests types in types, so one kind
/// is enough where Rust needs several.
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
    /// Name -> the type WRITTEN for it: fields of the enclosing class,
    /// parameters, locals. Never inferred — an entry here was read off a
    /// declaration.
    locals: BTreeMap<String, String>,
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

    /// Record that the enclosing type owns this declaration.
    ///
    /// `RelationKind::Owns` is the vocabulary for "this type declares this
    /// member", and it is the fact a member rung needs — no `SymbolKind` filter
    /// substitutes for it, because which kinds are type-owned differs per
    /// language. Java emitted NONE of these and every test passed, because no
    /// check compared the languages on it. 54,192 declarations, zero edges.
    fn owned_by_the_enclosing_type(&mut self, scope: &Scope, child: &Fqn, at: Span) {
        // A member is OWNED by its type; a top-level declaration is CONTAINED by
        // the file. Exactly one of the two, because both feed `nodes.parent_id`
        // and a child with two parents has none.
        //
        // `scope.from` is the right parent either way and that is not a
        // coincidence: at file scope it IS the file's own identity, which is
        // what the file now declares itself under.
        let kind = match &scope.container {
            Container::Type { .. } => RelationKind::Owns,
            Container::File => RelationKind::Contains,
        };
        self.relations.push(Relation {
            kind,
            child: child.clone(),
            // Proven, not guessed: `scope.from` is the identity this walk minted
            // for the enclosing type, from a declaration it read in this file.
            parent: Resolution::Resolved { fqn: scope.from.clone(), via: Rung::DeclaredHere },
            at,
        });
    }

    /// Mint the identity of a declaration in the current container.
    fn declare(&self, scope: &Scope, member: &str, reach: Reach) -> Result<Fqn, FqnError> {
        let lang = Language::Java;
        let package = self.package;
        match &scope.container {
            Container::File => {
                fqn::define(&Form::Item { lang, package, module: "", name: member, reach })
            }
            Container::Type { name: ty } => {
                fqn::define(&Form::Member { lang, package, module: "", ty, member, reach })
            }
        }
    }

    // ── imports ──────────────────────────────────────────────────────────────

    /// Every `import`, as an [`Import`] the ladder binds names through.
    ///
    /// EVERY ONE is reported external, naming the package it came from. A Java
    /// source file states nothing about which side of the boundary a name is
    /// on, and `Ladder::owned_by_this_scan` is what flips the ones this scan
    /// declares — the rung Rust's sibling crates already use.
    fn imports_of(&mut self, root: Node<'_>) {
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            if child.kind() != "import_declaration" {
                continue;
            }
            let at = span(child);
            let wildcard = self.text(child).contains(".*");
            let mut inner = child.walk();
            let Some(path) = child
                .children(&mut inner)
                .find(|c| matches!(c.kind(), "scoped_identifier" | "identifier"))
                .map(|c| self.text(c))
            else {
                continue;
            };
            let (package, binds) = if wildcard {
                (path.to_string(), Binding::Glob)
            } else {
                match path.rsplit_once('.') {
                    Some((head, last)) => (head.to_string(), Binding::Name(last.to_string())),
                    None => (String::new(), Binding::Name(path.to_string())),
                }
            };
            self.imports.push(Import {
                path: path.to_string(),
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
            "interface_declaration" | "annotation_type_declaration" => {
                self.type_declaration(scope, node, SymbolKind::Interface)
            }
            "enum_declaration" => self.type_declaration(scope, node, SymbolKind::Enum),
            "record_declaration" => self.type_declaration(scope, node, SymbolKind::Struct),
            // An annotation element declares a name and a type, which is a
            // method in the grammar's eyes and in ours.
            "method_declaration"
            | "constructor_declaration"
            | "annotation_type_element_declaration" => self.method(scope, node),
            "field_declaration" => self.field_declaration(scope, node),
            "enum_constant" => self.enum_constant(scope, node),
            "local_variable_declaration" => {
                let declared = self.field_text(node, "type").map(str::to_string);
                let mut inner = scope.clone();
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "variable_declarator"
                        && let Some(name) = child.child_by_field_name("name")
                    {
                        // Only when the source WROTE a type. An untyped `var`
                        // binding leaves the name absent from the map, which is
                        // what makes its receiver honestly unknown.
                        if let Some(ty) = &declared {
                            inner.locals.insert(self.text(name).to_string(), ty.clone());
                        }
                    }
                }
                self.type_use(&inner, node, "type");
                self.children(&inner, node);
            }
            _ => {
                self.expression(scope, node);
                self.children(scope, node);
            }
        }
    }

    fn enum_constant(&mut self, scope: &Scope, node: Node<'_>) {
        // `VIDEOCALL(1, ApplicationConstants.CallType.VideoCall.name())` — a
        // constant's ARGUMENTS hold calls and field accesses like any other
        // expression, and declaring the constant without walking into them
        // dropped every one. MEASURED: the head of A2's disagreement list was
        // enums, at 16 emitted against 34 counted.
        self.children(scope, node);
        let Some(name) = self.field_text(node, "name") else { return };
        // [`Reach::Field`], because a field access is the ONLY shape that
        // reaches one: `Status.ACTIVE` is a `field_access`, and this walk
        // records references for `method_invocation`,
        // `object_creation_expression` and `field_access` alone — a bare
        // `ACTIVE` behind `import static` is not a reference it emits, so there
        // is nothing at item reach left to strand.
        //
        // The other half of the field-reach fix, and it has to move WITH it.
        // Threading the use site's reach through `refer_to_member` lands 9,932
        // field edges; leaving the constant at `Reach::Item` would unland 2,856
        // in the same change — dangling edges traded for dangling edges. The
        // reach a declaration is minted at is a fact about how the language
        // reaches it, and Java reaches its enum constants through a dot.
        let Ok(fqn) = self.declare(scope, name, Reach::Field) else { return };
        self.owned_by_the_enclosing_type(scope, &fqn, span(node));
        self.symbols.push(Symbol {
            fqn,
            kind: SymbolKind::EnumVariant,
            name: name.to_string(),
            span: span(node),
            visibility: Visibility::Public,
            docstring: None,
            declared_type: DeclaredType::Unstated,
            params: Vec::new(),
        });
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

        // `extends` and `implements` are the same question the ladder answers
        // for a callee, so they climb the same rungs (R7).
        //
        // THREE clauses, and the third is reached differently on purpose. A
        // `class_declaration` states its parents in the NAMED fields
        // `superclass` and `interfaces`; an `interface_declaration` states its
        // in an `extends_interfaces` child that the grammar gives no field name
        // at all, so `child_by_field_name` finds nothing and every interface
        // read as having no parents. That is not a corner: every Spring Data
        // repository is `interface XRepository extends JpaRepository<..>`, and
        // without the relation there is no way to tell a method inherited from
        // one from a method nothing declares — 2,756 dangling edges in the
        // Dayamed corpus were labelled "unexplained" for exactly this reason.
        let mut cursor = node.walk();
        let mut clauses: Vec<(Node<'_>, RelationKind)> = node
            .children(&mut cursor)
            .filter(|c| c.kind() == "extends_interfaces")
            .map(|c| (c, RelationKind::Extends))
            .collect();
        for (field, relation) in
            [("superclass", RelationKind::Extends), ("interfaces", RelationKind::Implements)]
        {
            if let Some(clause) = node.child_by_field_name(field) {
                clauses.push((clause, relation));
            }
        }
        for (clause, relation) in clauses {
            for named in self.supertypes_named(clause) {
                let raw = self.text(named);
                self.relations.push(Relation {
                    kind: relation,
                    child: fqn.clone(),
                    parent: self.refer_to_type(raw, named),
                    at: span(named),
                });
            }
        }
        self.annotations(scope, node, &fqn);
        self.owned_by_the_enclosing_type(scope, &fqn, span(node));

        let inner = Scope {
            from: fqn,
            container: Container::Type { name: name.to_string() },
            // Every field of this class, shared by every method in it. This is
            // what makes a Java receiver typable where a JavaScript one is not.
            locals: self.fields_of(node),
        };
        if let Some(body) = node.child_by_field_name("body") {
            self.children(&inner, body);
        }
    }

    /// Every `type_identifier` at or under a node, in source order.
    fn type_names_under<'t>(&self, node: Node<'t>) -> Vec<Node<'t>> {
        self.type_names(node, false)
    }

    /// The types a SUPERTYPE clause names — its entries, and not the arguments
    /// they are parameterised by.
    ///
    /// `extends JpaRepository<User, Long>` names ONE supertype. Reading every
    /// type identifier under the clause also records `User` and `Long`, which
    /// says `UserRepository extends Long`: a WRONG edge, which R4 ranks below
    /// no edge and which pattern detection reads as real. `implements
    /// Comparable<Foo>` is the same shape and makes a type its own supertype.
    ///
    /// The argument is not lost as a fact anybody had — a supertype clause
    /// emits RELATIONS and never a reference, so the arguments were only ever
    /// present as the wrong relation.
    fn supertypes_named<'t>(&self, clause: Node<'t>) -> Vec<Node<'t>> {
        self.type_names(clause, true)
    }

    fn type_names<'t>(&self, node: Node<'t>, skip_arguments: bool) -> Vec<Node<'t>> {
        let mut out = Vec::new();
        let mut stack = vec![node];
        while let Some(n) = stack.pop() {
            if skip_arguments && n.kind() == "type_arguments" {
                continue;
            }
            if matches!(n.kind(), "type_identifier" | "scoped_type_identifier") {
                out.push(n);
                continue;
            }
            let mut cursor = n.walk();
            for c in n.children(&mut cursor) {
                stack.push(c);
            }
        }
        out.sort_by_key(|n| n.start_byte());
        out
    }

    /// Every field this type declares, as `name -> declared type`.
    fn fields_of(&self, node: Node<'_>) -> BTreeMap<String, String> {
        let mut out = BTreeMap::new();
        let Some(body) = node.child_by_field_name("body") else { return out };
        let mut cursor = body.walk();
        for member in body.children(&mut cursor) {
            if member.kind() != "field_declaration" {
                continue;
            }
            let Some(ty) = self.field_text(member, "type") else { continue };
            let mut inner = member.walk();
            for declarator in member.children(&mut inner) {
                if declarator.kind() == "variable_declarator"
                    && let Some(name) = declarator.child_by_field_name("name")
                {
                    out.insert(self.text(name).to_string(), ty.to_string());
                }
            }
        }
        out
    }

    fn method(&mut self, scope: &Scope, node: Node<'_>) {
        let Some(name) = self.field_text(node, "name") else { return };
        let Ok(fqn) = self.declare(scope, name, Reach::Item) else { return };
        let returns = match self.field_text(node, "type") {
            Some(t) => DeclaredType::Stated(t.to_string()),
            // A constructor states no return type. Unstated is what the source
            // says, not a value standing in for a failed read.
            None => DeclaredType::Unstated,
        };

        let mut params = Vec::new();
        let mut inner = scope.clone();
        inner.from = fqn.clone();
        if let Some(list) = node.child_by_field_name("parameters") {
            let mut cursor = list.walk();
            let formals: Vec<Node<'_>> = list
                .children(&mut cursor)
                .filter(|p| matches!(p.kind(), "formal_parameter" | "spread_parameter"))
                .collect();
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

        self.symbols.push(Symbol {
            fqn: fqn.clone(),
            kind: SymbolKind::Method,
            name: name.to_string(),
            span: span(node),
            visibility: self.visibility(node),
            docstring: None,
            declared_type: returns,
            params,
        });
        self.owned_by_the_enclosing_type(scope, &fqn, span(node));
        self.annotations(scope, node, &fqn);
        self.type_use(scope, node, "type");
        match node.child_by_field_name("body") {
            Some(body) => self.children(&inner, body),
            // An ANNOTATION ELEMENT has no body — it has a `default` clause, and
            // `OnNullInput onNullInput() default OnNullInput.CALLED` puts a
            // field access there. Walking only the body dropped every one.
            None => {
                let mut cursor = node.walk();
                let rest: Vec<Node<'_>> = node
                    .named_children(&mut cursor)
                    .filter(|c| !matches!(c.kind(), "modifiers" | "formal_parameters"))
                    .filter(|c| Some(c.id()) != node.child_by_field_name("type").map(|t| t.id()))
                    .filter(|c| Some(c.id()) != node.child_by_field_name("name").map(|n| n.id()))
                    .collect();
                for child in rest {
                    self.node(&inner, child);
                }
            }
        }
    }

    fn field_declaration(&mut self, scope: &Scope, node: Node<'_>) {
        let ty = self.field_text(node, "type").map(str::to_string);
        let mut cursor = node.walk();
        let declarators: Vec<Node<'_>> =
            node.children(&mut cursor).filter(|d| d.kind() == "variable_declarator").collect();
        for declarator in declarators {
            let Some(name) = self.field_text(declarator, "name") else { continue };
            let Ok(fqn) = self.declare(scope, name, Reach::Field) else { continue };
            self.symbols.push(Symbol {
                fqn: fqn.clone(),
                kind: SymbolKind::Field,
                name: name.to_string(),
                span: span(declarator),
                visibility: self.visibility(node),
                docstring: None,
                declared_type: match &ty {
                    Some(t) => DeclaredType::Stated(t.clone()),
                    None => DeclaredType::Unstated,
                },
                params: Vec::new(),
            });
            self.owned_by_the_enclosing_type(scope, &fqn, span(declarator));
            self.annotations(scope, node, &fqn);
        }
        self.type_use(scope, node, "type");
        // The DECLARATORS only. `children(node)` would descend into `modifiers`
        // too, and `annotations` above has already walked those arguments — so
        // `@Temporal(TemporalType.TIMESTAMP)` was emitted twice. A2 caught it as
        // the walk emitting MORE than an independent count, which is inventing
        // references rather than dropping them.
        let mut cursor = node.walk();
        let declarators: Vec<Node<'_>> =
            node.children(&mut cursor).filter(|d| d.kind() == "variable_declarator").collect();
        for declarator in declarators {
            self.children(scope, declarator);
        }
    }

    fn visibility(&self, node: Node<'_>) -> Visibility {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() != "modifiers" {
                continue;
            }
            let text = self.text(child);
            if text.contains("public") {
                return Visibility::Public;
            }
            if text.contains("protected") {
                return Visibility::Restricted("protected".to_string());
            }
            if text.contains("private") {
                return Visibility::Private;
            }
        }
        // No modifier is package-private, which is a real named visibility in
        // Java rather than an absence.
        Visibility::Restricted("package".to_string())
    }

    /// `@RestController`, `@Autowired`, `@Entity` — each a `Decorates` relation.
    ///
    /// Annotations are how a Java codebase says which framework it is built on,
    /// so dropping them would lose the single most useful structural fact about
    /// a Spring or JPA class.
    fn annotations(&mut self, scope: &Scope, node: Node<'_>, child: &Fqn) {
        let mut cursor = node.walk();
        let blocks: Vec<Node<'_>> =
            node.children(&mut cursor).filter(|c| c.kind() == "modifiers").collect();
        for modifiers in blocks {
            let mut inner = modifiers.walk();
            let annotations: Vec<Node<'_>> = modifiers
                .children(&mut inner)
                .filter(|a| matches!(a.kind(), "marker_annotation" | "annotation"))
                .collect();
            for annotation in annotations {
                let Some(name) = annotation.child_by_field_name("name") else { continue };
                let raw = self.text(name);
                self.relations.push(Relation {
                    kind: RelationKind::Decorates,
                    child: child.clone(),
                    parent: self.refer_to_type(raw, name),
                    at: span(annotation),
                });
                // An annotation's ARGUMENTS are expressions like any other:
                // `@GetMapping("/rest/" + Controller.API_VERSION)` holds a
                // field access, and recording only the Decorates relation
                // dropped it. Same shape as the enum-constant case above.
                //
                // Everything but the NAME, rather than the `arguments` field:
                // `@Target({ElementType.METHOD})` puts an array initialiser
                // there instead of an argument list, and asking for the field
                // by name missed it.
                let mut args = annotation.walk();
                let rest: Vec<Node<'_>> =
                    annotation.named_children(&mut args).filter(|c| c.id() != name.id()).collect();
                for child in rest {
                    self.node(scope, child);
                }
            }
        }
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

    fn expression(&mut self, scope: &Scope, node: Node<'_>) {
        match node.kind() {
            "method_invocation" => {
                let Some(name) = self.field_text(node, "name") else { return };
                let target = match node.child_by_field_name("object") {
                    // No receiver is a call on `this` or a static import.
                    None => self.unplaced(name, node, Reach::Item),
                    Some(object) => match self.type_of(scope, object) {
                        Some(ty) => self.refer_to_member(&ty, name, node, Reach::Item),
                        None => self.missed(name, node, Reason::ReceiverTypeUnknown, Reach::Item),
                    },
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
            "field_access" => {
                let Some(field) = self.field_text(node, "field") else { return };
                let target =
                    match node.child_by_field_name("object").and_then(|o| self.type_of(scope, o)) {
                        Some(ty) => self.refer_to_member(&ty, field, node, Reach::Field),
                        None => self.missed(field, node, Reason::ReceiverTypeUnknown, Reach::Field),
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
    /// inference — Java writes the type at the binding.
    fn type_of(&self, scope: &Scope, node: Node<'_>) -> Option<String> {
        match node.kind() {
            "this" => match &scope.container {
                Container::Type { name } => Some(name.clone()),
                Container::File => None,
            },
            "identifier" => {
                let name = self.text(node);
                if let Some(ty) = scope.locals.get(name) {
                    return Some(ty.clone());
                }
                // A capitalised bare name is a STATIC call on a type rather than
                // a variable: `Math.max`, `Optional.of`.
                super::names_a_type(name).then(|| name.to_string())
            }
            _ => None,
        }
    }

    // ── minting a reference ──────────────────────────────────────────────────

    /// A name the walk read as a type. `Unplaced`, because the ladder knows the
    /// imports and this does not (R7).
    fn refer_to_type(&self, raw: &str, at: Node<'_>) -> Resolution {
        match super::type_segment(raw) {
            Ok(segment) => self.unplaced(&segment, at, Reach::Item),
            Err(_) => self.missed(raw, at, Reason::UnhandledForm, Reach::Item),
        }
    }

    /// A member of a type this scope named. Placed only when the scan declares
    /// that type; otherwise the bare name goes to the ladder.
    fn refer_to_member(&self, ty: &str, member: &str, at: Node<'_>, reach: Reach) -> Resolution {
        let Ok(ty) = super::type_segment(ty) else {
            return self.missed(member, at, Reason::UnhandledForm, Reach::Item);
        };
        // Only when the scan DECLARES the type. Anything else goes to the
        // ladder as a path, which knows the imports and can name the library.
        //
        // THE REACH IS THE USE SITE'S, not a constant. A declaration's identity
        // ends in the reach its own form minted — a field at `Reach::Field`, a
        // method at `Reach::Item` — so a use site that mints one fixed reach can
        // only ever meet half of them. Minting `Item` here meant a first-party
        // field READ could never meet its own declaration: the two strings
        // differed in exactly one segment. MEASURED on the Dayamed corpus:
        // 22,316 field declarations and ZERO field edges, with 9,932 edges
        // naming a member whose declaration was sitting there under the other
        // reach.
        if matches!(self.types.lookup(self.package, &ty), Home::Ours { .. })
            && let Ok(fqn) = fqn::refer(&Form::Member {
                lang: Language::Java,
                package: self.package,
                module: "",
                ty: &ty,
                member,
                reach,
            })
        {
            return Resolution::Resolved { fqn, via: Rung::DeclaredHere };
        }
        // NOT OURS — so hand the ladder the WHOLE PATH, not just the member.
        //
        // `Mockito.when(...)` is a qualified static call, and dropping the
        // receiver leaves the ladder a bare `when` that no import binds. With
        // the path, `through_an_import` finds `Mockito` bound by
        // `import org.mockito.Mockito` and names the member in that library.
        // MEASURED: `when` 7,169, `any` 5,397, `anyLong` 3,087, `anyString`
        // 2,662 — the whole head of the miss histogram is this shape.
        //
        // Safe here in a way it was not in Rust, where the same handoff sent a
        // path with no import to `rooted_in_this_package` and it came back
        // placed FIRST-PARTY, taking dangling identities from 312 to 683. Java
        // has no path roots at all (`Grammar::roots` is empty), so an unbound
        // path falls through to the same miss it reaches today rather than to a
        // wrong edge. The dangling count is the check, and it is asserted below.
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
