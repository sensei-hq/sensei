//! PHP — identity rules and the grammar the ladder climbs.
//!
//! The walk is in [`walk`]; this file owns what a PHP name MEANS, which is R7's
//! split.
//!
//! # A namespace is the package
//!
//! `namespace App\Domain\Billing;` is written in the source and is the complete
//! answer to where a declaration lives, so a PHP fqn carries it in `<package>`
//! and leaves `<module>` EMPTY — the model Java, C# and Kotlin share. PSR-4 maps
//! a namespace onto a directory by convention, but the convention is the
//! autoloader's and the source is authoritative.
//!
//! # PHP says `extends` and `implements` in DIFFERENT clauses
//!
//! This is the one place PHP is EASIER than C# and Kotlin. Those two put a base
//! class and an interface in one list with one syntax, so each had to record a
//! decision about which relation to emit. PHP has `base_clause` for `extends`
//! and `class_interface_clause` for `implements`, so the relation is READ rather
//! than chosen — the same footing Java is on.
//!
//! # Three things worth naming
//!
//! - **Top-level functions.** A `function` may sit outside any class, so the
//!   file container carries weight as it does in Kotlin.
//! - **Traits.** A trait is a declaration in its own right and a `use` INSIDE a
//!   class body mixes one in — a different `use` from the import at file scope,
//!   spelled the same way.
//! - **Constructor property promotion.** `function __construct(private int $id)`
//!   declares a property, the shape C#'s positional records and Kotlin's primary
//!   constructors also have, and the grammar gives it its own node.
//!
//! # The two rules every language here has needed
//!
//! Built in from the start:
//!
//! 1. **A container is a PATH, not a leaf.**
//! 2. **A body does not declare members of the type enclosing it.**
//!
//! PHP has NO METHOD OVERLOADING, so unlike Java, C# and Kotlin there is no
//! callable-plus-arm split here — two methods of one name in one class is a
//! fatal error the language refuses, not a shape to represent.

use std::sync::LazyLock;

use super::{LanguageAdapter, Source, TypeHomes};
use crate::indexer::facts::{FileFacts, Fqn, Language};
use crate::indexer::fqn::{self, Form, FqnError, Reach, Segment};
use crate::indexer::resolve::Grammar;

mod walk;

/// The names PHP puts in every file with no import, as `(name, package, path)`.
///
/// The package is `php` — the RUNTIME, which is what ships these. They live in
/// the global namespace and so have no namespace to name, but "no package" is
/// not a package the ladder can file a reference under, and `php` is the real
/// thing that defines them rather than a placeholder standing in for one.
///
/// FUNCTIONS as well as classes, and they carry most of the weight. A PHP file
/// calls `count`, `array_map`, `sprintf` and `json_encode` with nothing
/// imported — the same shape as a Rust prelude trait — so leaving them out
/// would put the entire standard library at the head of the miss histogram.
const PRELUDE: &[(&str, &str, &str)] = &[
    // ── the class hierarchy every codebase touches ───────────────────────────
    ("Exception", "php", "Exception"),
    ("Throwable", "php", "Throwable"),
    ("Error", "php", "Error"),
    ("TypeError", "php", "TypeError"),
    ("ValueError", "php", "ValueError"),
    ("ArgumentCountError", "php", "ArgumentCountError"),
    ("RuntimeException", "php", "RuntimeException"),
    ("InvalidArgumentException", "php", "InvalidArgumentException"),
    ("LogicException", "php", "LogicException"),
    ("OutOfRangeException", "php", "OutOfRangeException"),
    ("OutOfBoundsException", "php", "OutOfBoundsException"),
    ("UnexpectedValueException", "php", "UnexpectedValueException"),
    ("DomainException", "php", "DomainException"),
    ("LengthException", "php", "LengthException"),
    ("BadMethodCallException", "php", "BadMethodCallException"),
    ("ArrayObject", "php", "ArrayObject"),
    ("ArrayIterator", "php", "ArrayIterator"),
    ("ArrayAccess", "php", "ArrayAccess"),
    ("Countable", "php", "Countable"),
    ("Iterator", "php", "Iterator"),
    ("IteratorAggregate", "php", "IteratorAggregate"),
    ("Traversable", "php", "Traversable"),
    ("JsonSerializable", "php", "JsonSerializable"),
    ("Serializable", "php", "Serializable"),
    ("Stringable", "php", "Stringable"),
    ("Closure", "php", "Closure"),
    ("Generator", "php", "Generator"),
    ("SplStack", "php", "SplStack"),
    ("SplQueue", "php", "SplQueue"),
    ("SplObjectStorage", "php", "SplObjectStorage"),
    ("SplFileInfo", "php", "SplFileInfo"),
    ("SplFileObject", "php", "SplFileObject"),
    ("DateTime", "php", "DateTime"),
    ("DateTimeImmutable", "php", "DateTimeImmutable"),
    ("DateTimeInterface", "php", "DateTimeInterface"),
    ("DateInterval", "php", "DateInterval"),
    ("DateTimeZone", "php", "DateTimeZone"),
    ("PDO", "php", "PDO"),
    ("PDOStatement", "php", "PDOStatement"),
    ("PDOException", "php", "PDOException"),
    ("ReflectionClass", "php", "ReflectionClass"),
    ("ReflectionMethod", "php", "ReflectionMethod"),
    ("ReflectionProperty", "php", "ReflectionProperty"),
    ("SimpleXMLElement", "php", "SimpleXMLElement"),
    ("stdClass", "php", "stdClass"),
    // ── the functions a PHP file calls with nothing written ──────────────────
    ("count", "php", "count"),
    ("is_array", "php", "is_array"),
    ("is_null", "php", "is_null"),
    ("is_string", "php", "is_string"),
    ("is_numeric", "php", "is_numeric"),
    ("is_object", "php", "is_object"),
    ("is_callable", "php", "is_callable"),
    ("isset", "php", "isset"),
    ("empty", "php", "empty"),
    ("array_map", "php", "array_map"),
    ("array_filter", "php", "array_filter"),
    ("array_merge", "php", "array_merge"),
    ("array_keys", "php", "array_keys"),
    ("array_values", "php", "array_values"),
    ("array_key_exists", "php", "array_key_exists"),
    ("array_reduce", "php", "array_reduce"),
    ("array_slice", "php", "array_slice"),
    ("array_search", "php", "array_search"),
    ("array_column", "php", "array_column"),
    ("array_combine", "php", "array_combine"),
    ("array_unique", "php", "array_unique"),
    ("array_push", "php", "array_push"),
    ("array_pop", "php", "array_pop"),
    ("array_shift", "php", "array_shift"),
    ("array_unshift", "php", "array_unshift"),
    ("in_array", "php", "in_array"),
    ("implode", "php", "implode"),
    ("explode", "php", "explode"),
    ("sprintf", "php", "sprintf"),
    ("printf", "php", "printf"),
    ("str_replace", "php", "str_replace"),
    ("str_repeat", "php", "str_repeat"),
    ("str_contains", "php", "str_contains"),
    ("str_starts_with", "php", "str_starts_with"),
    ("str_ends_with", "php", "str_ends_with"),
    ("str_pad", "php", "str_pad"),
    ("strlen", "php", "strlen"),
    ("strpos", "php", "strpos"),
    ("substr", "php", "substr"),
    ("strtolower", "php", "strtolower"),
    ("strtoupper", "php", "strtoupper"),
    ("ucfirst", "php", "ucfirst"),
    ("trim", "php", "trim"),
    ("preg_match", "php", "preg_match"),
    ("preg_match_all", "php", "preg_match_all"),
    ("preg_replace", "php", "preg_replace"),
    ("preg_split", "php", "preg_split"),
    ("preg_quote", "php", "preg_quote"),
    ("json_encode", "php", "json_encode"),
    ("json_decode", "php", "json_decode"),
    ("file_get_contents", "php", "file_get_contents"),
    ("file_put_contents", "php", "file_put_contents"),
    ("file_exists", "php", "file_exists"),
    ("fopen", "php", "fopen"),
    ("fclose", "php", "fclose"),
    ("dirname", "php", "dirname"),
    ("basename", "php", "basename"),
    ("sort", "php", "sort"),
    ("usort", "php", "usort"),
    ("uasort", "php", "uasort"),
    ("ksort", "php", "ksort"),
    ("intval", "php", "intval"),
    ("floatval", "php", "floatval"),
    ("strval", "php", "strval"),
    ("number_format", "php", "number_format"),
    ("round", "php", "round"),
    ("max", "php", "max"),
    ("min", "php", "min"),
    ("abs", "php", "abs"),
    ("date", "php", "date"),
    ("time", "php", "time"),
    ("strtotime", "php", "strtotime"),
    ("var_dump", "php", "var_dump"),
    ("print_r", "php", "print_r"),
    ("get_class", "php", "get_class"),
    ("method_exists", "php", "method_exists"),
    ("property_exists", "php", "property_exists"),
    ("class_exists", "php", "class_exists"),
    ("function_exists", "php", "function_exists"),
    ("call_user_func", "php", "call_user_func"),
    ("call_user_func_array", "php", "call_user_func_array"),
    ("func_get_args", "php", "func_get_args"),
    ("compact", "php", "compact"),
    ("extract", "php", "extract"),
];

/// Members on every object, and the magic methods the runtime calls rather than
/// any caller. A call to one says nothing about the receiver's type.
const PLUMBING: &[&str] = &[
    "__construct",
    "__destruct",
    "__toString",
    "__get",
    "__set",
    "__isset",
    "__unset",
    "__call",
    "__callStatic",
    "__invoke",
    "__clone",
    "__serialize",
    "__unserialize",
    // ArrayAccess, implemented by a large fraction of receivers.
    "offsetGet",
    "offsetSet",
    "offsetExists",
    "offsetUnset",
    "jsonSerialize",
];

/// A PHP type name is `StudlyCaps` — PSR-1, which every autoloader in the
/// ecosystem depends on.
///
/// The SAME rule Java, C# and Kotlin resolve by, so it is the shared one.
///
/// PHP's namespace segments are `StudlyCaps` TOO, where Java's packages are
/// lowercase, so this predicate cannot tell `App` from `User` in
/// `App\Models\User`. That is not a defect it hides: `Ladder::specifier` strips
/// the package's own segments before anything reaches `Ladder::identity`, so
/// what that rung reads is `["User"]` or `["User", "save"]` and never the
/// namespace in front of them. The one place the ambiguity does bite is
/// [`Grammar::paths_name_packages`] — see the note there.
use crate::indexer::lang::common::names_a_type_by_leading_case as names_a_type;

pub static GRAMMAR: LazyLock<Grammar> = LazyLock::new(|| Grammar {
    language: Language::Php,
    // The BACKSLASH, which is PHP's namespace separator and nothing else's.
    path_separator: "\\",
    module_separator: "\\",
    // EMPTY. A leading `\` means the GLOBAL namespace, which is where an
    // unqualified built-in already resolves from, so it roots nothing the
    // ladder has to climb to separately.
    roots: &[],
    module_segment: crate::indexer::lang::common::already_a_module_segment,
    relative_depth_prefix: None,
    names_the_binding: None,
    // `use App\Models\{A, B};` is a GROUP, not a wildcard — PHP has no
    // `use App\*`, so there is no spelling to list.
    wildcard: None,
    // TRUE, and it fires rarely on purpose. `a_fully_qualified_external` refuses
    // a path whose head names a type, and a PHP namespace head is `StudlyCaps`
    // like a class — so `Illuminate\Support\Facades\DB` is refused rather than
    // read as a package. That is the fail-closed side of the ambiguity: a miss,
    // never a library node minted out of a class name. It still answers for the
    // lowercase vendor namespaces that do exist.
    paths_name_packages: true,
    relative_to_directory: false,
    names_a_type,
    prelude: PRELUDE,
    plumbing: PLUMBING,
});

/// The type a PHP type expression NAMES.
pub(super) fn type_segment(raw: &str) -> Result<String, FqnError> {
    let bare = raw.trim();
    // A NULLABLE names the type it makes nullable: `?User` is a use of `User`.
    let bare = bare.strip_prefix('?').unwrap_or(bare).trim();
    // A UNION names several types. The first is taken here and the walk emits
    // the rest as their own use sites, so nothing is lost — and a segment
    // holding `A|B` would be an identity no declaration can equal.
    let bare = bare.split('|').next().unwrap_or(bare).trim();
    // `App\Models\User` at a use site names `User`; the namespace in front of it
    // is how it was reached.
    let last = bare.rsplit('\\').next().unwrap_or(bare).trim();
    if last.is_empty() {
        return Err(FqnError::EmptySegment { segment: Segment::Type });
    }
    Ok(last.to_string())
}

/// PHP, from `.php`.
pub struct PhpAdapter;

impl LanguageAdapter for PhpAdapter {
    fn language(&self) -> Language {
        Language::Php
    }

    fn name(&self) -> &'static str {
        "php"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &[".php"]
    }

    fn grammar(&self) -> &'static Grammar {
        &GRAMMAR
    }

    fn read(&self, source: &Source<'_>, types: &TypeHomes) -> Result<FileFacts, super::ReadError> {
        walk::read(source, types)
    }

    fn file_fqn(&self, package: &str, _module: &str, path: &str) -> Result<Fqn, FqnError> {
        let stem = path.rsplit('/').next().unwrap_or(path).trim_end_matches(".php");
        fqn::define(&Form::Item {
            lang: Language::Php,
            package,
            module: "",
            name: stem,
            reach: Reach::Mod,
        })
    }

    /// EMPTY, always — the namespace the walk reads IS where a declaration
    /// lives. PSR-4 makes the directory agree by convention, and a convention is
    /// the autoloader's rather than the source's.
    fn module_path(&self, _file: &str, _package_root: &str) -> String {
        String::new()
    }

    fn type_segment(&self, raw: &str) -> Result<String, FqnError> {
        type_segment(raw)
    }

    /// A PHP file's identity does not move when the file does — the namespace is
    /// declared IN the file.
    fn rename_remints_identity(&self, _from: &str, _to: &str, _package_root: &str) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The A7 ratchet for this corpus. A RATCHET, not a budget: it sits AT the
    /// measured value so a single new collision fails the gate.
    ///
    /// 312 of 72,420 declarations — 0.43% — over 2,263 files in three
    /// repositories, and every one is accounted for:
    ///
    ///   0    within ONE FILE. This is the bucket the walk controls, and it is
    ///        empty. C# left 52 here (a parse-recovery artifact) and Java 12,619
    ///        before the overload split; PHP has neither problem, because it
    ///        forbids redeclaration outright.
    ///   204  between two NAMESPACED files, which PSR-4 says cannot happen — and
    ///        all 204 sit in exactly two namespaces, `Grpc\Testing` (200) and
    ///        `GPBMetadata\Src\Proto\Grpc\Testing` (4). They are 24 pairs of
    ///        protoc-generated files checked into grpc TWICE, under
    ///        `tests/interop/` and `tests/qps/generated_code/`, differing only
    ///        in whether the descriptor is a raw string or `hex2bin`. Same
    ///        namespace, same class, same members — one identity is the correct
    ///        answer for a file duplicated verbatim.
    ///   108  involving a file in the GLOBAL namespace, which is CakePHP 2.x
    ///        (pre-PSR-4). `AppController` is declared by the app, by the
    ///        code-generator skeleton under `Console/Templates/skel/`, and by a
    ///        test fixture — three files that are never loaded together and
    ///        that the SOURCE gives nothing to tell apart. PHP would refuse all
    ///        three in one process.
    ///
    /// THE GLOBAL-NAMESPACE ONES ARE NOT PATCHED, and that is a decision rather
    /// than an oversight. The directory would discriminate them, but the ladder
    /// has no directory — every cross-file reference to a global-namespace class
    /// mints `<package>·<name>`, so moving the declaration side to a
    /// path-derived identity would trade these collisions for thousands of
    /// dangling edges, which R4 ranks worse.
    const A7_BOUND: usize = 312;
    use crate::indexer::facts::{
        Binding, DeclaredType, ImportOrigin, RefKind, RelationKind, Resolution, SymbolKind,
        Visibility,
    };

    fn seg(raw: &str) -> String {
        PhpAdapter.type_segment(raw).expect("the fixture names a type")
    }

    /// Read one PHP source, with no cross-file table.
    fn read(text: &str) -> FileFacts {
        let source = Source { package: "pkg", module: "", path: "src/Thing.php", text };
        walk::read(&source, &TypeHomes::unknown()).expect("the fixture parses")
    }

    /// Read one PHP source ANCHORED — a second pass that knows what the first
    /// declared, which is what lets a member land on a type this scan owns.
    fn read_anchored(text: &str) -> FileFacts {
        let source = Source { package: "pkg", module: "", path: "src/Thing.php", text };
        let first = walk::read(&source, &TypeHomes::unknown()).expect("the fixture parses");
        let homes = TypeHomes::of(first.symbols.iter().map(|s| (first.package.as_str(), s)));
        walk::read(&source, &homes).expect("the fixture parses")
    }

    fn named(facts: &FileFacts, name: &str) -> Vec<SymbolKind> {
        facts.symbols.iter().filter(|s| s.name == name).map(|s| s.kind).collect()
    }

    /// The namespace the FILE writes is where its declarations live — not the
    /// directory, and not what the caller handed in.
    ///
    /// MUTATION: drop `declared_namespace` and take `source.package` — every
    /// identity here reads `pkg` and no import in another file could ever reach
    /// one.
    #[test]
    fn the_file_states_its_own_namespace_and_that_is_the_package() {
        let facts = read("<?php\nnamespace App\\Domain;\nclass Invoice {}\n");
        assert_eq!(facts.package, "App\\Domain");
        assert!(facts.module.is_empty(), "the namespace is the whole of it");
        let invoice = facts.symbols.iter().find(|s| s.name == "Invoice").expect("declared");
        assert!(
            invoice.fqn.as_str().contains("App\\Domain"),
            "the identity carries the namespace: {}",
            invoice.fqn.as_str()
        );
    }

    /// PHP is the one language here that does not have to CHOOSE.
    ///
    /// C# and Kotlin put a base class and an interface in one list with one
    /// syntax, so each had to record a decision about which relation a name
    /// gets. PHP writes `extends` and `implements` in separate clauses and the
    /// grammar keeps them apart, so the relation is read off the source.
    ///
    /// MUTATION: map `class_interface_clause` to `Extends` — `Invoice` reads as
    /// having two base classes, which PHP cannot express.
    #[test]
    fn extends_and_implements_are_read_rather_than_decided() {
        let facts = read(
            "<?php\nnamespace App;\n\
             class Invoice extends Document implements Payable, Printable {}\n\
             interface Payable extends Chargeable {}\n",
        );
        let of = |kind: RelationKind| -> Vec<String> {
            facts
                .relations
                .iter()
                .filter(|r| r.kind == kind)
                .map(|r| match &r.parent {
                    Resolution::Unresolved { evidence, .. } => evidence.name.clone(),
                    Resolution::Resolved { fqn, .. } => fqn.as_str().to_string(),
                })
                .collect()
        };
        assert!(of(RelationKind::Extends).contains(&"Document".to_string()));
        assert!(of(RelationKind::Implements).contains(&"Payable".to_string()));
        assert!(of(RelationKind::Implements).contains(&"Printable".to_string()));
        // An INTERFACE spells its parents with `extends` too, and they are
        // interfaces. What the source wrote is what is recorded.
        assert!(of(RelationKind::Extends).contains(&"Chargeable".to_string()));
        assert!(
            !of(RelationKind::Extends).contains(&"Payable".to_string()),
            "an implemented interface is not a base class"
        );
    }

    /// `__construct(private Foo $bar)` declares a PROPERTY and takes a
    /// parameter. Both facts are true.
    ///
    /// The shape that cost C# (positional records) and Kotlin (`val` primary
    /// constructor parameters) real declarations when it was missed, and it is
    /// the dominant way modern PHP injects a dependency.
    ///
    /// MUTATION: stop emitting the property — `$this->repo` types as nothing
    /// and every call through an injected service becomes a receiver miss.
    #[test]
    fn a_promoted_constructor_parameter_declares_a_property_too() {
        let facts = read(
            "<?php\nnamespace App;\n\
             class Service {\n\
               public function __construct(private UserRepo $repo, public int $limit) {}\n\
             }\n",
        );
        assert_eq!(named(&facts, "repo"), vec![SymbolKind::Field], "the property is declared");
        assert_eq!(named(&facts, "limit"), vec![SymbolKind::Field]);
        let repo = facts.symbols.iter().find(|s| s.name == "repo").expect("declared");
        assert_eq!(repo.visibility, Visibility::Private);
        assert_eq!(repo.declared_type, DeclaredType::Stated("UserRepo".to_string()));
        // And it is a PARAMETER of the constructor, which is the other half.
        let ctor = facts.symbols.iter().find(|s| s.name == "__construct").expect("declared");
        assert_eq!(ctor.params.len(), 2);
        assert_eq!(ctor.params[0].name, "repo");
    }

    /// An injected collaborator is typable, so the call on it lands.
    ///
    /// `$this->repo->find()` is the single most common call shape in
    /// framework-written PHP. It needs the property table AND the one recursion
    /// in `type_of`.
    ///
    /// MUTATION: drop the `member_access_expression` arm of `type_of` — the
    /// call falls to `ReceiverTypeUnknown` and every service-to-service edge in
    /// the corpus disappears.
    #[test]
    fn a_call_through_an_injected_property_reaches_the_collaborator() {
        let facts = read_anchored(
            "<?php\nnamespace App;\n\
             class UserRepo { public function find(int $id): void {} }\n\
             class Service {\n\
               public function __construct(private UserRepo $repo) {}\n\
               public function go(): void { $this->repo->find(1); }\n\
             }\n",
        );
        let find = facts
            .references
            .iter()
            .find(|r| r.kind == RefKind::Calls && r.at.start_line == 6)
            .expect("the call is a reference");
        let Resolution::Resolved { fqn, .. } = &find.target else {
            panic!("the receiver is typed, so the call resolves: {:?}", find.target)
        };
        // The TYPE segment and the MEMBER segment, both — an identity carrying
        // `UserRepo` but some other member, or `find` under some other type,
        // would be the failure this is looking for. The trailing segment is the
        // REACH, which is what a method's own declaration mints.
        assert_eq!(fqn.as_str(), "php·App·UserRepo·find·item", "{}", fqn.as_str());
    }

    /// A trait is MIXED IN, not inherited.
    ///
    /// `instanceof` answers no for a trait, so collapsing the relation into
    /// `Extends` would make every trait look like a base class to pattern
    /// detection. The `use` inside a class body is also spelled exactly like
    /// the import at file scope, and only its position tells them apart.
    ///
    /// MUTATION: route `use_declaration` to the import handler — the trait
    /// becomes a phantom import binding `Loggable` and the mixin edge is lost.
    #[test]
    fn a_trait_use_inside_a_class_is_a_mixin_and_not_an_import() {
        let facts = read(
            "<?php\nnamespace App;\n\
             use App\\Support\\Helper;\n\
             class Service { use Loggable; }\n",
        );
        let mixins: Vec<&str> = facts
            .relations
            .iter()
            .filter(|r| r.kind == RelationKind::Mixin)
            .filter_map(|r| match &r.parent {
                Resolution::Unresolved { evidence, .. } => Some(evidence.name.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(mixins, vec!["Loggable"]);
        // The file-scope `use` is the import, and it is the ONLY one.
        assert_eq!(facts.imports.len(), 1);
        assert_eq!(facts.imports[0].path, "App\\Support\\Helper");
    }

    /// `Foo::class` NAMES the type. It reads no member.
    ///
    /// MUTATION: treat it as an ordinary constant read — every type in the
    /// corpus grows a phantom `class` member and the type use is lost.
    #[test]
    fn the_class_constant_names_the_type_rather_than_reading_a_member() {
        let facts = read(
            "<?php\nnamespace App;\n\
             class Service { public function go(): void { $x = Invoice::class; } }\n",
        );
        let uses: Vec<&str> = facts
            .references
            .iter()
            .filter(|r| r.kind == RefKind::TypeUse)
            .filter_map(|r| match &r.target {
                Resolution::Unresolved { evidence, .. } => Some(evidence.name.as_str()),
                _ => None,
            })
            .collect();
        assert!(uses.contains(&"Invoice"), "{uses:?}");
        assert!(
            !facts.references.iter().any(|r| matches!(
                &r.target,
                Resolution::Unresolved { evidence, .. } if evidence.name == "class"
            )),
            "`class` is not a member"
        );
    }

    /// A body declares GLOBALLY — PHP's own rule, and the second of the two
    /// rules every adapter here has needed.
    ///
    /// MUTATION: drop `in_body` from `declare` — `helper` is minted as a member
    /// of `Service`, an identity no call site can compose.
    #[test]
    fn a_function_declared_inside_a_body_is_not_a_member_of_the_enclosing_type() {
        let facts = read(
            "<?php\nnamespace App;\n\
             class Service {\n\
               public function go(): void { function helper(): int { return 1; } }\n\
             }\n",
        );
        let helper = facts.symbols.iter().find(|s| s.name == "helper").expect("declared");
        assert!(
            !helper.fqn.as_str().contains("Service"),
            "a body declares globally: {}",
            helper.fqn.as_str()
        );
        // And the one that IS a member still is.
        let go = facts.symbols.iter().find(|s| s.name == "go").expect("declared");
        assert!(go.fqn.as_str().contains("Service"), "{}", go.fqn.as_str());
    }

    /// Every shape a `use` takes binds the name a use site actually writes.
    ///
    /// MUTATION: ignore the `alias` field — `Customer` binds nothing and every
    /// reference through the alias misses.
    #[test]
    fn every_use_shape_binds_the_name_the_source_writes() {
        let facts = read(
            "<?php\nnamespace App;\n\
             use App\\Models\\User;\n\
             use App\\Models\\Order as Customer;\n\
             use App\\Support\\{Clock, Money};\n\
             use function App\\Support\\slugify;\n",
        );
        let bound: Vec<(&str, &str)> = facts
            .imports
            .iter()
            .map(|i| {
                let Binding::Name(name) = &i.binds else { panic!("php binds by name") };
                (i.path.as_str(), name.as_str())
            })
            .collect();
        assert!(bound.contains(&("App\\Models\\User", "User")), "{bound:?}");
        assert!(bound.contains(&("App\\Models\\Order", "Customer")), "the alias: {bound:?}");
        assert!(bound.contains(&("App\\Support\\Clock", "Clock")), "the group: {bound:?}");
        assert!(bound.contains(&("App\\Support\\Money", "Money")), "the group: {bound:?}");
        assert!(bound.contains(&("App\\Support\\slugify", "slugify")), "{bound:?}");
        // The namespace the name came FROM, which is what the ladder matches
        // against the packages this scan owns.
        let user = facts.imports.iter().find(|i| i.path == "App\\Models\\User").expect("imported");
        assert_eq!(user.origin, ImportOrigin::External { package: "App\\Models".to_string() });
    }

    /// A7, at the grain of one file: no two declarations mint one identity.
    ///
    /// The corpus gate below asks this of somebody else's checkout. This asks it
    /// of the shapes that have collided in every other language — a property, a
    /// promoted property, a constant and a method that all share a container.
    #[test]
    fn no_two_declarations_in_one_file_mint_one_identity() {
        use std::collections::BTreeMap;

        let facts = read(
            "<?php\nnamespace App;\n\
             const MODE = 'a';\n\
             function helper(): int { return 1; }\n\
             class Service {\n\
               public const MODE = 'b';\n\
               private string $mode = 'c';\n\
               public function __construct(private Clock $clock) {}\n\
               public function mode(): string { return $this->mode; }\n\
             }\n\
             enum Suit { case Hearts; case Spades; }\n",
        );
        let mut sites: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for symbol in &facts.symbols {
            sites.entry(symbol.fqn.as_str()).or_default().push(&symbol.name);
        }
        let colliding: Vec<_> = sites.iter().filter(|(_, at)| at.len() > 1).collect();
        assert!(colliding.is_empty(), "{colliding:?}");
        // And the two `MODE` are genuinely two, told apart by their container
        // AND their reach — the file-level one is an item, the class one a
        // field.
        assert_eq!(named(&facts, "MODE").len(), 2);
    }

    /// **A7 for PHP: no two declarations mint one identity.**
    ///
    /// Built BEFORE the flip, as it was for C#, because the absence of exactly
    /// this measurement is what let TypeScript reach 511 collisions, Java 409
    /// and Python 72 — each found only when someone went looking. This
    /// repository holds no PHP, so acceptance cannot measure it and the answer
    /// would be an empty denominator; the corpus is somebody else's checkout,
    /// named by `SENSEI_CORPUS`, which is why this is `#[ignore]`d.
    ///
    /// PARTITIONED BY REPOSITORY. An identity is scoped to a FOLDER because the
    /// scan indexes per repo; pooling asks a question production never asks,
    /// and on Java's corpus that read 16,561 collisions where the real figure
    /// was 409 purely because one repo vendored a copy of another.
    ///
    /// PHP HAS NO PARTIAL TYPE AND NO OVERLOAD, which removes both of the
    /// buckets that were CORRECT collisions in C# and Java. What is left here
    /// should be copies of one file, or a defect.
    ///
    ///     SENSEI_CORPUS=/path/to/php cargo test -p senseid --bin senseid \
    ///       php::tests::no_two_declarations_in_this_corpus_mint_one_identity \
    ///       -- --ignored --nocapture
    #[test]
    #[ignore]
    fn no_two_declarations_in_this_corpus_mint_one_identity() {
        use std::collections::{BTreeMap, BTreeSet};

        let Ok(root) = std::env::var("SENSEI_CORPUS") else {
            println!("SENSEI_CORPUS unset — nothing to read. See this test's docs.");
            return;
        };

        let mut by_repo: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
        for entry in walkdir::WalkDir::new(&root).into_iter().filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "php") {
                continue;
            }
            let shown = path.to_string_lossy().to_string();
            // Composer's install tree is a property of a toolchain rather than
            // of this reader — it is somebody else's source, vendored verbatim.
            if ["/vendor/", "/node_modules/", "/cache/"].iter().any(|skip| shown.contains(skip)) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(path) else { continue };
            let mut dir = path.parent();
            let mut repo = root.clone();
            while let Some(d) = dir {
                if d.join(".git").exists() {
                    repo = d.to_string_lossy().to_string();
                    break;
                }
                dir = d.parent();
            }
            by_repo.entry(repo).or_default().push((shown, text));
        }
        if by_repo.is_empty() {
            println!("no PHP under SENSEI_CORPUS — nothing to measure.");
            return;
        }

        let mut files = 0usize;
        let mut declarations = 0usize;
        let mut identities = 0usize;
        let mut namespaced = 0usize;
        let mut colliding: Vec<(String, BTreeSet<String>)> = Vec::new();
        let mut unreadable = 0usize;
        // Every file's text, so "are these two a COPY" is asked of the content
        // rather than inferred from the filename. Two files named `core.php` in
        // different directories share a basename and are not copies of each
        // other — reading the basename as proof is how a real collision gets
        // filed under the bucket labelled CORRECT.
        let mut text_of: BTreeMap<String, String> = BTreeMap::new();
        // Which files DECLARED a namespace. The sharp test: PSR-4 gives every
        // namespaced declaration a unique home, so a collision between two
        // namespaced files is a DEFECT IN THIS WALK, while one between files in
        // the global namespace is the language declining to discriminate. The
        // two must never be reported as one number.
        let mut namespaced_file: BTreeSet<String> = BTreeSet::new();

        for (repo, sources) in &by_repo {
            let package = repo.rsplit('/').next().unwrap_or("pkg").to_string();
            let read_all = |types: &TypeHomes| -> Vec<(String, FileFacts)> {
                sources
                    .iter()
                    .filter_map(|(path, text)| {
                        let source = Source { package: &package, module: "", path, text };
                        walk::read(&source, types).ok().map(|f| (path.clone(), f))
                    })
                    .collect()
            };
            let first = read_all(&TypeHomes::unknown());
            unreadable += sources.len() - first.len();
            let homes = TypeHomes::of(
                first.iter().flat_map(|(_, f)| f.symbols.iter().map(|s| (f.package.as_str(), s))),
            );
            let anchored = read_all(&homes);
            files += anchored.len();

            for (path, text) in sources {
                text_of.insert(path.clone(), text.clone());
            }
            let mut sites: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
            for (path, facts) in &anchored {
                if facts.package != package {
                    namespaced += 1;
                    namespaced_file.insert(path.clone());
                }
                for symbol in &facts.symbols {
                    declarations += 1;
                    sites.entry(symbol.fqn.as_str().to_string()).or_default().insert(format!(
                        "{:?} {} at {path}:{}",
                        symbol.kind, symbol.name, symbol.span.start_line
                    ));
                }
            }
            identities += sites.len();
            colliding.extend(sites.into_iter().filter(|(_, at)| at.len() > 1));
        }

        println!("\n── A7: one declaration, one identity (php) ──");
        println!("repositories {}", by_repo.len());
        println!("files        {files} ({unreadable} unreadable)");
        // A file with NO `namespace` line is in the global namespace, which is
        // legal and common in pre-PSR-4 code. Reported rather than asserted on,
        // because it decides how much of the corpus shares one package and so
        // how hard the identity question is.
        println!("namespaced   {namespaced}");
        println!("declarations {declarations}");
        println!("identities   {identities}");
        println!("COLLIDING    {}", colliding.len());

        // DECOMPOSE before concluding — reading a shape off the first few
        // samples has been wrong three times on this work.
        // PER BUCKET, not pooled. A pooled tally says 77 `Module` collisions
        // and leaves the reader to guess whether they are the correct kind or
        // the defect kind — which is exactly the read that has been wrong
        // before on this work.
        let mut by_kind: BTreeMap<(&str, String), usize> = BTreeMap::new();
        let mut one_file = 0usize;
        let mut between_namespaced = 0usize;
        let mut namespaced_examples: Vec<String> = Vec::new();
        let mut namespaced_by_ns: BTreeMap<String, usize> = BTreeMap::new();
        let mut copies = 0usize;
        let mut same_name_files = 0usize;
        let mut different_files = 0usize;
        let mut examples: Vec<String> = Vec::new();
        let mut one_file_examples: Vec<String> = Vec::new();
        let mut one_file_by_path: BTreeMap<String, usize> = BTreeMap::new();
        for (fqn, at) in &colliding {
            let kind = at
                .iter()
                .next()
                .and_then(|s| s.split_whitespace().next())
                .unwrap_or("?")
                .to_string();
            let paths: BTreeSet<&str> = at
                .iter()
                .filter_map(|s| s.rsplit_once(" at "))
                .filter_map(|(_, f)| f.rsplit_once(':'))
                .map(|(p, _)| p)
                .collect();
            if paths.len() > 1 && paths.iter().all(|p| namespaced_file.contains(*p)) {
                between_namespaced += 1;
                // GROUPED BY NAMESPACE, not sampled. Six capped examples is how
                // "they are all one shape" gets believed without being true, and
                // that read has already been wrong three times on this work.
                let ns = fqn.split('·').nth(1).unwrap_or("?").to_string();
                *namespaced_by_ns.entry(ns).or_default() += 1;
                if namespaced_examples.len() < 4 {
                    namespaced_examples.push(format!(
                        "{fqn}\n        {}",
                        paths.iter().take(2).cloned().collect::<Vec<_>>().join("\n        ")
                    ));
                }
            }
            if paths.len() <= 1 {
                one_file += 1;
                *by_kind.entry(("one file", kind)).or_default() += 1;
                if let Some(p) = paths.iter().next() {
                    *one_file_by_path.entry((*p).to_string()).or_default() += 1;
                }
                if one_file_examples.len() < 6 {
                    one_file_examples.push(format!(
                        "{fqn}\n        {}",
                        at.iter().take(3).cloned().collect::<Vec<_>>().join("\n        ")
                    ));
                }
                continue;
            }
            // A COPY is identical CONTENT. Several files holding the same
            // bytes declare the same things, and one identity for them is the
            // correct answer rather than a collision.
            let bodies: BTreeSet<&str> =
                paths.iter().filter_map(|p| text_of.get(*p)).map(String::as_str).collect();
            if bodies.len() == 1 && paths.len() > 1 {
                copies += 1;
                *by_kind.entry(("copies", kind)).or_default() += 1;
                continue;
            }
            let bases: BTreeSet<&str> =
                paths.iter().map(|p| p.rsplit('/').next().unwrap_or(p)).collect();
            if bases.len() == 1 {
                same_name_files += 1;
                *by_kind.entry(("same name, edited", kind)).or_default() += 1;
                continue;
            }
            different_files += 1;
            *by_kind.entry(("different files", kind)).or_default() += 1;
            if examples.len() < 8 {
                examples.push(format!(
                    "{fqn}\n        {}",
                    paths.iter().take(3).cloned().collect::<Vec<_>>().join("\n        ")
                ));
            }
        }
        for ((bucket, kind), n) in &by_kind {
            println!("  {bucket:<16} {kind:<14} {n}");
        }
        println!("  one file        {one_file}");
        println!(
            "  BETWEEN NAMESPACED FILES {between_namespaced} \
             (PSR-4 says unique, so this is a DEFECT unless the files are duplicated)"
        );
        let mut worst_ns: Vec<(&String, &usize)> = namespaced_by_ns.iter().collect();
        worst_ns.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
        println!("    across {} namespaces:", namespaced_by_ns.len());
        for (ns, n) in worst_ns.iter().take(12) {
            println!("      {n:>4}  {ns}");
        }
        for e in &namespaced_examples {
            println!("    {e}");
        }
        // WHICH FILES, not which six examples. A capped sample is how "it is
        // all one shape" gets believed without being true.
        let mut worst_files: Vec<(&String, &usize)> = one_file_by_path.iter().collect();
        worst_files.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
        println!("    across {} files, worst:", one_file_by_path.len());
        for (path, n) in worst_files.iter().take(6) {
            println!("      {n:>4}  {path}");
        }
        for e in &one_file_examples {
            println!("    {e}");
        }
        println!("  copies          {copies} (identical CONTENT — one identity is CORRECT)");
        println!("  same name, edited {same_name_files} (one file, diverged — a fork, not a copy)");
        println!("  different files {different_files} (a DEFECT, unless the files are copies)");
        for e in &examples {
            println!("    {e}");
        }
        // THE BOUND SITS AT THE MEASUREMENT, not above it. A ceiling with slack
        // silently absorbs a new defect until the slack runs out, which this
        // repository has already paid for once (A4's rust ceiling).
        //
        // The decomposition that justifies the number is on `A7_BOUND`.
        assert!(
            colliding.len() <= A7_BOUND,
            "{} colliding identities, was {A7_BOUND} over this corpus. Read the decomposition \
             above before moving this number: `copies` and the duplicated generated trees are \
             CORRECT, and a rise in `one file` or in a namespace other than grpc's is a defect \
             in the walk.",
            colliding.len()
        );
    }

    /// Every shape a PHP type expression takes, reduced to what it NAMES.
    ///
    /// MUTATION: split on `/` rather than `\` and a namespaced type mints the
    /// whole path as one segment, which no declaration equals.
    #[test]
    fn a_type_expression_names_its_own_type() {
        assert_eq!(seg("User"), "User");
        assert_eq!(seg("App\\Models\\User"), "User");
        assert_eq!(seg("\\Exception"), "Exception");
        assert_eq!(seg("?User"), "User");
        assert_eq!(seg("User|null"), "User");
        assert_eq!(seg("?App\\Models\\User"), "User");
    }

    #[test]
    fn a_type_expression_that_names_nothing_is_an_error() {
        assert!(PhpAdapter.type_segment("").is_err());
        assert!(PhpAdapter.type_segment("  ").is_err());
    }
}
