//! The Kotlin walk — one pass, every declaration and every use site (R1, R2).
//!
//! Java's shape, with three differences the language forces.
//!
//! # The grammar states almost nothing in FIELDS
//!
//! Java's and C#'s grammars name a declaration's parts — `name`, `parameters`,
//! `type`. Kotlin's names `name` and little else: a property's identifier, a
//! parameter's type and a delegation's supertype are all unnamed children found
//! by KIND. So this walk reaches for children by kind far more than its
//! siblings do, and the helpers that do it are named for what they are looking
//! for rather than for the field they read.
//!
//! # Top-level declarations are real here
//!
//! A `fun` or a `val` may sit outside any class. [`Container::File`] therefore
//! carries weight it never does in Java, where every declaration is a member of
//! something.
//!
//! # A primary constructor DECLARES
//!
//! `class User(val id: Int, name: String)` declares a property `id` and a plain
//! parameter `name` — the `val`/`var` is the whole difference, and reading only
//! the class body misses the property entirely. The same shape C#'s positional
//! records have, and the same answer.

use std::collections::{BTreeMap, BTreeSet};

use tree_sitter::Node;

use super::super::{Home, ReadError, Source, TypeHomes};
use crate::indexer::facts::{
    Binding, DeclaredType, Evidence, FileFacts, Fqn, Import, ImportOrigin, Language, Param, Reason,
    RefKind, Reference, Relation, RelationKind, Resolution, Rung, Span, Symbol, SymbolKind,
    Visibility,
};
use crate::indexer::fqn::{self, Form, FqnError, Reach};

/// Read one Kotlin file.
pub fn read(source: &Source<'_>, types: &TypeHomes) -> Result<FileFacts, ReadError> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_kotlin_ng::LANGUAGE.into())
        .map_err(|e| ReadError::GrammarUnavailable(e.to_string()))?;
    let tree = parser.parse(source.text, None).ok_or(ReadError::NotParsed)?;
    let root = tree.root_node();

    let declared = declared_package(source.text, root);
    let package = declared.as_deref().unwrap_or(source.package);

    let stem = source
        .path
        .rsplit('/')
        .next()
        .unwrap_or(source.path)
        .trim_end_matches(".kts")
        .trim_end_matches(".kt");
    let file = fqn::define(&Form::Item {
        lang: Language::Kotlin,
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
    let scope = Scope {
        from: file,
        container: Container::File,
        locals: BTreeMap::new(),
        overloaded: BTreeSet::new(),
        fn_depth: 0,
        container_at: 0,
    };
    walk.children(&scope, root);

    walk.symbols.insert(0, super::super::common::file_module(its_own, stem, source.text));

    Ok(FileFacts {
        language: Language::Kotlin,
        package: package.to_string(),
        // EMPTY, always — the package is the whole of where a declaration lives.
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
        if child.kind() == "package_header" {
            let mut inner = child.walk();
            for part in child.children(&mut inner) {
                if matches!(part.kind(), "qualified_identifier" | "identifier") {
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

/// The FIRST child of a kind, which is how this grammar states most things.
fn child_of_kind<'n>(node: Node<'n>, kind: &str) -> Option<Node<'n>> {
    let mut cursor = node.walk();
    node.children(&mut cursor).find(|c| c.kind() == kind)
}

/// The TYPE a declaration states, whichever shape the grammar gave it.
///
/// `String?` is a `nullable_type` and `String` a `user_type`, and reading only
/// the second spelled every nullable parameter `?` — which made an overload's
/// signature `(?,?)` and put two overloads back on one identity.
fn stated_type<'n>(node: Node<'n>) -> Option<Node<'n>> {
    let mut cursor = node.walk();
    node.children(&mut cursor).find(|c| {
        matches!(c.kind(), "user_type" | "nullable_type" | "function_type" | "parenthesized_type")
    })
}

/// The NAME a declaration states.
///
/// The node-types manifest advertises a `name` field and the tree does not use
/// one: a class states its name as a `type_identifier` child and a function as
/// a `simple_identifier`. Reading the field found nothing at all, which is why
/// this is a helper rather than a field access repeated at each site.
fn declared_name<'n>(node: Node<'n>) -> Option<Node<'n>> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|c| matches!(c.kind(), "type_identifier" | "simple_identifier" | "identifier"))
}

/// The function names a type body declares MORE THAN ONCE.
fn overloaded_in(src: &str, type_node: Node<'_>) -> BTreeSet<String> {
    let Some(body) = child_of_kind(type_node, "class_body") else { return BTreeSet::new() };
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    let mut stack = vec![body];
    while let Some(node) = stack.pop() {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "function_declaration" => {
                    if let Some(name) = declared_name(child) {
                        *seen.entry(src[name.byte_range()].to_string()).or_default() += 1;
                    }
                }
                // A member arrives wrapped, so the set is one level down.
                "class_member_declaration" => stack.push(child),
                _ => {}
            }
        }
    }
    seen.into_iter().filter(|(_, n)| *n > 1).map(|(name, _)| name).collect()
}

/// What a declaration is nested inside.
#[derive(Clone)]
enum Container {
    File,
    Type { name: String },
}

#[derive(Clone)]
struct Scope {
    from: Fqn,
    container: Container,
    /// Name -> the type WRITTEN for it. Never inferred.
    locals: BTreeMap<String, String>,
    overloaded: BTreeSet<String>,
    /// How many function BODIES enclose this point.
    fn_depth: usize,
    /// The [`Scope::fn_depth`] at which [`Scope::container`] was established.
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
        let lang = Language::Kotlin;
        let package = self.package;
        // A BODY DOES NOT DECLARE MEMBERS OF THE TYPE IT SITS IN.
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

    // ── imports ──────────────────────────────────────────────────────────────

    /// Every `import`, as an [`Import`] the ladder binds names through.
    ///
    /// EVERY ONE is reported external naming its package; the ladder flips the
    /// ones this scan declares.
    fn imports_of(&mut self, root: Node<'_>) {
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            if child.kind() != "import" {
                continue;
            }
            let at = span(child);
            let raw = self.text(child);
            let wildcard = raw.trim_end().ends_with(".*");
            let mut inner = child.walk();
            let Some(path) = child
                .children(&mut inner)
                .find(|c| matches!(c.kind(), "qualified_identifier" | "identifier"))
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

    fn children(&mut self, scope: &Scope, node: Node<'a>) {
        let mut cursor = node.walk();
        let kids: Vec<Node<'a>> = node.children(&mut cursor).collect();
        for child in kids {
            self.node(scope, child);
        }
    }

    fn node(&mut self, scope: &Scope, node: Node<'a>) {
        match node.kind() {
            "class_declaration" => self.type_declaration(scope, node, SymbolKind::Class),
            // An `object` is a singleton — a type declaration whose name is its
            // own, and the shape Kotlin uses where Java writes a static holder.
            "object_declaration" | "companion_object" => {
                self.type_declaration(scope, node, SymbolKind::Class)
            }
            "function_declaration" => self.function(scope, node),
            "property_declaration" => self.property(scope, node),
            "enum_entry" => self.enum_entry(scope, node),
            _ => {
                self.expression(scope, node);
                self.children(scope, node);
            }
        }
    }

    fn type_declaration(&mut self, scope: &Scope, node: Node<'a>, kind: SymbolKind) {
        // A COMPANION OBJECT may be anonymous, and Kotlin calls that
        // `Companion` — so this does too rather than dropping the declaration.
        let name = match declared_name(node).map(|n| self.text(n)) {
            Some(n) => n.to_string(),
            None if node.kind() == "companion_object" => "Companion".to_string(),
            None => return,
        };
        // An `interface` is spelled as a class_declaration with an `interface`
        // modifier, so the KIND comes from the source's own word.
        let kind = match self.text(node).trim_start() {
            t if t.starts_with("interface") || t.contains(" interface ") => SymbolKind::Interface,
            t if t.starts_with("enum") || t.contains(" enum class ") => SymbolKind::Enum,
            _ => kind,
        };
        let Ok(fqn) = self.declare(scope, &name, Reach::Item) else { return };
        self.symbols.push(Symbol {
            fqn: fqn.clone(),
            kind,
            name: name.clone(),
            span: span(node),
            visibility: self.visibility(node),
            docstring: None,
            declared_type: DeclaredType::Unstated,
            params: Vec::new(),
        });
        self.owned_by_the_enclosing_type(scope, &fqn, span(node));

        // KOTLIN STATES ITS SUPERTYPES IN ONE LIST, like C# and unlike Java: a
        // `delegation_specifiers` clause holds the base class and every
        // interface with one syntax. So an interface's are `Extends` — certain,
        // since an interface has no base class — and a class's are `Implements`,
        // which is wrong for at most one entry per type and never for a type
        // with no base class. The same decision C# records, for the same reason.
        let bases = match kind {
            SymbolKind::Interface => RelationKind::Extends,
            _ => RelationKind::Implements,
        };
        if let Some(list) = child_of_kind(node, "delegation_specifiers") {
            let mut cursor = list.walk();
            let specs: Vec<Node<'_>> = list.children(&mut cursor).collect();
            for spec in specs {
                for named in self.type_names_under(spec) {
                    self.relations.push(Relation {
                        kind: bases,
                        child: fqn.clone(),
                        parent: self.refer_to_type(self.text(named), named),
                        at: span(named),
                    });
                }
            }
        }

        // A NESTED type extends the enclosing name rather than replacing it.
        let nested = match &scope.container {
            Container::Type { name: outer } => format!("{outer}.{name}"),
            Container::File => name.clone(),
        };
        let mut inner = Scope {
            from: fqn,
            container: Container::Type { name: nested },
            locals: self.members_of(node),
            overloaded: overloaded_in(self.src, node),
            fn_depth: scope.fn_depth,
            container_at: scope.fn_depth,
        };

        // A PRIMARY CONSTRUCTOR DECLARES. `class User(val id: Int, name: String)`
        // declares a property `id`; `name` is a plain parameter and declares
        // nothing. The `val`/`var` is the whole difference, and reading only the
        // class body misses the property.
        if let Some(ctor) = child_of_kind(node, "primary_constructor") {
            // The parameters hang off the constructor DIRECTLY — the manifest
            // advertises a `class_parameters` wrapper the tree does not build.
            let mut cursor = ctor.walk();
            let formals: Vec<Node<'a>> =
                ctor.children(&mut cursor).filter(|p| p.kind() == "class_parameter").collect();
            for p in formals {
                for named in self.type_names_under_kind(p, "user_type") {
                    self.references.push(Reference {
                        from: inner.from.clone(),
                        kind: RefKind::TypeUse,
                        at: span(named),
                        target: self.refer_to_type(self.text(named), named),
                    });
                }
                let Some(pname) = declared_name(p).map(|n| self.text(n)) else { continue };
                let declared = stated_type(p).map(|t| self.text(t).to_string());
                if let Some(ty) = &declared {
                    inner.locals.insert(pname.to_string(), ty.clone());
                }
                // `binding_pattern_kind` is the `val`/`var` the source wrote,
                // and its ABSENCE is what makes a parameter a plain one that
                // declares no member.
                if child_of_kind(p, "binding_pattern_kind").is_none() {
                    continue;
                }
                let Ok(member) = self.declare(&inner, pname, Reach::Field) else { continue };
                self.symbols.push(Symbol {
                    fqn: member.clone(),
                    kind: SymbolKind::Property,
                    name: pname.to_string(),
                    span: span(p),
                    visibility: Visibility::Public,
                    docstring: None,
                    declared_type: match declared {
                        Some(t) => DeclaredType::Stated(t),
                        None => DeclaredType::Unstated,
                    },
                    params: Vec::new(),
                });
                self.owned_by_the_enclosing_type(&inner, &member, span(p));
            }
        }

        for body in ["class_body", "enum_class_body"] {
            if let Some(b) = child_of_kind(node, body) {
                self.children(&inner, b);
            }
        }
    }

    /// Every property this type declares, with the type WRITTEN for it.
    fn members_of(&self, node: Node<'_>) -> BTreeMap<String, String> {
        let mut out = BTreeMap::new();
        let Some(body) = child_of_kind(node, "class_body") else { return out };
        let mut stack = vec![body];
        while let Some(n) = stack.pop() {
            let mut cursor = n.walk();
            for child in n.children(&mut cursor) {
                match child.kind() {
                    "class_member_declaration" => stack.push(child),
                    "property_declaration" => {
                        if let Some(decl) = child_of_kind(child, "variable_declaration")
                            && let (Some(name), Some(ty)) = (declared_name(decl), stated_type(decl))
                        {
                            out.insert(self.text(name).to_string(), self.text(ty).to_string());
                        }
                    }
                    _ => {}
                }
            }
        }
        out
    }

    /// Split an OVERLOADED function into the callable a use site mints and the
    /// arm that implements this signature — the shape rust, Java and C# share.
    fn split_into_variant(&mut self, scope: &Scope, node: Node<'_>, symbol: Symbol) -> Symbol {
        if !scope.overloaded.contains(&symbol.name) {
            return symbol;
        }
        let Container::Type { name: ty } = &scope.container else { return symbol };
        let signature = self.signature_of(node);
        let Ok(arm) = fqn::define(&Form::MemberVariant {
            lang: Language::Kotlin,
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

    /// The parameter list, as ONE fqn segment: `(String,Int)`.
    fn signature_of(&self, node: Node<'_>) -> String {
        let Some(list) = child_of_kind(node, "function_value_parameters") else {
            return "()".to_string();
        };
        let mut cursor = list.walk();
        let types: Vec<String> = list
            .children(&mut cursor)
            .filter(|p| p.kind() == "parameter")
            .map(|p| match child_of_kind(p, "type") {
                Some(t) => self.text(t).split_whitespace().collect::<String>(),
                None => "?".to_string(),
            })
            .collect();
        format!("({})", types.join(","))
    }

    fn function(&mut self, scope: &Scope, node: Node<'a>) {
        let Some(name) = declared_name(node).map(|n| self.text(n)) else { return };
        let Ok(fqn) = self.declare(scope, name, Reach::Item) else { return };
        // The return type is the `type` child, which Kotlin writes after the
        // parameters and the grammar gives no field for.
        let returns = match stated_type(node) {
            Some(t) => DeclaredType::Stated(self.text(t).to_string()),
            None => DeclaredType::Unstated,
        };

        let mut params = Vec::new();
        let mut inner = scope.clone();
        inner.fn_depth = scope.fn_depth + 1;
        if let Some(list) = child_of_kind(node, "function_value_parameters") {
            let mut cursor = list.walk();
            let formals: Vec<Node<'_>> =
                list.children(&mut cursor).filter(|p| p.kind() == "parameter").collect();
            for p in formals {
                let Some(pname) = declared_name(p).map(|n| self.text(n)) else {
                    continue;
                };
                let declared_type = match stated_type(p) {
                    Some(t) => {
                        inner.locals.insert(pname.to_string(), self.text(t).to_string());
                        DeclaredType::Stated(self.text(t).to_string())
                    }
                    None => DeclaredType::Unstated,
                };
                params.push(Param {
                    name: pname.to_string(),
                    position: params.len() as u32,
                    declared_type,
                });
                for named in self.type_names_under_kind(p, "user_type") {
                    self.references.push(Reference {
                        from: scope.from.clone(),
                        kind: RefKind::TypeUse,
                        at: span(named),
                        target: self.refer_to_type(self.text(named), named),
                    });
                }
            }
        }

        // A top-level `fun` is a Function; one inside a type is a Method.
        let kind = match &scope.container {
            Container::Type { .. } if scope.fn_depth <= scope.container_at => SymbolKind::Method,
            _ => SymbolKind::Function,
        };
        let symbol = Symbol {
            fqn: fqn.clone(),
            kind,
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
        for named in self.type_names_under_kind(node, "user_type") {
            self.references.push(Reference {
                from: scope.from.clone(),
                kind: RefKind::TypeUse,
                at: span(named),
                target: self.refer_to_type(self.text(named), named),
            });
        }
        if let Some(body) = child_of_kind(node, "function_body") {
            self.children(&inner, body);
        }
    }

    /// A `val` or `var`. At type scope it is a PROPERTY; inside a body it is a
    /// local and declares nothing.
    fn property(&mut self, scope: &Scope, node: Node<'a>) {
        let Some(decl) = child_of_kind(node, "variable_declaration") else {
            self.children(scope, node);
            return;
        };
        let Some(name) = declared_name(decl).map(|n| self.text(n)) else { return };
        let declared = stated_type(decl).map(|t| self.text(t).to_string());

        // A LOCAL IS NOT A NODE — the rule settled for TypeScript, Python and
        // C#. A `val` inside a function body is not a property of the class it
        // sits in, and it is not something a call can target.
        if scope.fn_depth > scope.container_at {
            self.children(scope, node);
            return;
        }

        if let Ok(fqn) = self.declare(scope, name, Reach::Field) {
            self.symbols.push(Symbol {
                fqn: fqn.clone(),
                kind: SymbolKind::Property,
                name: name.to_string(),
                span: span(node),
                visibility: self.visibility(node),
                docstring: None,
                declared_type: match &declared {
                    Some(t) => DeclaredType::Stated(t.clone()),
                    None => DeclaredType::Unstated,
                },
                params: Vec::new(),
            });
            self.owned_by_the_enclosing_type(scope, &fqn, span(node));
        }
        for named in self.type_names_under_kind(decl, "user_type") {
            self.references.push(Reference {
                from: scope.from.clone(),
                kind: RefKind::TypeUse,
                at: span(named),
                target: self.refer_to_type(self.text(named), named),
            });
        }
        self.children(scope, node);
    }

    fn enum_entry(&mut self, scope: &Scope, node: Node<'a>) {
        let Some(name) = declared_name(node).map(|n| self.text(n)) else { return };
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
        // A constant's ARGUMENTS hold calls like any other expression.
        if let Some(args) = child_of_kind(node, "value_arguments") {
            self.children(scope, args);
        }
    }

    /// What the modifiers say. Kotlin's default is PUBLIC, which is the one
    /// place it differs from Java and C# — so an absent modifier is public
    /// because the language says so, not because nothing was read.
    fn visibility(&self, node: Node<'_>) -> Visibility {
        let Some(mods) = child_of_kind(node, "modifiers") else { return Visibility::Public };
        match self.text(mods) {
            t if t.contains("private") => Visibility::Private,
            t if t.contains("internal") || t.contains("protected") => Visibility::Crate,
            _ => Visibility::Public,
        }
    }

    // ── use sites ────────────────────────────────────────────────────────────

    fn type_names_under_kind(&self, node: Node<'a>, _kind: &str) -> Vec<Node<'a>> {
        match stated_type(node) {
            Some(t) => self.type_names_under(t),
            None => Vec::new(),
        }
    }

    /// Every type NAME inside a type expression, including generic arguments.
    fn type_names_under(&self, node: Node<'a>) -> Vec<Node<'a>> {
        let mut out = Vec::new();
        let mut stack = vec![node];
        while let Some(n) = stack.pop() {
            match n.kind() {
                "type_identifier" | "qualified_identifier" => out.push(n),
                _ => {
                    let mut cursor = n.walk();
                    stack.extend(n.named_children(&mut cursor));
                }
            }
        }
        out
    }

    fn expression(&mut self, scope: &Scope, node: Node<'a>) {
        if node.kind() == "call_expression" {
            {
                let mut cursor = node.walk();
                let Some(callee) = node.children(&mut cursor).next() else { return };
                let target = match callee.kind() {
                    // `foo()` — a call on `this` or a top-level function.
                    "identifier" => self.unplaced(self.text(callee), callee, Reach::Item),
                    // `a.b()` — the receiver is the expression before the dot.
                    "navigation_expression" => {
                        let mut inner = callee.walk();
                        let parts: Vec<Node<'_>> = callee.children(&mut inner).collect();
                        let Some(member) = parts.iter().rev().find(|p| p.kind() == "identifier")
                        else {
                            return;
                        };
                        let receiver = parts.first().copied();
                        match receiver.and_then(|r| self.type_of(scope, r)) {
                            Some(ty) => {
                                self.refer_to_member(&ty, self.text(*member), callee, Reach::Item)
                            }
                            None => self.missed(
                                self.text(*member),
                                callee,
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
        }
    }

    /// The type of a receiver, when this scope STATES one.
    fn type_of(&self, scope: &Scope, node: Node<'_>) -> Option<String> {
        match node.kind() {
            "this_expression" => match &scope.container {
                Container::Type { name } => Some(name.clone()),
                Container::File => None,
            },
            "identifier" => scope.locals.get(self.text(node)).cloned(),
            _ => None,
        }
    }

    // ── minting a reference ──────────────────────────────────────────────────

    fn refer_to_type(&self, raw: &str, at: Node<'_>) -> Resolution {
        match super::type_segment(raw) {
            Ok(segment) => self.unplaced(&segment, at, Reach::Item),
            Err(_) => self.missed(raw, at, Reason::UnhandledForm, Reach::Item),
        }
    }

    fn refer_to_member(&self, ty: &str, member: &str, at: Node<'_>, reach: Reach) -> Resolution {
        let Ok(ty) = super::type_segment(ty) else {
            return self.missed(member, at, Reason::UnhandledForm, Reach::Item);
        };
        if matches!(self.types.lookup(self.package, &ty), Home::Tabled { .. })
            && let Ok(fqn) = fqn::refer(&Form::Member {
                lang: Language::Kotlin,
                package: self.package,
                module: "",
                ty: &ty,
                member,
                reach,
            })
        {
            return Resolution::Resolved { fqn, via: Rung::DeclaredHere };
        }
        // NOT OURS — hand the ladder the WHOLE PATH, as Java and C# do. Kotlin
        // has no path roots (`Grammar::roots` is empty), so an unbound path
        // falls through to a miss rather than to a wrong first-party edge.
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

    fn twice(text: &str) -> FileFacts {
        let source = Source { package: "unnamed", module: "", path: "src/F.kt", text };
        let first = read(&source, &TypeHomes::unknown()).expect("the fixture parses");
        let declared = first.package.clone();
        let homes = TypeHomes::of(first.symbols.iter().map(|s| (declared.as_str(), s)));
        read(&source, &homes).expect("the fixture parses")
    }

    fn named(facts: &FileFacts, name: &str) -> Vec<String> {
        facts.symbols.iter().filter(|s| s.name == name).map(|s| s.fqn.to_string()).collect()
    }

    /// The file states its own package, and it is authoritative — Kotlin does
    /// not require the package and the directory to agree.
    #[test]
    fn the_file_states_its_own_package() {
        let facts = twice("package com.sg.alert.ui\n\nclass Screen\n");
        assert_eq!(facts.package, "com.sg.alert.ui");
        assert!(named(&facts, "Screen")[0].contains("com.sg.alert.ui"));
    }

    /// A TOP-LEVEL `fun` is a declaration of the file, which Java has no shape
    /// for at all.
    ///
    /// MUTATION: name it under a type — there is no type, so it mints nothing.
    #[test]
    fn a_top_level_function_is_declared_by_the_file() {
        let facts = twice("package p\n\nfun greet(name: String): String = name\n");
        let found = named(&facts, "greet");
        assert_eq!(found.len(), 1, "one declaration: {found:?}");
        assert!(!found[0].contains("··"), "and it is an item of the package: {found:?}");
    }

    /// A PRIMARY CONSTRUCTOR declares its `val`/`var` parameters and nothing
    /// else. `name` is a plain parameter and declares no member.
    ///
    /// MUTATION: declare every class parameter — `name` becomes a property the
    /// type does not have.
    #[test]
    fn a_primary_constructor_declares_its_val_parameters() {
        let facts = twice("package p\n\nclass User(val id: Int, name: String)\n");
        assert_eq!(named(&facts, "id").len(), 1, "a `val` parameter is a property");
        assert!(named(&facts, "name").is_empty(), "a plain parameter declares nothing");
    }

    /// **RULE ONE.** A nested type is named under the type enclosing it.
    ///
    /// MUTATION: push the leaf name — the two `Config` collapse.
    #[test]
    fn a_nested_type_is_named_under_the_type_that_encloses_it() {
        let facts = twice("package p\n\nclass A { class Config }\n\nclass B { class Config }\n");
        let found = named(&facts, "Config");
        assert_eq!(found.len(), 2, "two declarations: {found:?}");
        let distinct: std::collections::BTreeSet<&String> = found.iter().collect();
        assert_eq!(distinct.len(), 2, "two declarations, two identities: {found:?}");
        // A nested type is a MEMBER of its container, so its identity carries
        // the enclosing type as a segment of its own rather than as a dotted
        // name — `A` then `Config`, not `A.Config`.
        assert!(
            found.iter().any(|f| f.contains("A") && f.ends_with("Config·item")),
            "the enclosing type is a segment of the identity: {found:?}"
        );
    }

    /// **RULE TWO.** A `val` inside a function body is a LOCAL, not a property
    /// of the class — the rule settled for TypeScript, Python and C#.
    ///
    /// MUTATION: drop the depth guard — the two locals become properties of
    /// `Repo` and collide on one identity.
    #[test]
    fn a_local_val_is_not_a_property_of_the_class() {
        let facts = twice(
            "package p\n\nclass Repo {\n  val table = \"repos\"\n  fun find() { val row = 1 }\n  fun save() { val row = 2 }\n}\n",
        );
        assert!(
            named(&facts, "row").is_empty(),
            "a local is not a node: {:?}",
            named(&facts, "row")
        );
        assert_eq!(named(&facts, "table").len(), 1, "a class-body `val` is a property");
    }

    /// An `object` is a singleton TYPE, and an anonymous companion is called
    /// `Companion` — which is what Kotlin calls it.
    #[test]
    fn an_object_is_a_type_and_an_anonymous_companion_is_named() {
        let facts = twice("package p\n\nobject Registry { fun add() {} }\n");
        assert_eq!(named(&facts, "Registry").len(), 1, "an object is a declaration");

        // WRITTEN THE WAY KOTLIN IS WRITTEN. A one-line
        // `class Holder { companion object { ... } }` puts the grammar into
        // error recovery, and reading that as "the grammar cannot parse a
        // companion" would have been wrong: 38 of the 245 files in a real
        // corpus write one and only 4 files in it error at all.
        let comp = twice(
            "package p\n\n\
             class Holder {\n\
             \x20   companion object {\n\
             \x20       fun make() {}\n\
             \x20   }\n\
             }\n",
        );
        assert_eq!(
            named(&comp, "Companion").len(),
            1,
            "an anonymous companion is named `Companion`: {:?}",
            comp.symbols.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
    }

    /// An OVERLOAD set is one callable and one arm per signature.
    #[test]
    fn an_overload_set_is_one_callable_and_an_arm_per_signature() {
        let facts = twice(
            "package p\n\nclass W {\n  fun write(a: String) {}\n  fun write(a: Int, b: Int) {}\n  fun alone(a: String) {}\n}\n",
        );
        let made = named(&facts, "write");
        let distinct: std::collections::BTreeSet<&String> = made.iter().collect();
        assert_eq!(distinct.len(), 3, "the callable plus one arm each: {made:?}");
        assert_eq!(named(&facts, "alone").len(), 1, "an unambiguous function stays one node");
    }
}

#[cfg(test)]
mod probe {
    /// How much of a real Kotlin corpus this grammar can read.
    ///
    ///     SENSEI_CORPUS=/path/to/kotlin cargo test -p senseid --bin senseid \
    ///       kotlin::walk::probe::how_much_parses -- --ignored --nocapture
    #[test]
    #[ignore]
    fn how_much_parses() {
        let Ok(root) = std::env::var("SENSEI_CORPUS") else {
            println!("SENSEI_CORPUS unset — nothing to read.");
            return;
        };
        let mut p = tree_sitter::Parser::new();
        p.set_language(&tree_sitter_kotlin_ng::LANGUAGE.into()).expect("the grammar loads");
        let (mut files, mut clean, mut errored, mut companions) = (0, 0, 0, 0);
        for e in walkdir::WalkDir::new(&root).into_iter().filter_map(Result::ok) {
            let path = e.path();
            if path.extension().is_none_or(|x| x != "kt") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(path) else { continue };
            files += 1;
            if text.contains("companion object") {
                companions += 1;
            }
            let Some(tree) = p.parse(text.as_str(), None) else {
                errored += 1;
                continue;
            };
            if tree.root_node().has_error() {
                errored += 1;
            } else {
                clean += 1;
            }
        }
        println!("\nfiles {files} | clean {clean} | with a parse ERROR {errored}");
        println!("files writing `companion object`: {companions}");
    }
}
