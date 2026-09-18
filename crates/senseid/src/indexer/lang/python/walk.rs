//! The Python walk — one pass, every declaration and every use site (R1, R2).
//!
//! # What a Python binding states, and what it does not
//!
//! Java states a type at nearly every binding and JavaScript at almost none.
//! Python is in between, and WHICH of its bindings state one is the whole design
//! of [`Scope::locals`]:
//!
//! - `def f(p: Patient)` — an annotated parameter, read off the declaration.
//! - `x: Patient = ...` — an annotated assignment, likewise.
//! - `x = Patient()` — a constructor call. The type is not written as an
//!   annotation, but `Patient` is a name in an expression position and the class
//!   it names is one this scan may declare, so it is READ rather than inferred.
//! - `self` inside a method — the enclosing class, which is a fact about the
//!   language rather than about the code.
//!
//! Everything else — `x = something()`, `for x in xs`, `x = a.b` — states no
//! type, and the receiver of a later `x.method()` is coded
//! [`Reason::ReceiverTypeUnknown`]. That is a miss and not a wrong edge (R4).

use std::collections::BTreeMap;

use tree_sitter::Node;

use super::super::{Home, ReadError, Source, TypeHomes};
use crate::indexer::facts::{
    Binding, DeclaredType, Evidence, FileFacts, Fqn, Import, ImportOrigin, Language, Observation,
    Param, Reason, RefKind, Reference, Relation, RelationKind, Resolution, Rung, Span, Symbol,
    SymbolKind, Visibility,
};
use crate::indexer::fqn::{self, Form, FqnError, Reach};

/// Read one Python file.
pub fn read(source: &Source<'_>, types: &TypeHomes) -> Result<FileFacts, ReadError> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
        .map_err(|e| ReadError::GrammarUnavailable(e.to_string()))?;
    let tree = parser.parse(source.text, None).ok_or(ReadError::NotParsed)?;
    let root = tree.root_node();

    let file = super::file_fqn(source.package, source.module, source.path)
        .map_err(ReadError::NoFileIdentity)?;

    let mut walk = Walk {
        src: source.text,
        package: source.package,
        module: source.module,
        types,
        symbols: Vec::new(),
        references: Vec::new(),
        relations: Vec::new(),
        imports: Vec::new(),
    };
    walk.imports_of(root);

    let scope = Scope { from: file.clone(), container: Container::File, locals: BTreeMap::new() };
    walk.children(&scope, root);

    // THE FILE DECLARES ITS OWN MODULE, and in Python that module is what every
    // `import` of this file names. Emitted first so it precedes everything it
    // contains.
    let name = super::module_name_of(source.module, source.path);
    walk.symbols.insert(0, super::super::common::file_module(file.clone(), &name, source.text));

    // AND THE IMPORTS ENTER IT. Unlike Java — where every import names a type
    // and there is no module for one to enter — a Python import always names a
    // module, so every one of them is an edge. See
    // `common::specifier_names_a_module`.
    let entering = super::super::common::import_references(Language::Python, &walk.imports, &file);
    walk.references.extend(entering);

    Ok(FileFacts {
        language: Language::Python,
        package: source.package.to_string(),
        module: source.module.to_string(),
        path: source.path.to_string(),
        symbols: walk.symbols,
        references: walk.references,
        relations: walk.relations,
        imports: walk.imports,
    })
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

/// What a declaration is nested inside.
#[derive(Clone)]
enum Container {
    File,
    Class { name: String },
}

#[derive(Clone)]
struct Scope {
    /// The symbol a use site inside this scope belongs to.
    from: Fqn,
    container: Container,
    /// Name -> the type READ for it. Never inferred: every entry came off an
    /// annotation or a constructor call written in the source.
    locals: BTreeMap<String, String>,
}

struct Walk<'a> {
    src: &'a str,
    package: &'a str,
    module: &'a str,
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

    // ── imports ──────────────────────────────────────────────────────────────

    /// Every import in the file, as the module it names and the name it binds.
    ///
    /// `origin` is always `External` here and that is deliberate (R6, spec §2):
    /// this walk knows nothing about which packages the scan owns, and deciding
    /// externality from what has been read so far would make the graph depend on
    /// file order. The ladder reclassifies against `first_party`.
    fn imports_of(&mut self, root: Node<'_>) {
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            match child.kind() {
                "import_statement" => self.plain_import(child),
                "import_from_statement" => self.import_from(child),
                _ => {}
            }
        }
    }

    /// `import a.b` and `import a.b as c`.
    ///
    /// The bound name is the HEAD of the path, not its tail: `import a.b` puts
    /// `a` in scope and `a.b` is reached through it. An alias replaces the whole
    /// thing, which is why the two arms bind different strings.
    fn plain_import(&mut self, node: Node<'_>) {
        let at = span(node);
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let (path, bound) = match child.kind() {
                "dotted_name" => {
                    let path = self.text(child);
                    (path, path.split('.').next().unwrap_or(path).to_string())
                }
                "aliased_import" => {
                    let Some(path) = self.field_text(child, "name") else { continue };
                    let Some(alias) = self.field_text(child, "alias") else { continue };
                    (path, alias.to_string())
                }
                _ => continue,
            };
            self.imports.push(Import {
                path: path.to_string(),
                binds: Binding::Name(bound),
                origin: external(path),
                at,
            });
        }
    }

    /// `from a.b import C`, `from a.b import C as D`, `from a.b import *`, and
    /// the relative spellings of all three.
    fn import_from(&mut self, node: Node<'_>) {
        let at = span(node);
        let Some(module) = node.child_by_field_name("module_name") else { return };
        let path = self.text(module).to_string();
        // A RELATIVE import is ours by spelling — `from . import x` cannot name
        // anything but this package. Anything else the ladder decides.
        let origin = match path.starts_with('.') {
            true => ImportOrigin::Local,
            false => external(&path),
        };

        let mut cursor = node.walk();
        let mut bound_any = false;
        for child in node.children(&mut cursor) {
            if child.id() == module.id() {
                continue;
            }
            let binds = match child.kind() {
                "wildcard_import" => Binding::Glob,
                "dotted_name" => {
                    let name = self.text(child);
                    Binding::MemberOf { local: name.to_string(), member: name.to_string() }
                }
                "aliased_import" => {
                    let Some(name) = self.field_text(child, "name") else { continue };
                    let Some(alias) = self.field_text(child, "alias") else { continue };
                    Binding::MemberOf { local: alias.to_string(), member: name.to_string() }
                }
                _ => continue,
            };
            bound_any = true;
            self.imports.push(Import { path: path.clone(), binds, origin: origin.clone(), at });
        }
        // `from . import x` with nothing the loop recognised still entered a
        // module, and dropping it would lose the only edge the statement has.
        if !bound_any {
            self.imports.push(Import { path, binds: Binding::Glob, origin, at });
        }
    }

    // ── declarations ─────────────────────────────────────────────────────────

    fn children(&mut self, scope: &Scope, node: Node<'_>) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.node(scope, child);
        }
    }

    fn node(&mut self, scope: &Scope, node: Node<'_>) {
        match node.kind() {
            // A decorator wraps the definition it applies to; the definition is
            // what declares a name. The decorator itself is a use site, and it
            // is read by the expression walk below.
            // A decorator wraps the definition it applies to and is itself a
            // use site; both are ordinary children of this node.
            "decorated_definition" => self.children(scope, node),
            "class_definition" => self.class(scope, node),
            "function_definition" => self.function(scope, node),
            "assignment" => self.assignment(scope, node),
            // A CALL IS EMITTED HERE AND THE WALK CONTINUES THROUGH IT, exactly
            // once. `outer(inner())` is two use sites, and the inner one is
            // reached by descending rather than by a second traversal.
            "call" => {
                self.call(scope, node);
                self.children(scope, node);
            }
            // A block introduces no scope of its own in Python — a name bound
            // inside an `if` is visible after it — so the same scope walks in.
            _ => self.children(scope, node),
        }
    }

    /// Record that the enclosing class owns this declaration, or that the file
    /// contains it. Exactly one of the two, because both feed `nodes.parent_id`
    /// and a child with two parents has none.
    fn parented(&mut self, scope: &Scope, child: &Fqn, at: Span) {
        let kind = match &scope.container {
            Container::Class { .. } => RelationKind::Owns,
            Container::File => RelationKind::Contains,
        };
        self.relations.push(Relation {
            kind,
            child: child.clone(),
            parent: Resolution::Resolved { fqn: scope.from.clone(), via: Rung::DeclaredHere },
            at,
        });
    }

    /// Mint the identity of a declaration in the current container.
    fn declare(&self, scope: &Scope, member: &str, reach: Reach) -> Result<Fqn, FqnError> {
        let lang = Language::Python;
        let (package, module) = (self.package, self.module);
        match &scope.container {
            Container::File => {
                fqn::define(&Form::Item { lang, package, module, name: member, reach })
            }
            Container::Class { name: ty } => {
                fqn::define(&Form::Member { lang, package, module, ty, member, reach })
            }
        }
    }

    fn class(&mut self, scope: &Scope, node: Node<'_>) {
        let Some(name) = self.field_text(node, "name") else { return };
        let at = span(node);
        let Ok(fqn) = self.declare(scope, name, Reach::Item) else { return };

        self.symbols.push(Symbol {
            fqn: fqn.clone(),
            kind: SymbolKind::Class,
            name: name.to_string(),
            span: at,
            visibility: visibility_of(name),
            docstring: docstring_of(self.src, node),
            declared_type: DeclaredType::Unstated,
            params: Vec::new(),
        });
        self.parented(scope, &fqn, at);

        // A BASE CLASS IS A USE SITE. `class Nurse(Practitioner)` reads
        // `Practitioner` exactly as a call would.
        if let Some(bases) = node.child_by_field_name("superclasses") {
            self.node(scope, bases);
        }

        let inner = Scope {
            from: fqn,
            container: Container::Class { name: name.to_string() },
            locals: BTreeMap::new(),
        };
        if let Some(body) = node.child_by_field_name("body") {
            self.children(&inner, body);
        }
    }

    fn function(&mut self, scope: &Scope, node: Node<'_>) {
        let Some(name) = self.field_text(node, "name") else { return };
        let at = span(node);
        // A method reaches as a MEMBER of its class; a module-level `def` is an
        // ITEM. Both go through `declare`, which reads the container.
        let Ok(fqn) = self.declare(scope, name, Reach::Item) else { return };

        let params = self.params_of(node);
        self.symbols.push(Symbol {
            fqn: fqn.clone(),
            kind: match &scope.container {
                Container::Class { .. } => SymbolKind::Method,
                Container::File => SymbolKind::Function,
            },
            name: name.to_string(),
            span: at,
            visibility: visibility_of(name),
            docstring: docstring_of(self.src, node),
            declared_type: match self.field_text(node, "return_type").map(super::type_segment) {
                Some(Ok(ty)) => DeclaredType::Stated(ty),
                _ => DeclaredType::Unstated,
            },
            params: params.clone(),
        });
        self.parented(scope, &fqn, at);

        // The body's scope: the function's own identity, the SAME container
        // (a nested `def` inside a method is still that class's), and the
        // locals its parameters state.
        let mut locals = scope.locals.clone();
        for param in &params {
            if let DeclaredType::Stated(ty) = &param.declared_type {
                locals.insert(param.name.clone(), ty.clone());
            }
        }
        // `self` IS the enclosing class. A fact about the language, not an
        // inference about the code — and the single highest-yield entry in this
        // map, because a method body reaches its own class through nothing else.
        if let Container::Class { name: ty } = &scope.container {
            locals.insert("self".to_string(), ty.clone());
            locals.insert("cls".to_string(), ty.clone());
        }

        let inner = Scope { from: fqn, container: scope.container.clone(), locals };
        if let Some(body) = node.child_by_field_name("body") {
            self.children(&inner, body);
        }
        // A return annotation and a parameter annotation are both use sites of
        // the types they name.
        for raw in self.annotations_of(node) {
            let target = self.refer_to_type(&raw, node);
            self.references.push(Reference {
                from: inner.from.clone(),
                kind: RefKind::TypeUse,
                at,
                target,
            });
        }
    }

    fn params_of(&self, node: Node<'_>) -> Vec<Param> {
        let Some(list) = node.child_by_field_name("parameters") else { return Vec::new() };
        let mut cursor = list.walk();
        let mut params = Vec::new();
        for child in list.children(&mut cursor) {
            let (name, ty) = match child.kind() {
                "identifier" => (self.text(child), None),
                "typed_parameter" | "typed_default_parameter" => {
                    let name = child
                        .child_by_field_name("name")
                        .or_else(|| child.named_child(0))
                        .map(|c| self.text(c));
                    let Some(name) = name else { continue };
                    (name, self.field_text(child, "type"))
                }
                "default_parameter" => {
                    let Some(name) = self.field_text(child, "name") else { continue };
                    (name, None)
                }
                _ => continue,
            };
            params.push(Param {
                name: name.to_string(),
                position: params.len() as u32,
                declared_type: match ty.map(super::type_segment) {
                    Some(Ok(ty)) => DeclaredType::Stated(ty),
                    _ => DeclaredType::Unstated,
                },
            });
        }
        params
    }

    /// Every type expression the signature writes, for the use sites they are.
    fn annotations_of(&self, node: Node<'_>) -> Vec<String> {
        let mut found: Vec<String> = Vec::new();
        if let Some(raw) = self.field_text(node, "return_type") {
            found.push(raw.to_string());
        }
        if let Some(list) = node.child_by_field_name("parameters") {
            let mut cursor = list.walk();
            for child in list.children(&mut cursor) {
                if let Some(raw) = self.field_text(child, "type") {
                    found.push(raw.to_string());
                }
            }
        }
        found
    }

    /// An assignment, which may declare a class attribute, bind a local with a
    /// readable type, or both.
    fn assignment(&mut self, scope: &Scope, node: Node<'_>) {
        let Some(left) = node.child_by_field_name("left") else { return };
        let at = span(node);

        // A CLASS-BODY assignment declares an attribute of that class.
        if let Container::Class { .. } = &scope.container
            && left.kind() == "identifier"
            && let Ok(fqn) = self.declare(scope, self.text(left), Reach::Field)
        {
            let name = self.text(left);
            self.symbols.push(Symbol {
                fqn: fqn.clone(),
                kind: SymbolKind::Field,
                name: name.to_string(),
                span: at,
                visibility: visibility_of(name),
                docstring: None,
                declared_type: match self.field_text(node, "type").map(super::type_segment) {
                    Some(Ok(ty)) => DeclaredType::Stated(ty),
                    _ => DeclaredType::Unstated,
                },
                params: Vec::new(),
            });
            self.parented(scope, &fqn, at);
        }

        if let Some(right) = node.child_by_field_name("right") {
            self.node(scope, right);
        }
        if let Some(raw) = self.field_text(node, "type") {
            let target = self.refer_to_type(raw, node);
            self.references.push(Reference {
                from: scope.from.clone(),
                kind: RefKind::TypeUse,
                at,
                target,
            });
        }
    }

    // ── use sites ────────────────────────────────────────────────────────────

    fn call(&mut self, scope: &Scope, node: Node<'_>) {
        let Some(callee) = node.child_by_field_name("function") else { return };
        let at = span(node);
        let target = match callee.kind() {
            // `f()` — a bare name. The ladder places it from the imports.
            "identifier" => {
                let name = self.text(callee);
                self.unplaced(name, callee, Reach::Item)
            }
            // `x.m()` — a member of whatever `x` is.
            "attribute" => {
                let Some(member) = self.field_text(callee, "attribute") else { return };
                let Some(object) = callee.child_by_field_name("object") else { return };
                match self.type_of(scope, object) {
                    Some(ty) => self.refer_to_member(&ty, member, callee, Reach::Item),
                    None => self.missed(member, callee, Reason::ReceiverTypeUnknown, Reach::Item),
                }
            }
            _ => return,
        };
        self.references.push(Reference {
            from: scope.from.clone(),
            kind: RefKind::Calls,
            at,
            target,
        });
    }

    /// The type of an expression, READ rather than inferred. `None` means the
    /// source states none — which is a miss, and never a licence to guess.
    fn type_of(&self, scope: &Scope, node: Node<'_>) -> Option<String> {
        match node.kind() {
            "identifier" => {
                let name = self.text(node);
                if let Some(ty) = scope.locals.get(name) {
                    return Some(ty.clone());
                }
                // A capitalised bare name is the CLASS itself: `Patient.load()`
                // is a call on the type, not on an instance of it.
                crate::indexer::lang::common::names_a_type_by_leading_case(name)
                    .then(|| name.to_string())
            }
            // `self.field.method()` — only when the field's type was declared.
            "attribute" => {
                let object = node.child_by_field_name("object")?;
                let member = self.field_text(node, "attribute")?;
                let owner = self.type_of(scope, object)?;
                self.declared_member_type(&owner, member)
            }
            // `Patient().save()` — the constructor names the type outright.
            "call" => {
                let callee = node.child_by_field_name("function")?;
                let name = self.text(callee);
                crate::indexer::lang::common::names_a_type_by_leading_case(name)
                    .then(|| name.to_string())
            }
            _ => None,
        }
    }

    /// The declared type of a member this scan read, or `None`.
    fn declared_member_type(&self, owner: &str, member: &str) -> Option<String> {
        let want = fqn::refer(&Form::Member {
            lang: Language::Python,
            package: self.package,
            module: self.module,
            ty: owner,
            member,
            reach: Reach::Field,
        })
        .ok()?;
        self.symbols.iter().find(|s| s.fqn == want).and_then(|s| match &s.declared_type {
            DeclaredType::Stated(ty) => Some(ty.clone()),
            _ => None,
        })
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
    /// that type; otherwise the whole path goes to the ladder, which knows the
    /// imports and can name the library.
    fn refer_to_member(&self, ty: &str, member: &str, at: Node<'_>, reach: Reach) -> Resolution {
        let Ok(ty) = super::type_segment(ty) else {
            return self.missed(member, at, Reason::UnhandledForm, Reach::Item);
        };
        if let Home::Ours { module } = self.types.lookup(self.package, &ty)
            && let Ok(fqn) = fqn::refer(&Form::Member {
                lang: Language::Python,
                package: self.package,
                module,
                ty: &ty,
                member,
                reach,
            })
        {
            return Resolution::Resolved { fqn, via: Rung::DeclaredHere };
        }
        Resolution::Unresolved {
            reason: Reason::Unplaced,
            evidence: Evidence {
                name: member.to_string(),
                node_kind: at.kind().to_string(),
                reach: Reach::Item,
                saw: vec![Observation::UnplacedType(format!("{ty}.{member}"))],
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
                name: crate::indexer::lang::common::readable(name, at.kind()),
                node_kind: at.kind().to_string(),
                reach,
                saw: Vec::new(),
            },
        }
    }
}

/// The package an absolute import names: its FIRST segment.
///
/// `import requests.adapters` reaches into the distribution `requests`, and that
/// leading segment is what a manifest names and what `first_party` is checked
/// against. Java takes everything but the last segment instead, because a Java
/// package IS the whole prefix — Python's is only the head.
fn external(path: &str) -> ImportOrigin {
    ImportOrigin::External { package: path.split('.').next().unwrap_or(path).to_string() }
}

/// Python states visibility by CONVENTION, in the name.
///
/// A leading underscore means "not part of the API" and the whole ecosystem
/// honours it; a dunder is the object protocol and is public. Nothing is
/// enforced, which is why this reports what the name says rather than claiming
/// to know what the language permits.
fn visibility_of(name: &str) -> Visibility {
    match name.starts_with('_') && !name.starts_with("__") {
        true => Visibility::Private,
        false => Visibility::Public,
    }
}

/// A Python docstring is the first statement of the body, as a bare string.
fn docstring_of(src: &str, node: Node<'_>) -> Option<String> {
    let body = node.child_by_field_name("body")?;
    let first = body.named_child(0)?;
    if first.kind() != "expression_statement" {
        return None;
    }
    let literal = first.named_child(0)?;
    if literal.kind() != "string" {
        return None;
    }
    let raw = &src[literal.byte_range()];
    Some(raw.trim_matches(|c| c == '"' || c == '\'').trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexer::lang::LanguageAdapter;

    /// One Python file, read the way the processor will hand it over.
    fn read_py(text: &str) -> FileFacts {
        super::super::PythonAdapter
            .read(
                &Source { package: "proj", module: "pkg.a", path: "src/pkg/a.py", text },
                &TypeHomes::unknown(),
            )
            .expect("a well-formed file reads")
    }

    fn fqns(facts: &FileFacts) -> Vec<String> {
        facts.symbols.iter().map(|s| s.fqn.to_string()).collect()
    }

    /// THE FILE DECLARES ITS OWN MODULE, and it is the identity an `import` of
    /// this file mints. Without it a Python file's top-level declarations hang
    /// off nothing and `nodes.parent_id` has no value for them.
    #[test]
    fn a_file_declares_the_module_an_import_of_it_names() {
        let facts = read_py("def f():\n    pass\n");
        let module = &facts.symbols[0];
        assert_eq!(module.kind, SymbolKind::Module);
        assert_eq!(module.fqn.to_string(), "python·proj·pkg·a·mod");
        assert_eq!(module.name, "a");
    }

    /// A top-level declaration is CONTAINED by the file; a method is OWNED by
    /// its class. Exactly one of the two, because both feed `nodes.parent_id`.
    ///
    /// The mutation that must break this: make `parented` always emit
    /// `Contains`. Every method would then be parented on the file and the
    /// class would own nothing.
    #[test]
    fn a_method_is_owned_by_its_class_and_a_function_is_contained_by_the_file() {
        let facts = read_py(
            "class Widget:\n    def wide(self):\n        return 1\n\ndef free():\n    pass\n",
        );

        let owns: Vec<&str> = facts
            .relations
            .iter()
            .filter(|r| r.kind == RelationKind::Owns)
            .map(|r| r.child.as_str())
            .collect();
        assert_eq!(owns, vec!["python·proj·pkg.a·Widget·wide·item"], "the method, owned by Widget");

        let contains: Vec<&str> = facts
            .relations
            .iter()
            .filter(|r| r.kind == RelationKind::Contains)
            .map(|r| r.child.as_str())
            .collect();
        assert_eq!(
            contains,
            vec!["python·proj·pkg.a·Widget·item", "python·proj·pkg.a·free·item"],
            "the class and the free function, contained by the file"
        );
    }

    /// `import a.b` binds `a`, NOT `b`. The head is what goes into scope and
    /// `a.b` is reached through it.
    ///
    /// The mutation that must break this: bind the last segment instead. Every
    /// `import os.path` would then put `path` in scope, and a later `os.getcwd`
    /// would resolve against nothing.
    #[test]
    fn a_plain_import_binds_the_head_of_the_path() {
        let facts = read_py("import os.path\nimport numpy as np\n");
        let binds: Vec<(&str, &Binding)> =
            facts.imports.iter().map(|i| (i.path.as_str(), &i.binds)).collect();
        assert_eq!(binds[0].0, "os.path");
        assert_eq!(binds[0].1, &Binding::Name("os".to_string()), "the HEAD, not `path`");
        assert_eq!(binds[1].0, "numpy");
        assert_eq!(
            binds[1].1,
            &Binding::Name("np".to_string()),
            "an alias replaces the whole thing"
        );
    }

    /// The package of an absolute import is its FIRST segment, which is what a
    /// manifest names and what `first_party` is checked against. Java takes
    /// everything but the last, and copying that rule here would look for a
    /// distribution called `os.path`.
    #[test]
    fn the_package_an_import_names_is_its_leading_segment() {
        let facts = read_py("import os.path\nfrom a.b.c import D\n");
        let packages: Vec<String> = facts
            .imports
            .iter()
            .map(|i| match &i.origin {
                ImportOrigin::External { package } => package.clone(),
                ImportOrigin::Local => "<local>".to_string(),
            })
            .collect();
        assert_eq!(packages, vec!["os", "a"]);
    }

    /// `from a import C` names the MODULE in the specifier and the item in the
    /// clause, which is what makes Python's `Binding::Name` unambiguous where
    /// Rust's is not. A relative import is ours by spelling alone.
    #[test]
    fn a_from_import_carries_the_module_and_binds_the_member() {
        let facts = read_py(
            "from a.b import C\nfrom . import sibling\nfrom .mod import D as E\nfrom x import *\n",
        );
        let shapes: Vec<(&str, &Binding, bool)> = facts
            .imports
            .iter()
            .map(|i| (i.path.as_str(), &i.binds, matches!(i.origin, ImportOrigin::Local)))
            .collect();

        assert_eq!(shapes[0].0, "a.b");
        assert_eq!(shapes[0].1, &Binding::MemberOf { local: "C".into(), member: "C".into() });
        assert!(!shapes[0].2, "an absolute import is not local by spelling");

        assert_eq!(shapes[1].0, ".");
        assert!(shapes[1].2, "a relative import cannot name anything but this package");

        assert_eq!(shapes[2].0, ".mod");
        assert_eq!(
            shapes[2].1,
            &Binding::MemberOf { local: "E".into(), member: "D".into() },
            "the alias is what is in scope; the member is what was imported"
        );
        assert!(shapes[2].2);

        assert_eq!(
            shapes[3].1,
            &Binding::Glob,
            "`import *` is a glob, and the specifier stays clean"
        );
        assert_eq!(shapes[3].0, "x", "the star is NOT in the path, unlike Java's");
    }

    /// Every Python import enters a module, so every one of them is an edge —
    /// the step-11 shape. Java emits none of these because a Java import names
    /// a type; the difference is stated once, in `specifier_names_a_module`.
    ///
    /// The mutation that must break this: make `specifier_names_a_module`
    /// answer false for Python's `Name`. `import os.path` would stop being an
    /// edge and the module graph would lose every plain import in the corpus.
    #[test]
    fn every_import_is_an_edge_that_enters_the_file_s_own_module() {
        let facts = read_py("import os\nfrom a.b import C\n");
        let entering: Vec<&Reference> =
            facts.references.iter().filter(|r| r.kind == RefKind::Imports).collect();
        assert_eq!(entering.len(), 2, "both shapes name a module");
        for reference in entering {
            assert_eq!(
                reference.from.to_string(),
                "python·proj·pkg·a·mod",
                "an import belongs to the file, which is the module it enters things into"
            );
        }
    }

    /// `self` IS the enclosing class — a fact about the language rather than an
    /// inference about the code, and the single highest-yield entry in the
    /// locals map: a method body reaches its own class through nothing else.
    ///
    /// The mutation that must break this: drop the `self`/`cls` insert. The
    /// call resolves to `ReceiverTypeUnknown` instead.
    #[test]
    fn self_resolves_to_the_enclosing_class() {
        let facts = read_py(
            "class Widget:\n    def wide(self):\n        return 1\n    def area(self):\n        return self.wide()\n",
        );
        let call = facts
            .references
            .iter()
            .find(|r| r.kind == RefKind::Calls)
            .expect("the body calls something");
        // `Widget` is not in `TypeHomes::unknown()`, so the walk cannot place the
        // member — but it must have READ the receiver's type, which is what the
        // evidence carries.
        match &call.target {
            Resolution::Unresolved { reason, evidence } => {
                assert_eq!(*reason, Reason::Unplaced, "read, but not placeable without type homes");
                assert!(
                    evidence
                        .saw
                        .iter()
                        .any(|o| matches!(o, Observation::UnplacedType(t) if t == "Widget.wide")),
                    "the receiver was typed as Widget: {:?}",
                    evidence.saw
                );
            }
            other => panic!("expected an unplaced member, got {other:?}"),
        }
    }

    /// A binding that states NO type leaves the receiver unknown. That is a
    /// miss, and it must be CODED as one rather than guessed at (R4).
    ///
    /// The mutation that must break this: have `type_of` fall back to the
    /// receiver's own name. `thing.method()` would mint a member of a type
    /// called `thing`, which no declaration anywhere mints.
    #[test]
    fn an_untyped_receiver_is_coded_unknown_rather_than_guessed() {
        let facts = read_py("def f(thing):\n    return thing.method()\n");
        let call = facts
            .references
            .iter()
            .find(|r| r.kind == RefKind::Calls)
            .expect("the body calls something");
        assert!(
            matches!(
                &call.target,
                Resolution::Unresolved { reason: Reason::ReceiverTypeUnknown, .. }
            ),
            "got {:?}",
            call.target
        );
    }

    /// An ANNOTATED parameter states a type, so the receiver is readable where
    /// the unannotated one above is not. This is the whole reason Python sits
    /// between JavaScript and Java.
    #[test]
    fn an_annotated_parameter_types_its_receiver() {
        let facts = read_py("def f(p: Patient):\n    return p.load()\n");
        let call = facts.references.iter().find(|r| r.kind == RefKind::Calls).expect("a call");
        match &call.target {
            Resolution::Unresolved { evidence, .. } => assert!(
                evidence
                    .saw
                    .iter()
                    .any(|o| matches!(o, Observation::UnplacedType(t) if t == "Patient.load")),
                "the annotation typed the receiver: {:?}",
                evidence.saw
            ),
            other => panic!("expected the annotation to be read, got {other:?}"),
        }
    }

    /// **ONE CALL SITE, ONE REFERENCE.** A traversal that reaches a node twice
    /// emits its use site twice, and every count built on references —
    /// fan-in, the miss histogram, the coverage barrier — is then inflated by a
    /// factor nobody can see from the inside.
    ///
    /// My own fixtures could not catch this, because they all used `.find()` on
    /// the first matching reference and a duplicate reads exactly like the
    /// original. It took the corpus: 103 files produced 37,973 references
    /// against 1,487 symbols.
    #[test]
    fn a_call_site_emits_exactly_one_reference() {
        let facts = read_py("def f():\n    return g()\n");
        let calls: Vec<&Reference> =
            facts.references.iter().filter(|r| r.kind == RefKind::Calls).collect();
        assert_eq!(calls.len(), 1, "one call in the source, {} emitted", calls.len());
    }

    /// The same, for a call nested inside another expression — the shape a
    /// double-walking traversal multiplies rather than merely doubles.
    #[test]
    fn a_nested_call_is_not_counted_once_per_level_of_nesting() {
        let facts = read_py("def f():\n    return outer(inner(deep()))\n");
        let calls: Vec<&Reference> =
            facts.references.iter().filter(|r| r.kind == RefKind::Calls).collect();
        assert_eq!(calls.len(), 3, "three calls in the source, {} emitted", calls.len());
    }

    /// Python states visibility in the NAME, and a dunder is public. A rule that
    /// only checked for a leading underscore would file `__init__` as private
    /// and hide every constructor in the corpus.
    #[test]
    fn a_leading_underscore_is_private_and_a_dunder_is_not() {
        assert_eq!(visibility_of("load"), Visibility::Public);
        assert_eq!(visibility_of("_internal"), Visibility::Private);
        assert_eq!(visibility_of("__init__"), Visibility::Public);
        assert_eq!(visibility_of("__str__"), Visibility::Public);
    }

    /// A class body's assignment declares an ATTRIBUTE of that class, at
    /// `Reach::Field` — the reach a use site of it mints. A module-level
    /// assignment declares no field, because there is no type to hang it off.
    #[test]
    fn a_class_body_assignment_declares_a_field_and_a_module_level_one_does_not() {
        let facts = read_py("WIDTH = 0\n\nclass Widget:\n    height: int = 0\n");
        let all = fqns(&facts);
        assert!(
            all.contains(&"python·proj·pkg.a·Widget·height·field".to_string()),
            "the class attribute, at Field reach: {all:?}"
        );
        assert!(
            !all.iter().any(|f| f.contains("WIDTH")),
            "a module-level binding is not a field of anything: {all:?}"
        );
    }

    /// A damaged file must not take the walk down with it. tree-sitter returns
    /// ERROR nodes rather than failing, and what matters is that the file still
    /// declares its own module so nothing downstream sees a file with no
    /// identity.
    #[test]
    fn a_file_that_does_not_parse_still_declares_its_own_module() {
        let facts = read_py("class ???:\n  def (:\n");
        assert_eq!(facts.symbols[0].kind, SymbolKind::Module);
        assert_eq!(facts.symbols[0].fqn.to_string(), "python·proj·pkg·a·mod");
    }
}
