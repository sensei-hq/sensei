//! The tree-sitter walk itself: where it is, what it has in scope, and the one
//! dispatch that turns a node into a fact.
//!
//! Split out of the module that OWNS the Rust language so the two questions
//! stay apart. `mod.rs` answers "how is a Rust symbol named, and what does this
//! language look like to the ladder"; this file answers "what did the parser
//! just hand me". The identity rules are read here and decided there.

use std::collections::{BTreeMap, BTreeSet};

use tree_sitter::Node;

use super::super::common::{Miss, considered, named};
use super::super::{Home, Source, TypeHomes};
use super::MODULE;
use super::types::{element_type, extracted_type, simple_type_name};
use super::types::{type_path, type_segment};
use crate::indexer::facts::{
    Binding, DeclaredType, Fqn, Import, ImportOrigin, Language, Observation, Param, Reason,
    RefKind, Reference, Relation, RelationKind, Resolution, Rung, Symbol, SymbolKind, Visibility,
};
use crate::indexer::fqn::{self, Form, FqnError, Reach};

/// [`Miss::unplaced`] for a tree-sitter walk: the shared constructor takes the
/// node KIND, because the other language's parser has no `Node` to give it.
fn unplaced(node: Node<'_>, name: &str, reach: Reach, saw: Vec<Observation>) -> Miss {
    Miss::unplaced(node.kind(), name, reach, saw)
}

/// [`Miss::unhandled`], the same way.
fn unhandled(node: Node<'_>, name: &str, reach: Reach) -> Miss {
    Miss::unhandled(node.kind(), name, reach)
}

/// Everything one walk of one tree produced. Not a `FileFacts`: that carries
/// the file's package, module and path, which are identity and are stated by
/// the module above rather than read out of the source.
pub(super) struct Found {
    pub symbols: Vec<Symbol>,
    pub references: Vec<Reference>,
    pub relations: Vec<Relation>,
    pub imports: Vec<Import>,
}

/// Walk one parsed tree, with the file's identity already minted.
pub(super) fn walk<'a>(
    source: &Source<'a>,
    types: &'a TypeHomes,
    root: Node<'_>,
    from: Fqn,
) -> Found {
    let scope = Scope {
        holder: from.clone(),
        module: source.module.to_string(),
        fn_scope: Vec::new(),
        container: Container::File,
        from,
        owner: Owner::Nobody,
        // The file scope binds nothing: a `let` lives in a block, and the file
        // level has none.
        bindings: BTreeMap::new(),
        bound_to_a_call: BTreeMap::new(),
        returns: BTreeMap::new(),
        fields: BTreeMap::new(),
    };
    let mut walk = Walk {
        src: source.text,
        package: source.package,
        types,
        declared_fields: BTreeMap::new(),
        inherent_members: BTreeMap::new(),
        declared_here: BTreeMap::new(),
        symbols: Vec::new(),
        references: Vec::new(),
        relations: Vec::new(),
        imports: Vec::new(),
    };
    // Struct fields FIRST, over the whole tree: an `impl` block may appear
    // before the struct it is about, and a single-pass walk would reach
    // `self.field` with nothing recorded for it.
    // Seeded with the FILE's module, so a type declared at file scope is homed
    // there and one inside `mod tests` is homed a segment deeper.
    walk.collect_declared_fields(root, source.module);
    walk.collect_inherent_members(root);
    walk.children(root, &scope);
    Found {
        symbols: walk.symbols,
        references: walk.references,
        relations: walk.relations,
        imports: walk.imports,
    }
}

/// Where the walk currently is, in the terms the fqn grammar needs.
#[derive(Debug, Clone)]
struct Scope {
    /// Module path of the current container, which an inline `mod` extends.
    module: String,
    /// The chain of FUNCTIONS enclosing this point, outermost first.
    ///
    /// The AST always had this — the walk is literally inside
    /// [`Walk::function`] descending into that function's own children — and
    /// [`Walk::push`] was already recording the enclosing symbol in
    /// [`Scope::from`], which is why a REFERENCE in a body attributes to its
    /// function correctly. A DECLARATION did not, because [`Walk::declare`]
    /// reads [`Scope::container`] and nothing told the container it had gone
    /// inside a body. So `static RE` in three different `fn`s of one file minted
    /// ONE identity, and "where is `RE` defined" answered with whichever the
    /// scan wrote last.
    fn_scope: Vec<String>,
    container: Container,
    /// The symbol a use site found here sits inside — [`Reference::from`].
    from: Fqn,
    /// The nearest enclosing MODULE, as an identity.
    ///
    /// NOT [`Scope::from`], which is the nearest enclosing SYMBOL and changes at
    /// every declaration — a function's body sees `from` as the function. This
    /// changes only at a module boundary, which is what containment is measured
    /// against: it starts as the file's own identity and an inline `mod`
    /// replaces it.
    holder: Fqn,
    /// The type a declaration found here is a member of, as an IDENTITY.
    owner: Owner,
    /// Field name -> the TYPE it is declared with, for the CURRENT type.
    ///
    /// `for c in &self.checkers` needs the field's type to know the element
    /// type. Collected in the same pre-pass spirit as [`Scope::returns`] — a
    /// field can be used before it is declared in source order.
    fields: BTreeMap<String, String>,
    /// Method name -> the TYPE it returns, for the methods of the CURRENT impl
    /// block.
    ///
    /// Only `self.m().member` needs it: the inner call already resolves (the
    /// container types `self`), so the one missing fact is `m`'s return type —
    /// and `m` is a method of this same type, declared right here.
    ///
    /// Collected in a PRE-PASS over the impl block, because a method may be
    /// declared after the call that uses it and a single-pass walk would not
    /// have seen it yet. Scoped to the block, so it cannot type a call on some
    /// other type's identically-named method.
    returns: BTreeMap<String, String>,
    /// Local name -> the TYPE it is bound to, for the names in scope here.
    ///
    /// A member's identity ends in the type and then the member, so the
    /// receiver's type is a segment of the key a use site has to mint. Without
    /// this the walk knew a
    /// receiver's type in exactly one case — `self` inside a type's own body —
    /// and every other member call was `ReceiverTypeUnknown` even where the
    /// source states the type one line above.
    ///
    /// Only what the source STATES is recorded. A binding whose type is not
    /// written stays absent, and its uses stay unresolved: a guessed receiver
    /// type mints a wrong identity, which R4 ranks below no identity at all.
    ///
    /// Carried on the scope rather than on the walk so it obeys block
    /// structure for free — a scope is already cloned per body, so a binding
    /// cannot outlive the block that introduced it or be seen before its own
    /// `let`.
    bindings: BTreeMap<String, String>,
    /// Local name -> the CALLEE whose result bound it, for the locals the source
    /// never typed.
    ///
    /// The sibling of [`Scope::bindings`] and a weaker fact on purpose. That one
    /// holds a TYPE, which the source wrote down. This one holds a NAME, because
    /// `let s = pg_store()` states no type anywhere in this file — what `pg_store`
    /// returns is written on a declaration that may be in another file, so only
    /// a completed pass can answer it and the walk must not try (R6).
    ///
    /// So the walk records the question and the ladder answers it. The chained
    /// form `pg_store().script()` already resolved through
    /// `Ladder::through_what_the_receiver_returns`; this is the same edge split
    /// over two lines, and the only thing that was missing is the link from the
    /// receiver's NAME back to the call.
    ///
    /// Recorded only where [`Scope::bindings`] has nothing, so a type the source
    /// STATED is never overruled by one a lookup would infer.
    bound_to_a_call: BTreeMap<String, String>,
}

impl Scope {
    /// The module segment a declaration found HERE is named in.
    ///
    /// The file's module, extended by every enclosing function. `fn` is the
    /// joint because Rust has no module that can be spelled `fn` — it is a
    /// keyword — so a composed path can never collide with a real one.
    ///
    /// A local is not reachable by path from outside, so no OTHER file can mint
    /// this identity. That is not a reason to leave it at module scope: the
    /// declaration IS referenced, inside its own body, and both sides compose
    /// the same way from the same scope — so they merge, and three `RE`s in
    /// three functions stay three symbols instead of one.
    fn module_here(&self) -> String {
        if self.fn_scope.is_empty() {
            return self.module.clone();
        }
        let inner = self.fn_scope.join("::");
        if self.module.is_empty() {
            format!("fn::{inner}")
        } else {
            format!("{}::fn::{inner}", self.module)
        }
    }
}

/// The parent side of every ownership relation (spec §3.3, R8's Facade row).
///
/// Carried next to [`Container`] rather than derived from it. A container names
/// its type with a string, and an enum variant's container is the string
/// `Enum::Variant`, which is not the spelling the variant's own identity uses —
/// re-minting from it would produce an owner no declaration ever minted, which
/// is a dangling edge (A4). The identity the walk already pushed cannot drift
/// from the one its members were named under.
#[derive(Debug, Clone)]
enum Owner {
    /// A free item, at the file level or inside an inline `mod`. Owned by no
    /// type — naming the module as an owner would be an edge the source does
    /// not state. Also what an unnameable container leaves behind: a type with
    /// no identity owns nothing, because there is nothing to point at (R4).
    Nobody,
    Type(Fqn),
}

/// What a declaration found here is a member OF. This is the only thing that
/// decides which fqn form a declaration takes, so the choice is made once.
#[derive(Debug, Clone)]
enum Container {
    /// A free item, at the file level or inside an inline `mod`.
    File,
    /// A struct/enum/union body, a trait body, or an inherent `impl`.
    ///
    /// Carries the MODULE as well as the name, and that is the whole of the
    /// anchoring fix: a member's identity carries the module, then the type,
    /// then the member — and the MODULE is where the TYPE is declared. An
    /// `impl PgStore` in
    /// `db::pg_store::personas` declares members of the `PgStore` that lives in
    /// `db::pg_store`, so
    /// the block's own module is the wrong answer and was the one being used.
    /// A struct/enum/union body, a trait body, or an inherent `impl`.
    Type { module: String, name: String },
    /// An `impl Trait for Type` body.
    ///
    /// **Almost always keyed exactly like an inherent one** — that is stage
    /// 11's S8: a merge key must be what BOTH sides can produce, and a caller
    /// writing `p.draw()` cannot spell which trait supplies `draw`. The trait
    /// survives as the `TraitImpl` edge [`Walk::trait_impl`] emits.
    ///
    /// The container is still distinct because of the ONE case where the trait
    /// must stay in the key: when the type ALSO declares that name inherently.
    /// Rust resolves `p.name()` to the inherent method, deterministically, and
    /// a caller wanting the other one writes `<P as Trait>::name`. Flattening
    /// those two makes a deliberately non-recursive call into a self-loop. See
    /// [`Walk::declare`].
    TraitImpl { module: String, ty: String, tr: String },
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
    /// Where each type is declared. See [`TypeHomes`] — the walk is TOLD this,
    /// never looks it up, and an absent entry leaves the block's own module in
    /// place rather than producing a guess.
    types: &'a TypeHomes,
    /// Type name -> its fields and their declared types.
    ///
    /// On the WALK and not on a scope, because an `impl` block needs the fields
    /// of a struct declared elsewhere in the file — possibly after it. A scope
    /// only flows downward and could not reach them.
    declared_fields: BTreeMap<String, BTreeMap<String, String>>,
    /// Member names each type declares in an INHERENT `impl` block of THIS
    /// FILE.
    ///
    /// The one thing that decides whether a trait impl's member keeps the trait
    /// in its key. Collected over the whole tree before any body is walked, for
    /// the reason [`Walk::collect_declared_fields`] gives: `impl Trait for P`
    /// may be written above `impl P`, and a single-pass walk would key the
    /// trait copy before it had seen the inherent one.
    ///
    /// Scoped to the FILE deliberately. A type whose inherent and trait impls
    /// live in different files is not covered — the two then mint one identity
    /// and A7 counts the collision, which is a far better failure than reaching
    /// for a repo-wide table (R7, and the barrier this stage removes).
    inherent_members: BTreeMap<String, BTreeSet<String>>,
    /// Every type THIS FILE declares, and THE MODULE IT WAS DECLARED IN.
    ///
    /// The module matters and a name alone is not a home. A member's identity
    /// carries the module its TYPE lives in — that is the merge contract (§2) —
    /// so a call inside `mod tests` to a type declared at file scope must name
    /// the member under the TYPE's module, not the caller's.
    ///
    /// MEASURED before this was a map: `SenseiConfig::brew_install_script` was
    /// declared with `config` as its module segment at config.rs:220 and called
    /// with `config::tests` at config.rs:459 — same file, same type, one segment
    /// apart, and so never merging.
    /// A test calling its own file's subject IS this shape, so it was systematic.
    ///
    /// The nearest home there is, and the one `TypeHomes` cannot supply: it is
    /// built from a completed pass over the whole scan, so a single-file read
    /// is handed an empty one. Consulting the file first is also simply
    /// correct — a type declared here lives here, whatever a later barrier
    /// says.
    declared_here: BTreeMap<String, String>,
    symbols: Vec<Symbol>,
    references: Vec<Reference>,
    relations: Vec<Relation>,
    imports: Vec<Import>,
}

impl<'a> Walk<'a> {
    /// Where a type lives: THIS FILE first — its declarations, then its
    /// IMPORTS — and only then the scan.
    ///
    /// **The imports were the missing rung, and the file had already proved it
    /// could read them.** `use crate::a::Widget` states the module outright, and
    /// the TYPE reference on a parameter of that type resolves through exactly
    /// that import; the CALL on the same receiver did not, because this
    /// function went from the file's own declarations straight to a global
    /// table. So a member of an imported type could only ever be named after a
    /// barrier had walked the whole repository — for a fact written at the top
    /// of the file.
    ///
    /// MEASURED over this repository: of every member reference the barrier
    /// resolves, 41% are types the file declares, 38% types it imports by name,
    /// 18% reachable through a wildcard import and 1.6% spelled inline — 98.1%
    /// stated by the file itself.
    ///
    /// `TypeHomes` stays as the LAST resort rather than the second, and is on
    /// its way out with the barrier: it answers for a type this file never
    /// mentions, which is the 1.9% that arrives through an imported function's
    /// return type.
    fn home_of(&self, ty: &str) -> Home<'_> {
        // THE TWO RUNGS THIS FILE ANSWERS ITSELF. Both are `Stated`, so an
        // identity minted from either is `Named` and may become an edge on the
        // file's own authority (S7).
        if let Some(module) = self.declared_here.get(ty) {
            return Home::Stated { module: module.as_str() };
        }
        if let Some(module) = self.imported_from(ty) {
            return Home::Stated { module };
        }
        // The table's answer is a different claim and is graded as one: this
        // file says nothing about where the type lives, so what it mints is a
        // `Candidate` and still needs a declaration to agree.
        match self.types.lookup(self.package, ty) {
            Home::Stated { module } | Home::Tabled { module } => Home::Tabled { module },
            other => other,
        }
    }

    /// The module an import of `ty` names, when this file imports it from a
    /// package-rooted path.
    ///
    /// `use crate::a::b::Widget` names module `a::b`. Only a `crate`-rooted
    /// path answers: a bare `use serde::Serialize` names a library we do not
    /// open (R5), and a `self`/`super` path is relative to a module this
    /// function is not told, so reading either as ours would mint a
    /// first-party identity for something that is not — which R4 ranks below
    /// answering nothing.
    fn imported_from(&self, ty: &str) -> Option<&str> {
        self.imports.iter().find_map(|import| {
            match &import.binds {
                Binding::Name(bound) if bound == ty => {
                    let path = import.path.strip_prefix("crate::")?;
                    // Everything before the type's own segment is its module.
                    let (module, last) = path.rsplit_once("::")?;
                    (last == ty && !module.is_empty()).then_some(module)
                }
                _ => None,
            }
        })
    }

    fn text(&self, node: Node<'_>) -> &'a str {
        &self.src[node.byte_range()]
    }

    fn field_text(&self, node: Node<'_>, field: &str) -> Option<&'a str> {
        node.child_by_field_name(field).map(|c| self.text(c))
    }

    /// Mint the identity of a declaration named `member` in the current
    /// container. One place, so a declaration and a use site of it cannot pick
    /// different forms.
    fn declare(&self, scope: &Scope, member: &str, reach: Reach) -> Result<Fqn, FqnError> {
        let lang = Language::Rust;
        let package = self.package;
        let composed = scope.module_here();
        let module = composed.as_str();
        match &scope.container {
            Container::File => {
                fqn::define(&Form::Item { lang, package, module, name: member, reach })
            }
            Container::Type { module, name: ty } => {
                let module = module.as_str();
                fqn::define(&Form::Member { lang, package, module, ty, member, reach })
            }
            // **THE ONE PLACE THE TRAIT STAYS IN A KEY** (S8, narrowed).
            //
            // Flat by default, because no caller can spell which trait supplies
            // a name. Qualified when the type ALSO declares that name
            // inherently, because then Rust itself keeps them apart — the
            // inherent method wins `p.name()`, and `<P as Trait>::name` is how
            // the other is reached. Merging those two turns a call written to
            // AVOID recursion into a self-loop, and a wrong edge is worse than
            // a missing one (R4).
            Container::TraitImpl { module, ty, tr } => {
                let module = module.as_str();
                let shadowed = self
                    .inherent_members
                    .get(ty.as_str())
                    .is_some_and(|names| names.contains(member));
                match shadowed {
                    true => fqn::define(&Form::TraitMember {
                        lang,
                        package,
                        module,
                        ty,
                        tr,
                        member,
                        reach,
                    }),
                    false => {
                        fqn::define(&Form::Member { lang, package, module, ty, member, reach })
                    }
                }
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

    /// A block, walked in ORDER so each `let` types the statements after it.
    ///
    /// Order is the whole of it. The binding is recorded AFTER its own
    /// initialiser is walked — so `let x = x.foo()` reads the OUTER `x` — and
    /// before the next statement, so the following lines see it. The scope is
    /// a clone, so nothing recorded here escapes the block.
    ///
    /// Shadowing works by construction: a second `let x` overwrites the first
    /// from that point on, which is what Rust does — [`Walk::forget`] is the
    /// half of "overwrites" that a bare `insert` does not do.
    fn block(&mut self, node: Node<'_>, scope: &Scope) {
        let mut scope = scope.clone();
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.node(child, &scope);
            if child.kind() != "let_declaration" {
                continue;
            }
            // WHATEVER these names held, they no longer hold it. Before the two
            // reads below and never instead of them, so a `let` the walk CAN
            // read replaces the answer and one it cannot read removes it.
            if let Some(pattern) = child.child_by_field_name("pattern") {
                self.forget(&mut scope, pattern);
            }
            // A type the source STATED always wins. The callee is recorded only
            // where there is none, so a lookup can never overrule a declaration.
            if let Some((name, ty)) = self.binding_of(child) {
                scope.bindings.insert(name, ty);
            } else if let Some((name, callee)) = self.bound_to_a_call(child) {
                scope.bound_to_a_call.insert(name, callee);
            }
        }
    }

    /// **S1, which this walk had only half of.** Everything a binding form's
    /// names held before it ran, dropped — from BOTH tables, because they go
    /// stale independently.
    ///
    /// `let h = raw(); let h = h.finish();` is ordinary Rust and neither table
    /// can read the second line: `binding_of` finds no stated type, and
    /// `bound_to_a_call` refuses a method call on purpose. Without this the
    /// first line's answer stood, and every later `h.<member>` was typed as a
    /// `Raw` — a wrong edge pointing at a real node, which R4 ranks below no
    /// edge at all. The TypeScript walk has cleared on rebinding since 04b S1;
    /// this is the same rule, one language over.
    ///
    /// Every `identifier` UNDER the pattern, not only a pattern that IS one, so
    /// `let (a, b) = …` forgets both. The over-reading a struct pattern's
    /// shorthand might cause is safe in the one direction that matters: this
    /// only ever REMOVES, so the worst it can do is lose a type the walk knew
    /// and leave an honest miss (R4).
    fn forget(&self, scope: &mut Scope, pattern: Node<'_>) {
        let mut cursor = pattern.walk();
        let mut pending = vec![pattern];
        while let Some(node) = pending.pop() {
            if node.kind() == "identifier" {
                let name = self.text(node);
                scope.bindings.remove(name);
                scope.bound_to_a_call.remove(name);
            }
            pending.extend(node.named_children(&mut cursor));
        }
    }

    /// `for <pattern> in <collection> { … }` — the pattern is bound to the
    /// collection's ELEMENT type for the body, and for the body only.
    ///
    /// The collection's own type comes from whatever already states it: a field
    /// of `self`, or a local binding. A collection this walk cannot type yields
    /// no binding, and the body's member calls stay unresolved — which is the
    /// honest answer, not a fallback.
    fn for_expression(&mut self, node: Node<'_>, scope: &Scope) {
        // The collection is walked in the OUTER scope: `for x in x.iter()` reads
        // the outer `x`, and binding first would type it as its own element.
        if let Some(value) = node.child_by_field_name("value") {
            self.node(value, scope);
        }

        // The PATTERN, which was skipped entirely — the walk went from the
        // collection straight to the body. A destructuring pattern NAMES a
        // type: `for Placed { facts, .. } in &corpus` writes `Placed` down, and
        // the graph had no edge for it. Walked in the OUTER scope for the same
        // reason the collection is, since nothing it binds is in scope yet.
        //
        // A2 is what makes this safe to walk wholesale rather than by picking
        // the shapes: the independent counter counts the same node kinds, so an
        // over-count would fail it as loudly as the under-count did. It is 0
        // files disagreeing across the corpus either way.
        if let Some(pattern) = node.child_by_field_name("pattern") {
            self.node(pattern, scope);
        }

        let mut inner = scope.clone();
        // A loop pattern BINDS, so it forgets what its names held outside the
        // loop for exactly the reason a `let` does — `for h in hs` after
        // `let h: Raw = …` is the same staleness with a different keyword.
        if let Some(pattern) = node.child_by_field_name("pattern") {
            self.forget(&mut inner, pattern);
        }
        if let Some(pattern) = node.child_by_field_name("pattern")
            // Only a plain name. A destructuring pattern binds several names of
            // several types, and giving each the element type is false.
            && pattern.kind() == "identifier"
            && let Some(collection) = node.child_by_field_name("value")
            && let Some(ty) = self.collection_type(collection, scope)
            && let Some(element) = element_type(&ty)
        {
            inner.bindings.insert(self.text(pattern).to_string(), element);
        }

        if let Some(body) = node.child_by_field_name("body") {
            self.node(body, &inner);
        }
    }

    /// The declared type of the expression being iterated, when something in
    /// scope states it.
    ///
    /// Two shapes, both already recorded: `self.field` and a local binding.
    /// Anything else — a call, a chain, a literal — states no type here.
    fn collection_type(&self, node: Node<'_>, scope: &Scope) -> Option<String> {
        let text = self.text(node).trim().trim_start_matches('&').trim_start();
        match text.strip_prefix("self.") {
            Some(field) => scope.fields.get(field).cloned(),
            None => scope.bindings.get(text).cloned(),
        }
    }

    /// The `(name, type)` a `let` STATES, or `None` when it states none.
    ///
    /// Two shapes are read, and deliberately only two:
    ///
    /// - `let x: T = …` — the annotation.
    /// - `let x = T::assoc(…)` / `let x = T { … }` — an initialiser whose path
    ///   names the type. WHERE in the path it names it differs by shape, and
    ///   the match below is what says which.
    ///
    /// Everything else returns `None`. `let x = helper()` names no type;
    /// inferring one from the function's return type is a different capability
    /// (it needs the graph, not the file) and guessing is not an option — the
    /// type becomes a SEGMENT of the identity, so a wrong one mints a wrong key
    /// and every edge built on it points somewhere real but incorrect.
    ///
    /// The type must be a SIMPLE name. `Vec<Config>` is rejected rather than
    /// truncated to `Vec`: a member of `Vec` is not a first-party identity this
    /// grammar can mint, and `Vec<Config>` is not a name any declaration
    /// carries. Both spellings would be wrong, so neither is recorded.
    fn binding_of(&self, node: Node<'_>) -> Option<(String, String)> {
        // Only a plain `let x`. A destructuring pattern binds several names of
        // several types, and attributing the whole type to each is false.
        let pattern = node.child_by_field_name("pattern")?;
        if pattern.kind() != "identifier" {
            return None;
        }
        let name = self.text(pattern).to_string();

        if let Some(ty) = node.child_by_field_name("type") {
            return simple_type_name(self.text(ty)).map(|t| (name, t));
        }

        let value = node.child_by_field_name("value")?;
        // Each shape says WHERE in its path the type is, because they disagree
        // and a shared answer is wrong for two of the three. What they have in
        // common is that the answer is handed to `simple_type_name` whole:
        // that function already reads a path's last segment, so peeling one
        // here as well would be a second place that decides what a path names.
        let names_the_type = match value.kind() {
            // `T::assoc(…)` — the last segment is the FUNCTION, so the type is
            // the one before it. Reading the path's head instead worked only
            // while the path was `T::assoc`; with a module or crate in front,
            // the head is `crate`/`sensei_bootstrap` and names no type, so the
            // binding went untyped and every member read off it was lost.
            "call_expression" => self.field_text(value, "function")?.rsplit_once("::")?.0,
            // `T { … }` — nothing follows the type, so the path names it.
            "struct_expression" => self.field_text(value, "name")?,
            // `let p = MacOSProvider;` — a UNIT STRUCT names its own type, and
            // like the literal above it has no trailing segment to drop.
            //
            // `simple_type_name` requires a capitalised head, which is what
            // keeps a `let x = some_fn;` out. A `const` in PascalCase would slip
            // through, but Rust names consts in SCREAMING_SNAKE by convention
            // and the resolver refuses a candidate that matches no declaration
            // anyway — so the failure mode is a miss, not a wrong edge.
            "identifier" | "scoped_identifier" => self.text(value),
            _ => return None,
        };
        simple_type_name(names_the_type).map(|t| (name, t))
    }

    /// The `(name, callee)` a `let` binds when it states no type but its value
    /// IS a call — `let s = pg_store().await`.
    ///
    /// The weaker sibling of [`Walk::binding_of`], and reached only when that
    /// one found nothing. It answers a different question: not "what type is
    /// this" — the file does not say — but "what call produced it", which the
    /// file does say and which a completed pass can turn into a type.
    ///
    /// Precedence lives at the one call site and not here as well. A `let` that
    /// states a type has already had its chance at [`Walk::binding_of`], and a
    /// second copy of that rule in this function would be a second place for it
    /// to be changed.
    ///
    /// PLUMBING IS PEELED, from the grammar's own list rather than by matching
    /// text here, so the one place that decides what plumbing is stays the one
    /// place. `.await` and `?` are peeled too: they are syntax rather than
    /// calls, so no list can name them and the node kind is what says so.
    ///
    /// The innermost callee must be a PLAIN PATH — an identifier or a scoped
    /// one. A method call (`events.begin()`) is refused: its own receiver is
    /// untyped, so the ladder could not place the callee either, and an uncapped
    /// chase is how a wrong type travels a long way quietly (R4).
    fn bound_to_a_call(&self, node: Node<'_>) -> Option<(String, String)> {
        let pattern = node.child_by_field_name("pattern")?;
        if pattern.kind() != "identifier" {
            return None;
        }
        let callee = self.callee_under_the_plumbing(node.child_by_field_name("value")?)?;
        Some((self.text(pattern).to_string(), callee))
    }

    /// The call a value expression ultimately is, with `.await`, `?` and
    /// plumbing hops peeled off the outside.
    ///
    /// Bounded by the nesting of the expression itself, so it terminates: each
    /// step strips one layer and recurses on a strictly smaller node.
    fn callee_under_the_plumbing(&self, value: Node<'_>) -> Option<String> {
        match value.kind() {
            // `pg_store().await` and `thing()?` — syntax, not calls, and they
            // hand back what the inner expression produced.
            "await_expression" | "try_expression" => {
                self.callee_under_the_plumbing(value.named_child(0)?)
            }
            "call_expression" => {
                let function = value.child_by_field_name("function")?;
                match function.kind() {
                    // The callee the ladder can place.
                    "identifier" | "scoped_identifier" => Some(self.text(function).to_string()),
                    // `inner(..).clone()` — a plumbing hop wrapping the real
                    // call. Anything else is a method call on a receiver this
                    // walk has not typed, and is refused.
                    "field_expression" => {
                        let hop = self.field_text(function, "field")?;
                        if !super::GRAMMAR.plumbing.contains(&hop) {
                            return None;
                        }
                        self.callee_under_the_plumbing(function.child_by_field_name("value")?)
                    }
                    _ => None,
                }
            }
            _ => None,
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
                self.plain(node, scope, SymbolKind::TypeAlias, Reach::Item)
            }
            "const_item" => self.plain(node, scope, SymbolKind::Const, Reach::Item),
            "static_item" => self.plain(node, scope, SymbolKind::Static, Reach::Item),
            "macro_definition" => self.plain(node, scope, SymbolKind::Macro, Reach::Macro),
            "mod_item" => self.module_item(node, scope),
            "impl_item" => self.impl_block(node, scope),
            // A block is the unit a `let` is scoped to, so it is walked in
            // order rather than as an unordered bag of children.
            "block" => self.block(node, scope),
            // A `for` binding is a DECLARATION: the collection states the
            // element type.
            "for_expression" => self.for_expression(node, scope),

            _ => {
                self.use_site(node, scope);
                self.children(node, scope);
            }
        }
    }

    /// Push a symbol and return the scope its body is walked in. Returns the
    /// scope unchanged when the declaration could not be named, so the body is
    /// still walked and nothing below it is lost.
    ///
    /// This is also the one place ownership is recorded, because it is the one
    /// place every declaration passes through: a member declared in a type's
    /// body is owned by that type, whichever arm read it. Doing it per-arm would
    /// be six chances to forget one.
    fn push(&mut self, symbol: Result<Symbol, FqnError>, scope: &Scope) -> Scope {
        match symbol {
            Ok(symbol) => {
                // A member is OWNED by its type. Everything else written
                // directly in a module is CONTAINED by that module. Exactly one
                // of the two, because both feed `nodes.parent_id` and a child
                // with two parents has none.
                let structure = match &scope.owner {
                    // Proven, not guessed: this identity was minted by the
                    // same rule that named the member, from a declaration
                    // this walk read, so the two sides cannot disagree.
                    Owner::Type(owner) => Some((RelationKind::Owns, owner.clone())),
                    // DIRECTLY in it, which is what the empty function chain
                    // says. A declaration inside a function body is held by
                    // that body, and calling it a child of the module would put
                    // a local `const` beside the file's public surface as if
                    // the two were the same kind of thing.
                    _ if scope.fn_scope.is_empty() => {
                        Some((RelationKind::Contains, scope.holder.clone()))
                    }
                    _ => None,
                };
                if let Some((kind, parent)) = structure {
                    self.relations.push(Relation {
                        kind,
                        child: symbol.fqn.clone(),
                        parent: Resolution::Resolved { fqn: parent, via: Rung::DeclaredHere },
                        at: symbol.span,
                    });
                }
                let inner = Scope { from: symbol.fqn.clone(), ..scope.clone() };
                self.symbols.push(symbol);
                inner
            }
            Err(_) => scope.clone(),
        }
    }

    /// [`Walk::push`] for a declaration whose own body declares members OF it,
    /// returning a scope in which those members are owned by it. A declaration
    /// whose identity could not be minted owns nothing, because there would be
    /// nothing for the edge to point at (R4).
    fn push_owner(&mut self, symbol: Result<Symbol, FqnError>, scope: &Scope) -> Scope {
        let owner = symbol.as_ref().ok().map(|s| s.fqn.clone());
        let mut inner = self.push(symbol, scope);
        inner.owner = match owner {
            Some(fqn) => Owner::Type(fqn),
            None => Owner::Nobody,
        };
        inner
    }

    fn symbol(
        &self,
        node: Node<'_>,
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
            span: span(node),
            visibility: self.visibility(node),
            docstring: self.docstring(node),
            declared_type,
            params: Vec::new(),
        })
    }

    /// A declaration whose whole identity is a `name` field: type alias,
    /// associated type, const, static, macro definition.
    fn plain(&mut self, node: Node<'_>, scope: &Scope, kind: SymbolKind, reach: Reach) {
        let Some(name) = self.field_text(node, "name") else {
            self.children(node, scope);
            return;
        };
        let declared = self.declared_type(node, "type");
        let symbol = self.symbol(node, scope, name, kind, reach, declared);
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
        let symbol = self.symbol(node, scope, name, kind, Reach::Item, declared).map(|mut s| {
            s.params = self.params(node);
            s
        });
        let mut inner = self.push(symbol, scope);
        // A parameter's type is STATED in the signature, so the body knows the
        // type of every name the signature binds. Same rule as a `let`, one
        // level up — and the same refusal to guess: a parameter whose type is
        // not a simple name records nothing.
        //
        // Bound on `inner`, the body's scope, so it cannot leak to a sibling
        // function; and set BEFORE the body is walked, because unlike a `let`
        // a parameter is in scope from the body's first statement.
        inner.bindings.extend(self.param_bindings(node));
        // The body is INSIDE this function, and everything it declares is named
        // so. A nested `fn` extends the chain rather than replacing it.
        inner.fn_scope.push(name.to_string());
        self.children(node, &inner);
    }

    /// Every method this impl block declares, with the TYPE it returns, for the
    /// ones that state a simple type. See [`Scope::returns`].
    fn method_returns(&self, impl_node: Node<'_>) -> BTreeMap<String, String> {
        let Some(body) = impl_node.child_by_field_name("body") else {
            return BTreeMap::new();
        };
        let mut cursor = body.walk();
        body.named_children(&mut cursor)
            .filter(|c| c.kind() == "function_item")
            .filter_map(|f| {
                let name = self.field_text(f, "name")?;
                let ret = self.field_text(f, "return_type")?;
                // `Self` names the type the impl is about, which the container
                // already states — resolved at the use site rather than here, so
                // there is one place that knows what `Self` means.
                simple_type_name(ret).map(|t| (name.to_string(), t))
            })
            .collect()
    }

    /// The `(name, type)` each parameter STATES, for the ones that state a
    /// simple type. `self` is deliberately absent: its type comes from the
    /// enclosing `impl`, which [`Walk::name_member`] already reads off the
    /// container — recording it here would be a second source for one fact.
    fn param_bindings(&self, node: Node<'_>) -> BTreeMap<String, String> {
        let Some(list) = node.child_by_field_name("parameters") else {
            return BTreeMap::new();
        };
        let mut cursor = list.walk();
        list.named_children(&mut cursor)
            .filter(|p| p.kind() == "parameter")
            .filter_map(|p| {
                let pattern = p.child_by_field_name("pattern")?;
                let ty = self.text(p.child_by_field_name("type")?);
                match pattern.kind() {
                    "identifier" => {
                        simple_type_name(ty).map(|t| (self.text(pattern).to_string(), t))
                    }
                    // `State(state): State<AppState>` — a wrapper binding ONE
                    // name. The old rule refused every pattern that was not a
                    // bare identifier, on the grounds that a destructuring
                    // parameter binds several names of several types. True of
                    // `(a, b): (X, Y)`; false here, and the type is written
                    // beside it.
                    "tuple_struct_pattern" => self.one_binding_of(pattern, ty),
                    // Still refused: several names, or a shape nothing states.
                    _ => None,
                }
            })
            .collect()
    }

    /// The `(name, type)` a ONE-BINDING tuple-struct pattern states, or `None`.
    ///
    /// Three gates, in this order, and the order is the point:
    ///
    /// 1. EXACTLY ONE binding. `Pair(a, b)` binds two names and no single type
    ///    belongs to both — this is the case the blanket refusal was written
    ///    for, and it stays refused. Checked FIRST, so a field-0 lookup can
    ///    never make a two-name pattern look answerable.
    /// 2. A FIRST-PARTY newtype is read from its own declaration: field 0's
    ///    declared type, which states what it holds instead of assuming it.
    /// 3. Otherwise an EXTERNAL wrapper on the named list, where the single
    ///    type argument is the answer. Anything else states nothing.
    fn one_binding_of(&self, pattern: Node<'_>, ty: &str) -> Option<(String, String)> {
        let mut cursor = pattern.walk();
        let children: Vec<Node<'_>> = pattern.named_children(&mut cursor).collect();
        // The FIRST child is the constructor — `State` in `State(state)` — and
        // the bindings are what follow it. Counting every identifier instead
        // would count the constructor as a binding and refuse every one-name
        // pattern as if it bound two.
        let [_constructor, bound @ ..] = &children[..] else { return None };
        // Exactly one, and a plain name: `Pair(a, b)` binds two names that no
        // single type belongs to, and `Wrap(Inner(x))` binds through a nested
        // pattern this does not follow.
        let [name] = bound else { return None };
        if name.kind() != "identifier" {
            return None;
        }
        let name = self.text(*name).to_string();

        // The wrapper's own name, off the declared TYPE rather than the
        // pattern: the type is what a lookup is keyed by, and a pattern may
        // spell a path (`axum::extract::State(s)`) the declaration never does.
        let head = simple_type_name(ty)?;
        if let Some(declared) = self.declared_fields.get(&head).and_then(|f| f.get("0")) {
            return simple_type_name(declared).map(|t| (name, t));
        }
        extracted_type(ty).map(|t| (name, t))
    }

    /// A struct, union, enum or trait: a named type whose body declares members
    /// of it. The body is walked with the type as the container, which is what
    /// makes its fields and methods members rather than free items.
    /// Every field this type declares, with the TYPE it is declared with.
    /// See [`Scope::fields`].
    /// Walk the whole tree once for struct field types. See
    /// [`Walk::declared_fields`].
    fn collect_declared_fields(&mut self, node: Node<'_>, module: &str) {
        // A nested `mod x` extends the module path for everything inside it —
        // which is the whole point: a type declared at file scope and one
        // declared in `mod tests` do not live in the same module, and a member
        // of either must be named under the one that declares its type.
        let here = match (node.kind(), self.field_text(node, "name")) {
            ("mod_item", Some(name)) if module.is_empty() => name.to_string(),
            ("mod_item", Some(name)) => format!("{module}::{name}"),
            _ => module.to_string(),
        };
        if matches!(node.kind(), "struct_item" | "union_item" | "enum_item" | "trait_item")
            && let Some(name) = self.field_text(node, "name")
        {
            self.declared_here.insert(name.to_string(), here.clone());
        }
        if matches!(node.kind(), "struct_item" | "union_item")
            && let Some(name) = self.field_text(node, "name")
        {
            let fields = self.field_types(node);
            if !fields.is_empty() {
                self.declared_fields.insert(name.to_string(), fields);
            }
        }
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.collect_declared_fields(child, &here);
        }
    }

    /// Every member name an INHERENT `impl` block of this file declares, by
    /// type. See [`Walk::inherent_members`].
    ///
    /// An `impl` with a `trait` field is skipped: what is being collected is
    /// exactly the set that can SHADOW a trait's copy of a name.
    fn collect_inherent_members(&mut self, node: Node<'_>) {
        if node.kind() == "impl_item"
            && node.child_by_field_name("trait").is_none()
            && let Some(raw) = self.field_text(node, "type")
            && let Ok(ty) = type_segment(raw)
            && let Some(body) = node.child_by_field_name("body")
        {
            let mut cursor = body.walk();
            let names: BTreeSet<String> = body
                .named_children(&mut cursor)
                .filter_map(|child| self.field_text(child, "name"))
                .map(str::to_string)
                .collect();
            self.inherent_members.entry(ty).or_default().extend(names);
        }
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.collect_inherent_members(child);
        }
    }

    fn field_types(&self, node: Node<'_>) -> BTreeMap<String, String> {
        let Some(body) = node.child_by_field_name("body") else {
            return BTreeMap::new();
        };
        let mut cursor = body.walk();
        // A TUPLE STRUCT names its fields by POSITION. Keyed by the index, in
        // the same spelling [`Walk::positional_fields`] uses when it emits
        // them — field "0", then "1" — so the type of a field and the identity
        // of that field cannot disagree about what it is called.
        if body.kind() == "ordered_field_declaration_list" {
            return body
                .children_by_field_name("type", &mut cursor)
                .enumerate()
                .map(|(position, ty)| (position.to_string(), self.text(ty).to_string()))
                .collect();
        }
        body.named_children(&mut cursor)
            .filter(|c| c.kind() == "field_declaration")
            .filter_map(|f| {
                let name = self.field_text(f, "name")?;
                let ty = self.field_text(f, "type")?;
                Some((name.to_string(), ty.to_string()))
            })
            .collect()
    }

    fn type_with_fields(&mut self, node: Node<'_>, scope: &Scope, kind: SymbolKind) {
        let Some(name) = self.field_text(node, "name") else {
            self.children(node, scope);
            return;
        };
        let symbol = self.symbol(node, scope, name, kind, Reach::Item, DeclaredType::Unstated);
        let mut inner = self.push_owner(symbol, scope);
        // A type declared HERE is named in this module, so its home is the
        // scope's own — no table needed and none consulted.
        inner.container = Container::Type { module: scope.module.clone(), name: name.to_string() };
        if let Owner::Type(child) = &inner.owner {
            self.supertraits(node, scope, child.clone());
        }
        self.children(node, &inner);
    }

    /// `trait Sub: Super` is the one thing Rust states in the shape `extends` is
    /// stated in: every `Sub` is a `Super`. A struct has no such bound, so this
    /// fires only where the grammar puts one.
    ///
    /// Only the three node kinds that NAME a type are read. A lifetime bound
    /// (`'a`), a relaxed bound (`?Sized`) and a higher-ranked one bound no
    /// supertrait, and the first two are not supertraits at all — a relation for
    /// either would say the opposite of what the source says (R4).
    fn supertraits(&mut self, node: Node<'_>, scope: &Scope, child: Fqn) {
        let Some(bounds) = node.child_by_field_name("bounds") else {
            return;
        };
        let mut cursor = bounds.walk();
        let named: Vec<Node<'_>> = bounds
            .named_children(&mut cursor)
            .filter(|b| {
                matches!(b.kind(), "type_identifier" | "scoped_type_identifier" | "generic_type")
            })
            .collect();
        for bound in named {
            let parent = self.name_type(bound, scope).resolution();
            self.relations.push(Relation {
                kind: RelationKind::Extends,
                child: child.clone(),
                parent,
                at: span(bound),
            });
        }
    }

    /// An enum variant is a member of its enum, reached the way any path leaf
    /// is, and its own fields are members of the variant. `Enum::Variant` as the
    /// type segment is what keeps two variants' same-named fields apart.
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
            Reach::Item,
            DeclaredType::Unstated,
        );
        let mut inner = self.push_owner(symbol, scope);
        if let Container::Type { module, name: enum_name } = &scope.container {
            inner.container =
                Container::Type { module: module.clone(), name: format!("{enum_name}::{name}") };
        }
        self.children(node, &inner);
    }

    fn named_field(&mut self, node: Node<'_>, scope: &Scope) {
        let Some(name) = self.field_text(node, "name") else {
            self.children(node, scope);
            return;
        };
        let declared = self.declared_type(node, "type");
        let symbol = self.symbol(node, scope, name, SymbolKind::Field, Reach::Field, declared);
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
            let symbol = self.symbol(ty, scope, &name, SymbolKind::Field, Reach::Field, declared);
            let inner = self.push(symbol, scope);
            // The type node is dispatched, not descended into: the field's type
            // is itself a use site and skipping it would lose the edge.
            self.node(ty, &inner);
        }
    }

    /// An inline `mod` extends the module path, so a declaration inside one is
    /// named the same as if it lived in its own file. Two spellings of one
    /// symbol would never merge (spec §2).
    ///
    /// [`MODULE`] and not [`Reach::Item`], for the reason recorded there — and
    /// this is the other half of the pair that must move with [`file_fqn`].
    fn module_item(&mut self, node: Node<'_>, scope: &Scope) {
        let Some(name) = self.field_text(node, "name") else {
            self.children(node, scope);
            return;
        };
        // A `mod` DECLARES a module only when the module's body is here.
        //
        // `mod x;` names a module whose body is another FILE, and that file
        // declares itself — same identity, by construction, because `file_fqn`
        // mints it exactly the way this arm would. Emitting a Symbol here as
        // well put TWO resolved definitions on one node with different
        // `file_id`, `line_start` and `is_exported`, and the upsert
        // (graph.rs:824-836) arbitrates only stub-vs-definition: every
        // definition column is gated on `EXCLUDED.resolved`, which both writers
        // set, so the LAST WRITER WINS. Proven by running the production SQL in
        // a rolled-back transaction: `declared_in` flipped to the parent,
        // `is_exported` was destroyed (only the parent can see `pub`), and the
        // span was overwritten. Under parallel per-file indexing the winner is
        // whoever commits second — order dependence, which R6/A6 forbid.
        //
        // The legacy walk has always read it this way (`rust_lang.rs` descends
        // only `if let Some(body)`), and the shipped graph therefore has one
        // declarer per module. This restores that, and leaves the parent's
        // `mod x;` to become a containment RELATION once `Contains` exists —
        // which is the stub-then-promote path: the relation names the child's
        // identity, minting a stub if that file has not been indexed yet, and
        // the stub is merged into the real declaration when it is.
        if node.child_by_field_name("body").is_none() {
            return;
        }
        let symbol =
            self.symbol(node, scope, name, SymbolKind::Module, MODULE, DeclaredType::Unstated);
        let mut inner = self.push(symbol, scope);
        // What is written inside this `mod` is held by it, not by the file.
        // `push` set `from` to the module's own identity, which is the same
        // string — read from there so the two cannot be minted apart.
        inner.holder = inner.from.clone();
        inner.module = if scope.module.is_empty() {
            name.to_string()
        } else {
            format!("{}::{name}", scope.module)
        };
        inner.container = Container::File;
        // A `mod` inside a function body re-roots the path: its contents are
        // reached as `<module>::<mod>::x`, not through the function.
        inner.fn_scope.clear();
        // A module is not a type, so what it contains are free items owned by
        // nothing — even when the `mod` itself sits inside a type's body.
        inner.owner = Owner::Nobody;
        self.children(node, &inner);
    }

    /// An `impl` is not a declaration — it is a container for the ones inside
    /// it, and an inherent block and a trait block are the SAME container.
    ///
    /// Whether a trait is named decides an EDGE ([`Walk::trait_impl`]) and no
    /// longer decides an identity: a method is keyed on its type and its name
    /// (stage 11, S8), because that is the only spelling a caller can produce.
    fn impl_block(&mut self, node: Node<'_>, scope: &Scope) {
        let Some(ty) = node.child_by_field_name("type").map(|n| self.text(n)) else {
            self.children(node, scope);
            return;
        };
        let Ok(ty) = type_segment(ty) else {
            let mut inner = scope.clone();
            inner.container = Container::Unnameable { raw: ty.to_string() };
            inner.owner = Owner::Nobody;
            self.children(node, &inner);
            return;
        };
        // WHERE THE MEMBERS OF THIS BLOCK ARE NAMED. The type's own module, not
        // this block's: `impl PgStore` in `db::pg_store::personas` declares
        // members of the `PgStore` in `db::pg_store`. When the scan has not been told
        // where the type lives — or has been told two places — the block's own
        // module stands, which is the previous behaviour and is right whenever
        // the type is declared here.
        // The GRADE does not matter here and `Home::module` says so. This block
        // DECLARES these members, so they are ours wherever the type came from
        // — `impl MyTrait for PathBuf` puts our methods in our module. Unlike
        // the use sites below, nothing is being PLACED here, so there is no
        // claim to grade; the block's own module is the answer when the scan
        // has said nothing or two things, not a stand-in for one.
        let home = match self.home_of(&ty).module() {
            Some(module) => module.to_string(),
            None => scope.module.clone(),
        };

        let mut inner = scope.clone();
        // A PRE-PASS over this block's methods, before any body is walked: a
        // method may be called before it is declared, and a single-pass walk
        // would not have its return type yet. `returns` is REPLACED rather than
        // extended, so an outer impl's methods cannot type a call here.
        inner.returns = self.method_returns(node);
        // The fields of the type this impl is ABOUT, so `self.field` can be
        // typed inside its methods. Looked up from what the struct declaration
        // recorded, which may be anywhere in the file — hence the map on the
        // walk rather than a scope that only flows downward.
        inner.fields = match self.declared_fields.get(&ty) {
            Some(fields) => fields.clone(),
            // A type declared in ANOTHER file has no fields to find here. This
            // empty map is genuinely empty and not a failed read — which is the
            // distinction the guard against a defaulted value exists to force
            // somebody to state.
            None => BTreeMap::new(),
        };
        inner.container = match node
            .child_by_field_name("trait")
            .map(|n| self.text(n))
            .and_then(|raw| type_segment(raw).ok())
        {
            Some(tr) => Container::TraitImpl { module: home.clone(), ty: ty.clone(), tr },
            None => Container::Type { module: home.clone(), name: ty.clone() },
        };
        inner.owner = Owner::Nobody;
        // A use site in the impl header sits inside no member, so it belongs to
        // the type the impl is about.
        //
        // AT `home`, NOT AT THIS BLOCK'S MODULE, and for the same reason the
        // container above is. An `impl` block states facts about a type that
        // lives wherever it was declared, and this file may only have imported
        // it. Minting the owner under `scope.module` made ONE header emit two
        // identities for one type: the members went to the type's home (right)
        // and both the `Owns` parent and the `TraitImpl` target went to the
        // block's own module (an identity no declaration mints), so a correct
        // node sat under a dangling parent.
        if let Ok(owner) = fqn::define(&Form::Item {
            lang: Language::Rust,
            package: self.package,
            module: &home,
            name: &ty,
            reach: Reach::Item,
        }) {
            inner.from = owner.clone();
            inner.owner = Owner::Type(owner.clone());
            self.trait_impl(node, scope, owner);
        }
        self.children(node, &inner);
    }

    /// `impl Trait for Type` states that `Type` IS a `Trait`. An inherent
    /// `impl Type { }` states no such thing — it only groups members — so it
    /// reaches here and emits nothing. That distinction is the whole point:
    /// pattern detection reads an implements edge as real, and an Adapter is
    /// "implements X and holds an X", so an inheritance edge on every impl block
    /// would name a large part of any repository an Adapter.
    fn trait_impl(&mut self, node: Node<'_>, scope: &Scope, child: Fqn) {
        let Some(named) = node.child_by_field_name("trait") else {
            return;
        };
        // `impl !Send for T` states the OPPOSITE of an implements edge, and the
        // grammar hands back a `trait` node of `Send` either way — measured by
        // deleting this guard, which turns `impl !Send for Widget` into
        // `TraitImpl Widget -> Send`. So the `!` has to be read off the header,
        // which is everything before the ` for ` that a trait impl always has.
        if self.text(node).split(" for ").next().is_some_and(|head| head.contains('!')) {
            return;
        }
        let parent = self.name_type(named, scope).resolution();
        self.relations.push(Relation {
            kind: RelationKind::TraitImpl,
            child,
            parent,
            at: span(node),
        });
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
            Some(callee) => self.name_callee(callee, scope, Reach::Item),
            None => unhandled(node, self.text(node), Reach::Item),
        };
        self.emit(scope, RefKind::Calls, node, miss);
    }

    /// Whatever stands in callee position. Every shape either names something or
    /// says which shape defeated it — there is no path out of here that emits
    /// nothing, which is the defect this rewrite exists to remove.
    fn name_callee(&self, callee: Node<'_>, scope: &Scope, reach: Reach) -> Miss {
        match callee.kind() {
            "identifier" | "scoped_identifier" => {
                let path = self.text(callee);
                unplaced(callee, path, reach, self.considered_path(path, scope, reach))
            }
            // A turbofish wraps the real callee; the type arguments are use
            // sites in their own right and are counted as such.
            "generic_function" => match callee.child_by_field_name("function") {
                Some(inner) => self.name_callee(inner, scope, reach),
                None => unhandled(callee, self.text(callee), reach),
            },
            "field_expression" => self.name_member(callee, scope, Reach::Item),
            _ => unhandled(callee, self.text(callee), reach),
        }
    }

    /// `receiver.member`, in either callee or read position. The member's
    /// identity depends on the receiver's TYPE, which the walk only knows when
    /// the receiver is `self` inside a type's own body — anywhere else that is a
    /// real, permanent-until-inference miss and is reported as one.
    fn name_member(&self, node: Node<'_>, scope: &Scope, reach: Reach) -> Miss {
        // Both children are malformed-source cases, and an empty string in
        // either would name a member that is not there. Say what was seen.
        let (Some(member), Some(receiver)) =
            (self.field_text(node, "field"), self.field_text(node, "value"))
        else {
            return unhandled(node, self.text(node), reach);
        };

        // `self` inside a type's body, or a local whose type the source STATED.
        // Nothing else: a receiver the file does not type is reported as such.
        let self_type = match (&scope.container, receiver) {
            (
                Container::Type { name: ty, .. } | Container::TraitImpl { ty, .. },
                "self" | "Self",
            ) => Some(ty.as_str()),
            // A local whose type the source stated.
            _ => scope.bindings.get(receiver).map(String::as_str).or_else(|| {
                // `self.m().member` — the inner call resolves already, so the
                // only missing fact is what `m` RETURNS, and `m` is a method of
                // this same type. Measured: 351 of 28,001 unresolved receivers.
                //
                // Only `self.` and only a bare method name: a deeper chain
                // (`self.a().b().c()`) needs the return type of something this
                // block does not declare, and guessing there is how a wrong
                // identity gets minted (R4).
                let inner = receiver.strip_prefix("self.")?;
                let method = inner.strip_suffix("()")?;
                if !method.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    return None;
                }
                let returned = scope.returns.get(method)?;
                // `Self` is this type — the container already knows which.
                Some(match (returned.as_str(), &scope.container) {
                    (
                        "Self",
                        Container::Type { name: ty, .. } | Container::TraitImpl { ty, .. },
                    ) => ty.as_str(),
                    _ => returned.as_str(),
                })
            }),
        };
        let Some(ty) = self_type else {
            // Nothing here types the receiver. If it is a local bound to a
            // CALL, say which — the ladder has a completed pass and can read
            // what that call returns, which this file cannot (R6).
            let mut saw = vec![Observation::Receiver(receiver.to_string())];
            if let Some(callee) = scope.bound_to_a_call.get(receiver) {
                saw.push(Observation::BoundToTheResultOf(callee.clone()));
            }
            return Miss {
                reason: Reason::ReceiverTypeUnknown,
                name: member.to_string(),
                node_kind: node.kind().to_string(),
                reach,
                saw,
            };
        };
        // The type's home, exactly as the declaration side used it. Minting the
        // USE SITE's module here is what made a call resolve only when the
        // caller happened to share a module with the impl block — the two sides
        // agreeing is the merge contract (§2), not an optimisation.
        // **WHERE THE GRADE IS DECIDED** (S7). The identity is the same string
        // either way; what differs is whether THIS FILE established it. A file
        // that declares `Widget`, or imports it by a package-rooted path, has
        // said where `Widget` lives — so `w.wide()` may become an edge on the
        // file's own authority. A home the barrier TABLE supplied is a guess
        // about the scan, and still needs a declaration to agree.
        match self.home_of(ty) {
            Home::Stated { module } | Home::Tabled { module } => {
                let stated = matches!(self.home_of(ty), Home::Stated { .. });
                let minted = fqn::refer(&Form::Member {
                    lang: Language::Rust,
                    package: self.package,
                    module,
                    ty,
                    member,
                    reach,
                });
                let saw = match stated {
                    true => named(minted),
                    false => considered(minted),
                };
                unplaced(node, member, reach, saw)
            }
            // Two first-party types answer to this name, so there is no one
            // home and picking would be a coin toss recorded as a fact.
            Home::Ambiguous => Miss {
                reason: Reason::AmbiguousCandidates,
                name: member.to_string(),
                node_kind: node.kind().to_string(),
                reach,
                saw: vec![Observation::Receiver(receiver.to_string())],
            },
            // NOTHING OF OURS DECLARES THIS TYPE. The receiver IS known — we
            // read it off a declaration — and it is outside, so this is the
            // boundary and not a lookup that failed.
            //
            // Before this, the use site's own module stood in and the walk
            // minted a first-party member identity for `Path::join` under the
            // using file's own module: an identity for a type we do not
            // declare. MEASURED at 6,109 sites, 5,671 of them surfacing as
            // `NoImportInScope` — sending a reader after an import that was
            // never the issue.
            //
            // Handing the ladder the PATH instead (`Path::join`, via
            // `Observation::UnplacedType`) would be better still: it knows the
            // imports and could name the library member directly, a resolved
            // external edge. MEASURED and NOT DONE: a path with no import to
            // bind it gets placed first-party by a later rung instead, which
            // took dangling identities from 312 to 683. The ladder needs to be
            // told the type is external, which a bare path cannot say.
            Home::NotOurs => Miss {
                reason: Reason::ExternalBoundary,
                name: member.to_string(),
                node_kind: node.kind().to_string(),
                reach,
                saw: vec![Observation::Receiver(receiver.to_string())],
            },
        }
    }

    fn member_access(&mut self, node: Node<'_>, scope: &Scope) {
        // A read and a write of one field are different facts, and pattern
        // detection reads the difference (R8).
        let kind = if is_assignment_target(node) { RefKind::Writes } else { RefKind::Reads };
        let miss = self.name_member(node, scope, Reach::Field);
        self.emit(scope, kind, node, miss);
    }

    fn macro_use(&mut self, node: Node<'_>, scope: &Scope) {
        let miss = match node.child_by_field_name("macro") {
            Some(name) => {
                let path = self.text(name);
                Miss {
                    node_kind: node.kind().to_string(),
                    ..unplaced(
                        name,
                        path,
                        Reach::Macro,
                        self.considered_path(path, scope, Reach::Macro),
                    )
                }
            }
            None => unhandled(node, self.text(node), Reach::Macro),
        };
        self.emit(scope, RefKind::MacroInvokes, node, miss);
    }

    fn construct(&mut self, node: Node<'_>, scope: &Scope) {
        let miss = match node.child_by_field_name("name") {
            Some(name) => self.name_type(name, scope),
            None => unhandled(node, self.text(node), Reach::Item),
        };
        self.emit(scope, RefKind::Constructs, node, miss);
    }

    fn path_use(&mut self, node: Node<'_>, scope: &Scope) {
        let path = self.text(node);
        let miss =
            unplaced(node, path, Reach::Item, self.considered_path(path, scope, Reach::Item));
        self.emit(scope, RefKind::Reads, node, miss);
    }

    fn type_use(&mut self, node: Node<'_>, scope: &Scope) {
        let miss = self.name_type(node, scope);
        self.emit(scope, RefKind::TypeUse, node, miss);
    }

    /// `Self` spelled as the type it means.
    ///
    /// `Self::new()` and `-> Self` name the type the enclosing `impl` or trait
    /// body is about, which [`Container`] already holds — so minting a type
    /// literally called `Self` files a reference under a name no declaration
    /// carries. MEASURED: 33 references named `Self` and could reach nothing.
    ///
    /// Only the LEADING segment is rewritten, and only when it is exactly
    /// `Self`: a type genuinely named `SelfTest` is not this, and a `Self` at
    /// the file level is inside no type and stays as it was rather than being
    /// attached to a guess.
    fn concrete(&self, scope: &Scope, raw: &str) -> String {
        let ty = match &scope.container {
            Container::Type { name, .. } => name.as_str(),
            Container::TraitImpl { ty, .. } => ty.as_str(),
            Container::File | Container::Unnameable { .. } => return raw.to_string(),
        };
        match raw.strip_prefix("Self") {
            Some("") => ty.to_string(),
            Some(rest) if rest.starts_with("::") => format!("{ty}{rest}"),
            _ => raw.to_string(),
        }
    }

    /// A type named in a signature, a field, a bound or a construction. The one
    /// owner of type-text normalisation is `fqn::type_segment`, so the use side
    /// and the definition side reduce `Widget<T>` and `Widget` the same way.
    fn name_type(&self, node: Node<'_>, scope: &Scope) -> Miss {
        let raw = self.text(node);
        let Ok(path) = type_path(raw) else {
            return unhandled(node, raw, Reach::Item);
        };
        let Ok(name) = type_segment(&path) else {
            return unhandled(node, raw, Reach::Item);
        };
        let name = self.concrete(scope, &name);
        let mut saw = considered(fqn::refer(&Form::Item {
            lang: Language::Rust,
            package: self.package,
            module: &scope.module,
            name: &name,
            reach: Reach::Item,
        }));
        // The name alone cannot say WHICH `Widget` is meant; the path can, and
        // it is the only place the root word of `crate::db::PgStore` survives
        // the reduction to a segment. Discarding it here would be discarding
        // something already parsed.
        saw.push(Observation::UnplacedType(path));
        unplaced(node, &name, Reach::Item, saw)
    }

    /// The identity a path COULD name if it named something in this module —
    /// CONSIDERED, never proven, which is why it goes into the evidence and not
    /// into the resolution. Only the shapes whose reading is unambiguous get
    /// one; a longer path needs the import table, which is the ladder's.
    fn considered_path(&self, raw: &str, scope: &Scope, reach: Reach) -> Vec<Observation> {
        let lang = Language::Rust;
        let package = self.package;
        let module = scope.module.as_str();
        // Turbofish arguments decorate a path without naming a segment of it.
        // `Self` is the enclosing type, which the container already knows —
        // see `Walk::concrete`.
        let raw = &self.concrete(scope, raw);
        let segments: Vec<&str> = raw
            .split("::")
            .map(str::trim)
            .filter(|s| !s.starts_with('<') && !s.is_empty())
            .collect();
        match segments.as_slice() {
            [name] => considered(fqn::refer(&Form::Item { lang, package, module, name, reach })),
            [ty, member] if !matches!(*ty, "crate" | "self" | "super") => match type_segment(ty) {
                // Only when the scan DECLARES the type. `Vec::new()` used to
                // mint a first-party member identity for `Vec::new` off the
                // using file's own module — `Vec` headed the measured list at
                // 1,138 sites. An
                // empty candidate list leaves the bare path for the ladder,
                // which resolves it through the import that brought `Vec` in.
                // Graded the same way and for the same reason as `name_member`:
                // `Config::load()` where this file imports `Config` by a
                // package-rooted path is the file naming the member, not the
                // walk guessing at it.
                Ok(ty) => {
                    let minted = |module| {
                        fqn::refer(&Form::Member { lang, package, module, ty: &ty, member, reach })
                    };
                    match self.home_of(&ty) {
                        Home::Stated { module } => named(minted(module)),
                        Home::Tabled { module } => considered(minted(module)),
                        Home::Ambiguous | Home::NotOurs => Vec::new(),
                    }
                }
                Err(_) => Vec::new(),
            },
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
            target: miss.resolution(),
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
