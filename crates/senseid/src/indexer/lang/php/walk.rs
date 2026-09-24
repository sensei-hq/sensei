//! The PHP walk — one pass, every declaration and every use site (R1, R2).
//!
//! # The file states its own namespace
//!
//! Like Java, and unlike every adapter that is TOLD its package: `namespace
//! App\Domain;` is written in the file and is authoritative over anything the
//! directory suggests. PSR-4 makes the two agree by convention, and a
//! convention belongs to the autoloader.
//!
//! A file may open SEVERAL braced namespace blocks. That is legal, vanishingly
//! rare in hand-written code, and cheap to get right — so [`Walk::package`] is
//! updated on entering a block rather than fixed for the file, and a
//! declaration is minted under the namespace actually in force at it.
//!
//! # `$this->x` and `$x` are DIFFERENT names
//!
//! Java folds a class's fields into the same map as its locals, because
//! `field` and `field` are spelled alike and the innermost binding wins. PHP
//! spells a property `$this->x` and a local `$x`, so they cannot shadow each
//! other and they get two maps — [`Scope::locals`] and [`Scope::fields`].
//! Nothing has to be resolved by precedence, because the source already said
//! which one it meant.
//!
//! # A body declares GLOBALLY
//!
//! `function helper() {}` written inside a method body does not declare a
//! member of the enclosing class. PHP declares it in the GLOBAL namespace, at
//! the moment the enclosing function runs. So [`Scope::in_body`] sends such a
//! declaration to the file container — which is both PHP's own rule and the
//! second of the two rules every adapter here has needed.

use std::collections::BTreeMap;

use tree_sitter::Node;

use super::super::{Home, ReadError, Source, TypeHomes};
use crate::indexer::facts::{
    Binding, DeclaredType, Evidence, FileFacts, Fqn, Import, ImportOrigin, Language, Observation,
    Param, Reason, RefKind, Reference, Relation, RelationKind, Resolution, Rung, Span, Symbol,
    SymbolKind, Visibility,
};
use crate::indexer::fqn::{self, Form, FqnError, Reach};

/// Read one PHP file.
pub fn read(source: &Source<'_>, types: &TypeHomes) -> Result<FileFacts, ReadError> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_php::LANGUAGE_PHP.into())
        .map_err(|e| ReadError::GrammarUnavailable(e.to_string()))?;
    let tree = parser.parse(source.text, None).ok_or(ReadError::NotParsed)?;
    let root = tree.root_node();

    // The file's OWN namespace, which is the first one it opens. A second
    // braced block changes the namespace for what is inside it, and the walk
    // follows that — but the file is homed where it starts.
    let declared = declared_namespace(source.text, root);
    let package = declared.unwrap_or_else(|| source.package.to_string());

    let stem = source.path.rsplit('/').next().unwrap_or(source.path).trim_end_matches(".php");
    let file = fqn::define(&Form::Item {
        lang: Language::Php,
        package: &package,
        module: "",
        name: stem,
        reach: Reach::Mod,
    })
    .map_err(ReadError::NoFileIdentity)?;

    let mut walk = Walk {
        src: source.text,
        package: package.clone(),
        types,
        symbols: Vec::new(),
        references: Vec::new(),
        relations: Vec::new(),
        imports: Vec::new(),
    };
    let its_own = file.clone();
    let scope = Scope {
        from: file,
        container: Container::File,
        locals: BTreeMap::new(),
        fields: BTreeMap::new(),
        in_body: false,
    };
    walk.children(&scope, root);

    // The file declares its own module, so its top-level declarations hang off
    // something and `nodes.parent_id` has a value for one. Same as Java.
    //
    // NO IMPORT REFERENCE goes with it: every PHP `use` binds a CLASS, a
    // function or a constant, never a namespace as a thing you enter — there is
    // no module for a reference to reach.
    walk.symbols.insert(0, super::super::common::file_module(its_own, stem, source.text));

    Ok(FileFacts {
        language: Language::Php,
        package,
        // EMPTY, always — the namespace is the whole of where a declaration
        // lives, exactly as it is in Java, C# and Kotlin.
        module: String::new(),
        path: source.path.to_string(),
        symbols: walk.symbols,
        references: walk.references,
        relations: walk.relations,
        imports: walk.imports,
    })
}

/// The first `namespace` the file opens, if it opens one.
///
/// A file with none is in the GLOBAL namespace, which is what the caller's
/// value stands for — not a failed read.
fn declared_namespace(src: &str, root: Node<'_>) -> Option<String> {
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if node.kind() == "namespace_definition" {
            return node.child_by_field_name("name").map(|n| src[n.byte_range()].to_string());
        }
        let mut cursor = node.walk();
        let kids: Vec<Node<'_>> = node.children(&mut cursor).collect();
        for child in kids.into_iter().rev() {
            stack.push(child);
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

/// What a declaration is nested inside. PHP nests a class in a namespace and
/// members in a class, and does not nest a class in a class — so one kind is
/// enough.
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
    /// `$x` -> the type WRITTEN for it: a typed parameter, a promoted
    /// constructor property, a caught exception, a `$x = new Foo` binding.
    /// Never inferred beyond what the source spelled.
    locals: BTreeMap<String, String>,
    /// `$this->x` -> the type written on the property's declaration. A SECOND
    /// map rather than entries in `locals`, because PHP spells the two
    /// differently and so they cannot collide.
    fields: BTreeMap<String, String>,
    /// Whether this scope is a function or method BODY.
    ///
    /// A `function` or `class` written inside one declares into the GLOBAL
    /// namespace when the body runs — PHP's own rule — so it must not be minted
    /// as a member of whatever type encloses the body.
    in_body: bool,
}

struct Walk<'a> {
    src: &'a str,
    /// The namespace in force. Updated on entering a braced `namespace` block,
    /// because a file may open more than one and a declaration belongs to the
    /// one it is written in.
    package: String,
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

    /// The first child of a kind — the grammar states plenty of things as an
    /// unnamed child rather than a field.
    fn child_of_kind<'t>(&self, node: Node<'t>, kind: &str) -> Option<Node<'t>> {
        let mut cursor = node.walk();
        node.children(&mut cursor).find(|c| c.kind() == kind)
    }

    /// Record that the enclosing type owns this declaration.
    ///
    /// Exactly one of `Owns` and `Contains`, because both feed
    /// `nodes.parent_id` and a child with two parents has none.
    fn owned_by_the_enclosing_type(&mut self, scope: &Scope, child: &Fqn, at: Span) {
        let kind = match (&scope.container, scope.in_body) {
            // A body's declaration is global, so the file contains it.
            (_, true) | (Container::File, _) => RelationKind::Contains,
            (Container::Type { .. }, false) => RelationKind::Owns,
        };
        self.relations.push(Relation {
            kind,
            child: child.clone(),
            parent: Resolution::Resolved { fqn: scope.from.clone(), via: Rung::DeclaredHere },
            at,
        });
    }

    /// Mint the identity of a declaration in the container actually in force.
    fn declare(&self, scope: &Scope, member: &str, reach: Reach) -> Result<Fqn, FqnError> {
        let lang = Language::Php;
        let package = self.package.as_str();
        match &scope.container {
            Container::Type { name: ty } if !scope.in_body => {
                fqn::define(&Form::Member { lang, package, module: "", ty, member, reach })
            }
            _ => fqn::define(&Form::Item { lang, package, module: "", name: member, reach }),
        }
    }

    /// The reduction a use site applies to a written name.
    ///
    /// ONE rule for types and functions alike, because PHP reaches both the
    /// same way: `\App\Models\User` and `\App\Support\slugify` are the same
    /// shape, and the last segment is the name in both. Sharing it is what
    /// keeps the two from drifting.
    fn named(&self, raw: &str, at: Node<'_>, reach: Reach) -> Resolution {
        match super::type_segment(raw) {
            Ok(segment) => self.missed(&segment, at, Reason::Unplaced, reach),
            Err(_) => self.missed(raw, at, Reason::UnhandledForm, reach),
        }
    }

    // ── imports ──────────────────────────────────────────────────────────────

    /// One `use` clause, as an [`Import`] the ladder binds a name through.
    ///
    /// `prefix` is the group's shared head (`use App\Models\{User, Post};`) and
    /// empty for the ordinary single form.
    fn import_clause(&mut self, clause: Node<'_>, prefix: &str, at: Span) {
        let mut cursor = clause.walk();
        let Some(path) = clause
            .children(&mut cursor)
            .find(|c| matches!(c.kind(), "qualified_name" | "name"))
            .map(|c| self.text(c))
        else {
            return;
        };
        let full = if prefix.is_empty() { path.to_string() } else { format!("{prefix}\\{path}") };
        // A leading `\` says "from the global namespace", which is where an
        // absolute `use` already starts. Keeping it would put an empty first
        // segment in every path the ladder splits.
        let full = full.trim_start_matches('\\').to_string();

        // `use App\Models\User as Customer;` binds the ALIAS. The import's path
        // still names the target, so the ladder reaches the same declaration.
        let alias = clause.child_by_field_name("alias").map(|a| self.text(a).to_string());
        let last = full.rsplit('\\').next().unwrap_or(&full).to_string();
        let (package, name) = match full.rsplit_once('\\') {
            Some((head, _)) => (head.to_string(), last),
            // A single-segment `use Foo;` names the global namespace, which no
            // package spells. `php` is where the global names live.
            None => ("php".to_string(), last),
        };
        self.imports.push(Import {
            path: full,
            binds: Binding::Name(alias.unwrap_or(name)),
            // EXTERNAL naming the namespace it came from, and
            // `Ladder::owned_by_this_scan` is what flips the ones this scan
            // declares — the same handoff Java makes.
            origin: ImportOrigin::External { package },
            at,
        });
    }

    fn import_declaration(&mut self, node: Node<'_>) {
        let at = span(node);
        let prefix = match self.child_of_kind(node, "namespace_name") {
            // A GROUP shares a head: `use App\Models\{User, Post};`. The head is
            // the `namespace_name` child and the clauses hang off the `body`.
            Some(head) => self.text(head).trim_start_matches('\\'),
            // The ordinary single form has NO head. An absence the grammar
            // states, written as one rather than defaulted into one — a `use`
            // whose head could not be read must not silently become a `use` at
            // the root.
            None => "",
        };
        if let Some(body) = node.child_by_field_name("body") {
            let mut cursor = body.walk();
            let clauses: Vec<Node<'_>> =
                body.children(&mut cursor).filter(|c| c.kind() == "namespace_use_clause").collect();
            for clause in clauses {
                self.import_clause(clause, prefix, at);
            }
            return;
        }
        let mut cursor = node.walk();
        let clauses: Vec<Node<'_>> =
            node.children(&mut cursor).filter(|c| c.kind() == "namespace_use_clause").collect();
        for clause in clauses {
            self.import_clause(clause, "", at);
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
            "namespace_definition" => {
                // The namespace in force changes HERE, for everything inside.
                if let Some(name) = self.field_text(node, "name") {
                    self.package = name.trim_start_matches('\\').to_string();
                }
                self.children(scope, node);
            }
            "namespace_use_declaration" => self.import_declaration(node),
            "class_declaration" => self.type_declaration(scope, node, SymbolKind::Class),
            "interface_declaration" => self.type_declaration(scope, node, SymbolKind::Interface),
            "trait_declaration" => self.type_declaration(scope, node, SymbolKind::Trait),
            "enum_declaration" => self.type_declaration(scope, node, SymbolKind::Enum),
            "method_declaration" => self.callable(scope, node, SymbolKind::Method),
            "function_definition" => self.callable(scope, node, SymbolKind::Function),
            "property_declaration" => self.property_declaration(scope, node),
            "const_declaration" => self.const_declaration(scope, node),
            "enum_case" => self.enum_case(scope, node),
            // `use SomeTrait;` INSIDE a class body — a different `use` from the
            // import at file scope, spelled the same way and told apart by
            // where it sits. The import form is `namespace_use_declaration`.
            "use_declaration" => self.trait_use(scope, node),
            "assignment_expression" => {
                // `$repo = new UserRepository()` is the one binding PHP types
                // without a type annotation, and it is everywhere. The type
                // comes off the `new`, which the SOURCE wrote.
                let mut inner = scope.clone();
                if let (Some(left), Some(right)) =
                    (node.child_by_field_name("left"), node.child_by_field_name("right"))
                    && left.kind() == "variable_name"
                    && right.kind() == "object_creation_expression"
                    && let Some(constructed) = self.constructed_type(right)
                {
                    inner.locals.insert(self.variable(left).to_string(), constructed);
                }
                self.expression(&inner, node);
                self.children(&inner, node);
            }
            "catch_clause" => {
                let mut inner = scope.clone();
                if let (Some(ty), Some(name)) =
                    (node.child_by_field_name("type"), node.child_by_field_name("name"))
                {
                    // `catch (FooException | BarException $e)` states a union.
                    // The FIRST is bound here and the rest are emitted as type
                    // uses below, so nothing is dropped.
                    if let Some(first) = self.type_names_under(ty).first() {
                        inner
                            .locals
                            .insert(self.variable(name).to_string(), self.text(*first).to_string());
                    }
                }
                self.type_use(scope, node, "type");
                self.children(&inner, node);
            }
            _ => {
                self.expression(scope, node);
                self.children(scope, node);
            }
        }
    }

    /// A `variable_name` without its `$`. The map is keyed on the NAME, because
    /// `$this->x` and `$x` reach different things and carrying the sigil into
    /// one of them would only make the two look alike.
    fn variable(&self, node: Node<'_>) -> &'a str {
        self.text(node).trim_start_matches('$')
    }

    fn type_declaration(&mut self, scope: &Scope, node: Node<'_>, kind: SymbolKind) {
        let Some(name) = self.field_text(node, "name") else { return };
        let Ok(fqn) = self.declare(scope, name, Reach::Item) else { return };
        self.symbols.push(Symbol {
            fqn: fqn.clone(),
            kind,
            name: name.to_string(),
            span: span(node),
            // A PHP type is public. There is no package-private class and no
            // `internal` — the only visibility modifiers PHP has apply to
            // members.
            visibility: Visibility::Public,
            docstring: None,
            declared_type: DeclaredType::Unstated,
            params: Vec::new(),
        });

        // PHP states the two SEPARATELY, so the relation is READ rather than
        // decided. C# and Kotlin put a base class and an interface in one list
        // with one syntax and each had to record a choice; `base_clause` is
        // `extends` and `class_interface_clause` is `implements`, always.
        //
        // An INTERFACE extending several interfaces still writes `extends`, so
        // its `base_clause` carries more than one name and all of them are
        // `Extends`. That is what the source says.
        for (kind, relation) in [
            ("base_clause", RelationKind::Extends),
            ("class_interface_clause", RelationKind::Implements),
        ] {
            let Some(clause) = self.child_of_kind(node, kind) else { continue };
            let mut cursor = clause.walk();
            let named: Vec<Node<'_>> = clause
                .children(&mut cursor)
                .filter(|c| matches!(c.kind(), "name" | "qualified_name"))
                .collect();
            for parent in named {
                let raw = self.text(parent);
                self.relations.push(Relation {
                    kind: relation,
                    child: fqn.clone(),
                    parent: self.refer_to_type(raw, parent),
                    at: span(parent),
                });
            }
        }
        self.attributes(scope, node, &fqn);
        self.owned_by_the_enclosing_type(scope, &fqn, span(node));

        let inner = Scope {
            from: fqn,
            container: Container::Type { name: name.to_string() },
            locals: BTreeMap::new(),
            // Every property of this class, shared by every method in it —
            // what makes `$this->service->find()` typable.
            fields: self.properties_of(node),
            in_body: false,
        };
        if let Some(body) = node.child_by_field_name("body") {
            self.children(&inner, body);
        }
    }

    /// Every property this type declares, as `name -> declared type`.
    ///
    /// Both shapes: the ordinary `private Foo $bar;` and the PROMOTED
    /// constructor parameter `__construct(private Foo $bar)`. Missing the
    /// second cost C# and Kotlin real declarations, and it is the dominant
    /// shape in modern PHP.
    fn properties_of(&self, node: Node<'_>) -> BTreeMap<String, String> {
        let mut out = BTreeMap::new();
        let Some(body) = node.child_by_field_name("body") else { return out };
        let mut cursor = body.walk();
        let members: Vec<Node<'_>> = body.children(&mut cursor).collect();
        for member in members {
            match member.kind() {
                "property_declaration" => {
                    let Some(ty) = self.field_text(member, "type") else { continue };
                    let mut inner = member.walk();
                    for element in member.children(&mut inner) {
                        if element.kind() == "property_element"
                            && let Some(name) = element.child_by_field_name("name")
                        {
                            out.insert(self.variable(name).to_string(), ty.to_string());
                        }
                    }
                }
                "method_declaration" => {
                    let Some(list) = member.child_by_field_name("parameters") else { continue };
                    let mut inner = list.walk();
                    for p in list.children(&mut inner) {
                        if p.kind() == "property_promotion_parameter"
                            && let (Some(name), Some(ty)) =
                                (p.child_by_field_name("name"), self.field_text(p, "type"))
                        {
                            out.insert(self.variable(name).to_string(), ty.to_string());
                        }
                    }
                }
                _ => {}
            }
        }
        out
    }

    fn callable(&mut self, scope: &Scope, node: Node<'_>, kind: SymbolKind) {
        let Some(name) = self.field_text(node, "name") else { return };
        let Ok(fqn) = self.declare(scope, name, Reach::Item) else { return };
        let returns = match self.field_text(node, "return_type") {
            Some(t) => DeclaredType::Stated(t.to_string()),
            None => DeclaredType::Unstated,
        };

        let mut params = Vec::new();
        let mut inner = scope.clone();
        inner.from = fqn.clone();
        inner.in_body = true;
        if let Some(list) = node.child_by_field_name("parameters") {
            let mut cursor = list.walk();
            let formals: Vec<Node<'_>> = list
                .children(&mut cursor)
                .filter(|p| {
                    matches!(
                        p.kind(),
                        "simple_parameter" | "variadic_parameter" | "property_promotion_parameter"
                    )
                })
                .collect();
            for p in formals {
                let Some(pname) = p.child_by_field_name("name") else { continue };
                let pname = self.variable(pname);
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
                    declared_type: declared_type.clone(),
                });
                self.type_use(scope, p, "type");
                // A PROMOTED parameter declares a property as well as taking an
                // argument. Both facts are true and both are emitted: the
                // parameter above, and the property here.
                if p.kind() == "property_promotion_parameter"
                    && let Ok(prop) = self.declare(scope, pname, Reach::Field)
                {
                    self.symbols.push(Symbol {
                        fqn: prop.clone(),
                        kind: SymbolKind::Field,
                        name: pname.to_string(),
                        span: span(p),
                        visibility: self.visibility(p),
                        docstring: None,
                        declared_type,
                        params: Vec::new(),
                    });
                    self.owned_by_the_enclosing_type(scope, &prop, span(p));
                }
            }
        }

        // NO OVERLOAD SPLIT, and that is a fact about PHP rather than an
        // omission. Java, C# and Kotlin each needed a callable plus one arm per
        // signature because two declarations of one name are legal there. PHP
        // refuses them outright — `Cannot redeclare` is a fatal error — so a
        // name in a class body belongs to exactly one declaration and there is
        // nothing to split.
        self.symbols.push(Symbol {
            fqn: fqn.clone(),
            kind,
            name: name.to_string(),
            span: span(node),
            visibility: self.visibility(node),
            docstring: None,
            declared_type: returns,
            params,
        });
        self.owned_by_the_enclosing_type(scope, &fqn, span(node));
        self.attributes(scope, node, &fqn);
        self.type_use(scope, node, "return_type");
        if let Some(body) = node.child_by_field_name("body") {
            self.children(&inner, body);
        }
    }

    fn property_declaration(&mut self, scope: &Scope, node: Node<'_>) {
        let ty = self.field_text(node, "type").map(str::to_string);
        let mut cursor = node.walk();
        let elements: Vec<Node<'_>> =
            node.children(&mut cursor).filter(|e| e.kind() == "property_element").collect();
        for element in elements {
            let Some(name) = element.child_by_field_name("name") else { continue };
            let name = self.variable(name);
            let Ok(fqn) = self.declare(scope, name, Reach::Field) else { continue };
            self.symbols.push(Symbol {
                fqn: fqn.clone(),
                kind: SymbolKind::Field,
                name: name.to_string(),
                span: span(element),
                visibility: self.visibility(node),
                docstring: None,
                declared_type: match &ty {
                    Some(t) => DeclaredType::Stated(t.clone()),
                    None => DeclaredType::Unstated,
                },
                params: Vec::new(),
            });
            self.owned_by_the_enclosing_type(scope, &fqn, span(element));
            self.attributes(scope, node, &fqn);
            // The DEFAULT VALUE only. Walking the whole element would descend
            // into the name too, and the attributes above have already walked
            // their arguments.
            if let Some(default) = element.child_by_field_name("default_value") {
                self.node(scope, default);
            }
        }
        self.type_use(scope, node, "type");
    }

    fn const_declaration(&mut self, scope: &Scope, node: Node<'_>) {
        // A CLASS constant is reached through `Foo::BAR`, which is a scoped
        // access — so it is minted at field reach, exactly as Java mints an
        // enum constant. A FILE-level `const` is reached by a bare name and is
        // an item. The container decides, so the two never cross.
        let reach = match (&scope.container, scope.in_body) {
            (Container::Type { .. }, false) => Reach::Field,
            _ => Reach::Item,
        };
        let mut cursor = node.walk();
        let elements: Vec<Node<'_>> =
            node.children(&mut cursor).filter(|e| e.kind() == "const_element").collect();
        for element in elements {
            let Some(name) = self.child_of_kind(element, "name").map(|n| self.text(n)) else {
                continue;
            };
            let Ok(fqn) = self.declare(scope, name, reach) else { continue };
            self.symbols.push(Symbol {
                fqn: fqn.clone(),
                kind: SymbolKind::Const,
                name: name.to_string(),
                span: span(element),
                visibility: self.visibility(node),
                docstring: None,
                declared_type: match self.field_text(node, "type") {
                    Some(t) => DeclaredType::Stated(t.to_string()),
                    None => DeclaredType::Unstated,
                },
                params: Vec::new(),
            });
            self.owned_by_the_enclosing_type(scope, &fqn, span(element));
            // A constant's VALUE holds calls and class-constant reads like any
            // other expression — `const MODE = Config::DEFAULT_MODE;`.
            self.children(scope, element);
        }
    }

    fn enum_case(&mut self, scope: &Scope, node: Node<'_>) {
        let Some(name) = self.field_text(node, "name") else { return };
        // Field reach, because `Suit::Hearts` is a scoped access and that is the
        // only shape that reaches one.
        let Ok(fqn) = self.declare(scope, name, Reach::Field) else { return };
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
        self.attributes(scope, node, &fqn);
    }

    /// `use SomeTrait;` in a class body — a MIXIN, which is what a PHP trait is.
    ///
    /// NOT `Extends` and not `Implements`: a trait contributes an
    /// implementation without entering the type hierarchy, so `instanceof`
    /// answers no for it. [`RelationKind::Mixin`] is the vocabulary for exactly
    /// that, and collapsing it into inheritance would make every trait look
    /// like a base class to pattern detection.
    fn trait_use(&mut self, scope: &Scope, node: Node<'_>) {
        let Container::Type { .. } = &scope.container else { return };
        let mut cursor = node.walk();
        let named: Vec<Node<'_>> = node
            .children(&mut cursor)
            .filter(|c| matches!(c.kind(), "name" | "qualified_name"))
            .collect();
        for mixed_in in named {
            let raw = self.text(mixed_in);
            self.relations.push(Relation {
                kind: RelationKind::Mixin,
                child: scope.from.clone(),
                parent: self.refer_to_type(raw, mixed_in),
                at: span(mixed_in),
            });
        }
    }

    /// `public`, `protected`, `private` — and nothing at all, which PHP defines
    /// as `public` rather than leaving open.
    fn visibility(&self, node: Node<'_>) -> Visibility {
        let modifier = match node.kind() {
            // A promoted parameter states it in a FIELD; everything else states
            // it as a plain child.
            "property_promotion_parameter" => self.field_text(node, "visibility"),
            _ => self.child_of_kind(node, "visibility_modifier").map(|c| self.text(c)),
        };
        match modifier.map(str::trim) {
            Some(m) if m.starts_with("private") => Visibility::Private,
            Some(m) if m.starts_with("protected") => {
                Visibility::Restricted("protected".to_string())
            }
            // PHP's DEFAULT is public, stated by the language. Not a fallback
            // standing in for a failed read.
            _ => Visibility::Public,
        }
    }

    /// `#[Route('/x')]`, `#[Attribute]` — each a `Decorates` relation, the same
    /// fact Java's annotations carry.
    fn attributes(&mut self, scope: &Scope, node: Node<'_>, child: &Fqn) {
        let Some(list) = node.child_by_field_name("attributes") else { return };
        let mut stack = vec![list];
        let mut found: Vec<Node<'_>> = Vec::new();
        while let Some(n) = stack.pop() {
            if n.kind() == "attribute" {
                found.push(n);
                continue;
            }
            let mut cursor = n.walk();
            for c in n.children(&mut cursor) {
                stack.push(c);
            }
        }
        for attribute in found {
            let Some(name) = self
                .child_of_kind(attribute, "name")
                .or_else(|| self.child_of_kind(attribute, "qualified_name"))
            else {
                continue;
            };
            let raw = self.text(name);
            self.relations.push(Relation {
                kind: RelationKind::Decorates,
                child: child.clone(),
                parent: self.refer_to_type(raw, name),
                at: span(attribute),
            });
            // An attribute's ARGUMENTS are expressions like any other:
            // `#[Assert\Choice(choices: Status::ALL)]` holds a class-constant
            // read, and recording only the relation would drop it.
            if let Some(args) = attribute.child_by_field_name("parameters") {
                self.children(scope, args);
            }
        }
    }

    // ── use sites ────────────────────────────────────────────────────────────

    /// Every `named_type` at or under a node — the types a signature NAMES.
    ///
    /// A `primitive_type` (`int`, `string`, `bool`, `void`) names no
    /// declaration and is skipped: emitting one would put a reference on a node
    /// that cannot exist.
    fn type_names_under<'t>(&self, node: Node<'t>) -> Vec<Node<'t>> {
        let mut out = Vec::new();
        let mut stack = vec![node];
        while let Some(n) = stack.pop() {
            if n.kind() == "named_type" {
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

    /// The class a `new` names, when it names one statically.
    ///
    /// `new $class` and `new class { ... }` name nothing this walk can read, so
    /// they answer `None` rather than a plausible class.
    fn constructed_type(&self, node: Node<'_>) -> Option<String> {
        let mut cursor = node.walk();
        let named =
            node.children(&mut cursor).find(|c| matches!(c.kind(), "name" | "qualified_name"))?;
        Some(self.text(named).to_string())
    }

    fn expression(&mut self, scope: &Scope, node: Node<'_>) {
        match node.kind() {
            "function_call_expression" => {
                let Some(callee) = node.child_by_field_name("function") else { return };
                let target = match callee.kind() {
                    // `strlen($x)`, `\App\Support\slugify($x)` — a free
                    // function, which is a name the ladder places.
                    "name" | "qualified_name" => self.named(self.text(callee), callee, Reach::Item),
                    // `$fn($x)` and `$this->handlers[$k]($x)` — the callee is a
                    // value, and nothing written says which function it holds.
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
            "member_call_expression" | "nullsafe_member_call_expression" => {
                self.member(scope, node, RefKind::Calls, Reach::Item);
            }
            "member_access_expression" | "nullsafe_member_access_expression" => {
                self.member(scope, node, RefKind::Reads, Reach::Field);
            }
            "scoped_call_expression" => self.scoped(scope, node, RefKind::Calls, Reach::Item),
            "scoped_property_access_expression" => {
                self.scoped(scope, node, RefKind::Reads, Reach::Field);
            }
            "class_constant_access_expression" => self.class_constant(scope, node),
            "object_creation_expression" => {
                // `new $class` and `new class { }` are handled where they are
                // declared, not here — there is no static name to construct.
                let mut cursor = node.walk();
                let Some(named) = node
                    .children(&mut cursor)
                    .find(|c| matches!(c.kind(), "name" | "qualified_name"))
                else {
                    return;
                };
                let raw = self.text(named);
                self.references.push(Reference {
                    from: scope.from.clone(),
                    kind: RefKind::Constructs,
                    at: span(node),
                    target: self.refer_to_type(raw, named),
                });
            }
            _ => {}
        }
    }

    /// `$x->name` and `$x->name()` — the receiver decides.
    fn member(&mut self, scope: &Scope, node: Node<'_>, kind: RefKind, reach: Reach) {
        let Some(name) = node.child_by_field_name("name") else { return };
        // `$obj->$method()` names the member with a VALUE. The call is real and
        // its target is not knowable from the source, so it is recorded as a
        // miss rather than dropped.
        if name.kind() != "name" {
            let target = self.missed(self.text(name), name, Reason::DynamicDispatch, reach);
            self.references.push(Reference {
                from: scope.from.clone(),
                kind,
                at: span(node),
                target,
            });
            return;
        }
        let member = self.text(name);
        let target = match node.child_by_field_name("object").and_then(|o| self.type_of(scope, o)) {
            Some(ty) => self.refer_to_member(&ty, member, node, reach),
            None => self.missed(member, node, Reason::ReceiverTypeUnknown, reach),
        };
        self.references.push(Reference { from: scope.from.clone(), kind, at: span(node), target });
    }

    /// `Foo::bar()` and `Foo::$bar` — a static reach through `::`.
    fn scoped(&mut self, scope: &Scope, node: Node<'_>, kind: RefKind, reach: Reach) {
        let Some(name) = node.child_by_field_name("name") else { return };
        let member = match name.kind() {
            "name" => self.text(name),
            // `Foo::$bar` puts the property name in a `variable_name`, sigil and
            // all, and the declaration side stripped it.
            "variable_name" => self.variable(name),
            _ => {
                let target = self.missed(self.text(name), name, Reason::DynamicDispatch, reach);
                self.references.push(Reference {
                    from: scope.from.clone(),
                    kind,
                    at: span(node),
                    target,
                });
                return;
            }
        };
        let target = match node.child_by_field_name("scope").and_then(|s| self.type_of(scope, s)) {
            Some(ty) => self.refer_to_member(&ty, member, node, reach),
            None => self.missed(member, node, Reason::ReceiverTypeUnknown, reach),
        };
        self.references.push(Reference { from: scope.from.clone(), kind, at: span(node), target });
    }

    /// `Foo::BAR`, `Suit::Hearts` and `Foo::class`.
    ///
    /// The grammar gives this node NO fields, so the two halves are read
    /// positionally: the first named child is the scope and the last is the
    /// constant.
    fn class_constant(&mut self, scope: &Scope, node: Node<'_>) {
        let mut cursor = node.walk();
        let parts: Vec<Node<'_>> = node.named_children(&mut cursor).collect();
        let (Some(owner), Some(name)) = (parts.first(), parts.last()) else { return };
        if owner.id() == name.id() {
            return;
        }
        let member = self.text(*name);
        // `Foo::class` is the magic constant that yields Foo's own name. It
        // reads no member — it NAMES THE TYPE, and that is the reference to
        // emit. Treating it as a member read would put a `class` constant on
        // every type in the corpus.
        if member == "class" {
            let raw = self.text(*owner);
            self.references.push(Reference {
                from: scope.from.clone(),
                kind: RefKind::TypeUse,
                at: span(node),
                target: self.refer_to_type(raw, *owner),
            });
            return;
        }
        let target = match self.type_of(scope, *owner) {
            Some(ty) => self.refer_to_member(&ty, member, node, Reach::Field),
            None => self.missed(member, node, Reason::ReceiverTypeUnknown, Reach::Field),
        };
        self.references.push(Reference {
            from: scope.from.clone(),
            kind: RefKind::Reads,
            at: span(node),
            target,
        });
    }

    /// The type of a receiver, when this scope STATES one. A lookup, never an
    /// inference.
    fn type_of(&self, scope: &Scope, node: Node<'_>) -> Option<String> {
        match node.kind() {
            // `$this`, `$service`, `$e`.
            "variable_name" => {
                let name = self.variable(node);
                if name == "this" {
                    return match &scope.container {
                        Container::Type { name } => Some(name.clone()),
                        Container::File => None,
                    };
                }
                scope.locals.get(name).cloned()
            }
            // `$this->repo->find()` — the receiver is itself a property read,
            // and the property's declared type answers for it. THE ONE
            // recursion here, and it is what makes constructor-injected
            // services reachable: it is how nearly every framework-shaped PHP
            // class calls its collaborators.
            "member_access_expression" | "nullsafe_member_access_expression" => {
                let object = node.child_by_field_name("object")?;
                let name = node.child_by_field_name("name")?;
                if name.kind() != "name" {
                    return None;
                }
                // Only through `$this`. A property of some OTHER object needs
                // that object's field table, which this scope does not carry —
                // and guessing would be a fabricated type.
                if object.kind() != "variable_name" || self.variable(object) != "this" {
                    return None;
                }
                scope.fields.get(self.text(name)).cloned()
            }
            // `Foo::bar()`, `\App\Foo::bar()` — the scope IS the type.
            "name" | "qualified_name" => Some(self.text(node).to_string()),
            // `self::`, `static::`, `parent::`. The first two are this type;
            // `parent` needs the base, which this pass has not resolved, so it
            // answers nothing rather than answering wrong.
            "relative_scope" => match self.text(node) {
                "self" | "static" => match &scope.container {
                    Container::Type { name } => Some(name.clone()),
                    Container::File => None,
                },
                _ => None,
            },
            _ => None,
        }
    }

    // ── minting a reference ──────────────────────────────────────────────────

    /// A name the walk read as a type. `Unplaced`, because the ladder knows the
    /// imports and this does not (R7).
    fn refer_to_type(&self, raw: &str, at: Node<'_>) -> Resolution {
        self.named(raw, at, Reach::Item)
    }

    /// A member of a type this scope named. Placed only when the scan declares
    /// that type; otherwise the whole path goes to the ladder.
    fn refer_to_member(&self, ty: &str, member: &str, at: Node<'_>, reach: Reach) -> Resolution {
        let Ok(ty) = super::type_segment(ty) else {
            return self.missed(member, at, Reason::UnhandledForm, reach);
        };
        // THE REACH IS THE USE SITE'S, not a constant — a property declared at
        // field reach and referred to at item reach are two strings that differ
        // in one segment and can never meet. Java paid 22,316 field
        // declarations and zero field edges for getting this wrong.
        if matches!(self.types.lookup(&self.package, &ty), Home::Tabled { .. })
            && let Ok(fqn) = fqn::refer(&Form::Member {
                lang: Language::Php,
                package: &self.package,
                module: "",
                ty: &ty,
                member,
                reach,
            })
        {
            return Resolution::Resolved { fqn, via: Rung::DeclaredHere };
        }
        // NOT OURS — hand the ladder the WHOLE PATH. `Log::info(...)` is a
        // facade call, and dropping the receiver leaves a bare `info` that no
        // import binds; with the path, `through_an_import` finds `Log` bound by
        // `use Illuminate\Support\Facades\Log` and names the member there.
        Resolution::Unresolved {
            reason: Reason::Unplaced,
            evidence: Evidence {
                name: member.to_string(),
                node_kind: at.kind().to_string(),
                reach,
                saw: vec![Observation::UnplacedType(format!("{ty}\\{member}"))],
            },
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
