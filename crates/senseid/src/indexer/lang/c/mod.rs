//! C — identity rules and the grammar the ladder climbs.
//!
//! The walk is in [`walk`]; this file owns what a C name MEANS, which is R7's
//! split.
//!
//! # LINKAGE is C's namespace, and it is the whole design
//!
//! Every other language here states where a declaration lives: a `package`
//! line, a `namespace` block, a directory a manifest roots. C states nothing —
//! and it does not have to, because it says something better. A declaration has
//! **external linkage** unless it is `static`, and external linkage means
//! exactly one thing across the entire link unit.
//!
//! So a C fqn reads the storage class:
//!
//! - **extern** (the default) → `<module>` is EMPTY, so `void parse(void)` in
//!   `src/parse.c` mints an identity of just the language, the package, the
//!   name and the reach. A call to `parse()` in any other file of the package
//!   mints that same string with nothing written to bring it into scope. That
//!   is not a convenience; it is what the linker does.
//! - **`static`** → `<module>` is the FILE. `static void helper(void)` in
//!   `src/parse.c` and another in `src/emit.c` are two functions, and C says so.
//!
//! # A PROTOTYPE IS NOT A DECLARATION
//!
//! `void parse(void);` in `parse.h` does not define anything — it promises that
//! a definition exists elsewhere. If the header minted a symbol, every function
//! in a C codebase would have two declarations at one identity and A7 would be
//! a header count rather than a defect count.
//!
//! So only a `function_definition` — the form with a BODY — mints a function.
//! The prototype's return and parameter types are still read as use sites,
//! because those are real references the header makes.
//!
//! The same rule applies to `extern int errno;`: a declaration with `extern`
//! written on it is a promise, not a definition.
//!
//! # TAGS ARE THEIR OWN NAMESPACE
//!
//! C 6.2.3 gives struct, union and enum TAGS a namespace of their own, separate
//! from ordinary identifiers — which is the whole reason
//! `typedef struct node { .. } node;` is legal and idiomatic. `struct node` and
//! `node` are two names, and you cannot write the tag without its keyword.
//!
//! So a tag's NAME SEGMENT carries the keyword: `struct node`, `enum color`.
//! That is how C spells it, and it is what keeps the tag and the typedef of it
//! apart. Reading them as one name put 14 collisions in the `one file` bucket
//! over pljava alone — every one of them this idiom.
//!
//! # Not everything with a name has LINKAGE
//!
//! A typedef, a tag, an enum constant and a macro have NO LINKAGE: each is
//! private to the translation unit that declares it. Only functions and
//! file-scope variables have the external linkage described above.
//!
//! A translation unit is not a file, though — it is a `.c` plus everything it
//! includes. So the EXTENSION answers it:
//!
//! - declared in a `.h` → every file that includes the header shares it, which
//!   is unit scope;
//! - declared in a `.c` → private to that one file.
//!
//! Without the rule, every tree-sitter `parser.c` in a grammar collection
//! declares `enum { sym_identifier, .. }` and they all collapse onto one
//! identity: 189 collisions in one small corpus, none of them a defect in C.
//!
//! # An enum constant is an ITEM, not a member
//!
//! `enum Color { RED, GREEN };` puts `RED` in the ORDINARY namespace at file
//! scope — `RED` is written bare, never `Color.RED`. Every other language here
//! reaches an enum constant through its enum, and C is the one that does not,
//! so its constants are minted as items beside the enum rather than under it.
//!
//! # C++ IS NOT C
//!
//! v1's C adapter claimed `.cpp`, `.hpp` and `.cc` and read them with a
//! line-based scanner. This one claims `.c` and `.h` only, because
//! `tree-sitter-c` parses C: handed a class or a template it recovers into
//! `ERROR` nodes, and facts read out of a recovered parse are invented ones.
//! The C++ extensions are left to the "unknown file type" branch, which gives
//! them a file node and no symbols — see the note in `crate::languages`.

use std::sync::LazyLock;

use super::{LanguageAdapter, Source, TypeHomes};
use crate::indexer::facts::{FileFacts, Fqn, Language};
use crate::indexer::fqn::{self, Form, FqnError, Reach, Segment};
use crate::indexer::resolve::Grammar;

mod walk;

/// The names a C file has with nothing but the standard headers, as
/// `(name, package, path)`.
///
/// The package is `libc` — the C standard library, which is a real thing that
/// ships these and is how every other tool names it. A translation unit reaches
/// them through an `#include <...>`, and the include is recorded as an import
/// too; this table is what places the ones a file uses without one, plus the
/// types (`size_t`, `FILE`) that are spelled bare everywhere.
const PRELUDE: &[(&str, &str, &str)] = &[
    // types
    ("size_t", "libc", "size_t"),
    ("ssize_t", "libc", "ssize_t"),
    ("ptrdiff_t", "libc", "ptrdiff_t"),
    ("FILE", "libc", "FILE"),
    ("va_list", "libc", "va_list"),
    ("time_t", "libc", "time_t"),
    ("uint8_t", "libc", "uint8_t"),
    ("uint16_t", "libc", "uint16_t"),
    ("uint32_t", "libc", "uint32_t"),
    ("uint64_t", "libc", "uint64_t"),
    ("int8_t", "libc", "int8_t"),
    ("int16_t", "libc", "int16_t"),
    ("int32_t", "libc", "int32_t"),
    ("int64_t", "libc", "int64_t"),
    ("intptr_t", "libc", "intptr_t"),
    ("uintptr_t", "libc", "uintptr_t"),
    ("bool", "libc", "bool"),
    // memory
    ("malloc", "libc", "malloc"),
    ("calloc", "libc", "calloc"),
    ("realloc", "libc", "realloc"),
    ("free", "libc", "free"),
    ("memcpy", "libc", "memcpy"),
    ("memmove", "libc", "memmove"),
    ("memset", "libc", "memset"),
    ("memcmp", "libc", "memcmp"),
    // strings
    ("strlen", "libc", "strlen"),
    ("strcmp", "libc", "strcmp"),
    ("strncmp", "libc", "strncmp"),
    ("strcpy", "libc", "strcpy"),
    ("strncpy", "libc", "strncpy"),
    ("strcat", "libc", "strcat"),
    ("strdup", "libc", "strdup"),
    ("strchr", "libc", "strchr"),
    ("strstr", "libc", "strstr"),
    ("strtol", "libc", "strtol"),
    ("strtod", "libc", "strtod"),
    ("snprintf", "libc", "snprintf"),
    ("sprintf", "libc", "sprintf"),
    ("sscanf", "libc", "sscanf"),
    // stdio
    ("printf", "libc", "printf"),
    ("fprintf", "libc", "fprintf"),
    ("fopen", "libc", "fopen"),
    ("fclose", "libc", "fclose"),
    ("fread", "libc", "fread"),
    ("fwrite", "libc", "fwrite"),
    ("fseek", "libc", "fseek"),
    ("ftell", "libc", "ftell"),
    ("fgets", "libc", "fgets"),
    ("puts", "libc", "puts"),
    ("putchar", "libc", "putchar"),
    // process / misc
    ("exit", "libc", "exit"),
    ("abort", "libc", "abort"),
    ("assert", "libc", "assert"),
    ("qsort", "libc", "qsort"),
    ("bsearch", "libc", "bsearch"),
    ("abs", "libc", "abs"),
    ("atoi", "libc", "atoi"),
    ("getenv", "libc", "getenv"),
    ("errno", "libc", "errno"),
    ("NULL", "libc", "NULL"),
];

/// EMPTY, and that is a fact about C rather than an omission.
///
/// Every other language's plumbing list names members that say nothing about a
/// receiver's type — `toString`, `equals`, `__init__`. C has no receivers and no
/// members outside a struct, so there is no call shape for the list to exclude.
const PLUMBING: &[&str] = &[];

/// C has NO spelling that tells a type from a value.
///
/// Every other language here can be asked: Java lints a type to `UpperCamel`,
/// Rust to `CamelCase`, PHP to `StudlyCaps`. C's own standard library writes
/// `size_t` and `FILE` in one namespace and `printf` in another, and a project
/// may spell a struct `node_t` or `Node` with equal idiomatic claim.
///
/// So this is the predicate that never fires, which
/// [`Grammar::names_a_type`]'s own documentation names as the honest answer
/// where a language does not state it. Guessing would mint a member identity
/// for what is actually a module path, and R4 ranks a dangling edge above a
/// wrong one.
///
/// It costs nothing here, because C's paths have no segments for the predicate
/// to split: a C name is one identifier, and the only two-part spelling
/// (`s.field`) is read positionally by the walk rather than by the ladder.
fn names_a_type(_segment: &str) -> bool {
    false
}

pub static GRAMMAR: LazyLock<Grammar> = LazyLock::new(|| Grammar {
    language: Language::C,
    // A C NAME HAS NO SEPARATOR. There is no `a::b`, no `a.b.c` — an identifier
    // is one segment, always. `/` is stated for the MODULE only, which is a
    // file path when a declaration is `static`.
    path_separator: "/",
    module_separator: "/",
    // No `crate::`, no `./`. An `#include` spells a filesystem path and is
    // resolved as one, which is `relative_to_directory` below rather than a
    // root word.
    roots: &[],
    module_segment: crate::indexer::lang::common::already_a_module_segment,
    relative_depth_prefix: None,
    names_the_binding: None,
    // `#include` brings in EVERY name the header declares, which is what a glob
    // is — but it is spelled as a path rather than a `*`, so there is no token
    // for this to list. The walk emits `Binding::Glob` directly.
    wildcard: None,
    // FALSE. A C reference is a bare identifier, so a multi-segment path never
    // reaches the ladder and the rung this enables could only ever misfire.
    paths_name_packages: false,
    // An `#include "x.h"` is resolved against the DIRECTORY of the including
    // file, which is the same rule JavaScript's `./x` uses.
    relative_to_directory: true,
    names_a_type,
    prelude: PRELUDE,
    plumbing: PLUMBING,
});

/// The type a C type expression NAMES.
pub(super) fn type_segment(raw: &str) -> Result<String, FqnError> {
    let mut bare = raw.trim();
    // A QUALIFIER is decoration on a USE of the type and comes off. A TAG DOES
    // NOT — see `tags_are_their_own_namespace`, and the two must be one loop
    // because C lets them interleave: stripping in two passes read
    // `const struct node *` as `struct node` in one order and as `node` in the
    // other, and only one of those is a name.
    const DECORATION: &[&str] =
        &["const ", "volatile ", "restrict ", "static ", "extern ", "register "];
    loop {
        let before = bare;
        for word in DECORATION {
            if let Some(rest) = bare.strip_prefix(word) {
                bare = rest.trim();
            }
        }
        if bare == before {
            break;
        }
    }
    // A POINTER names the type it points at. No array case: C puts the
    // brackets on the DECLARATOR, never on the type, so `char buf[16]` reaches
    // here as `char` and there is nothing to strip.
    let bare = bare.trim_end_matches(['*', ' ', '\t']);
    if bare.is_empty() {
        return Err(FqnError::EmptySegment { segment: Segment::Type });
    }
    // WHITESPACE NORMALISED, because a tagged name is two words and the source
    // may put any amount of space between them. `struct   node` and
    // `struct node` are one name.
    Ok(bare.split_whitespace().collect::<Vec<_>>().join(" "))
}

/// C, from `.c` and `.h`.
pub struct CAdapter;

impl LanguageAdapter for CAdapter {
    fn language(&self) -> Language {
        Language::C
    }

    fn name(&self) -> &'static str {
        "c"
    }

    /// `.c` and `.h` ONLY. See this module's header for why `.cpp`, `.hpp` and
    /// `.cc` are not here.
    fn extensions(&self) -> &'static [&'static str] {
        &[".c", ".h"]
    }

    fn grammar(&self) -> &'static Grammar {
        &GRAMMAR
    }

    fn read(&self, source: &Source<'_>, types: &TypeHomes) -> Result<FileFacts, super::ReadError> {
        walk::read(source, types)
    }

    fn file_fqn(&self, package: &str, module: &str, path: &str) -> Result<Fqn, FqnError> {
        let stem = path.rsplit('/').next().unwrap_or(path);
        let stem = stem.rsplit_once('.').map_or(stem, |(head, _)| head);
        // THE MODULE IS CARRIED, unlike Java's and PHP's, and that is the
        // header/implementation pair talking. `src/parse.c` and `include/parse.h`
        // have the same stem and are two files; with the module dropped they
        // would be one node holding both, and a project with parallel `src/` and
        // `include/` trees is the ordinary C layout rather than a corner.
        fqn::define(&Form::Item {
            lang: Language::C,
            package,
            module,
            name: stem,
            reach: Reach::Mod,
        })
    }

    /// The file's own path — WITH its extension — which is the scope a
    /// `static` declaration lives in.
    ///
    /// NOT dropped to a stem: see [`CAdapter::file_fqn`]. And NOT reduced by
    /// `mod`/`index` rules — C has no file name with a special meaning, so the
    /// path is already the module.
    ///
    /// **THE EXTENSION STAYS**, and dropping it was a measured defect: `src/x.c`
    /// and `src/x.h` are the ordinary header/implementation pair and both
    /// reduce to `src/x`, so their file modules minted one identity and their
    /// `static`s shared a scope. 299 colliding file modules over one corpus.
    /// The two really are different files, and a header's `static` really is a
    /// different object from the `.c`'s — each includer gets its own copy.
    fn module_path(&self, file: &str, package_root: &str) -> String {
        file.strip_prefix(package_root).unwrap_or(file).trim_start_matches('/').to_string()
    }

    fn type_segment(&self, raw: &str) -> Result<String, FqnError> {
        type_segment(raw)
    }

    /// TRUE for the `static` half and false for the rest, and the honest answer
    /// for a file is the stronger of the two: moving `parse.c` re-mints every
    /// `static` it declares, because their module IS the path.
    ///
    /// An external-linkage declaration does not move: its identity carries no
    /// module at all, so `void parse(void)` names the same thing wherever the
    /// file sits. That is the linker's view, and the whole basis of this
    /// adapter.
    fn rename_remints_identity(&self, from: &str, to: &str, package_root: &str) -> bool {
        self.module_path(from, package_root) != self.module_path(to, package_root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexer::facts::{
        Binding, ImportOrigin, RefKind, Resolution, SymbolKind, Visibility,
    };

    /// The A7 ratchet. A RATCHET, not a budget: it sits AT the measured value
    /// so a single new collision fails the gate.
    ///
    /// SET FOR PLJAVA — 129 files of hand-written C, which is the corpus that
    /// answers whether this walk reads C. Three were measured:
    ///
    /// | corpus | files | declarations | colliding | `one file` |
    /// |---|---:|---:|---:|---:|
    /// | pljava | 129 | 1,906 | 11 | **0** |
    /// | codegraph (generated) | 22 | 2,532 | 161 | **0** |
    /// | grpc + envoy (vendored) | 1,959 of 3,776 | 77,394 | 13,834 | 42 |
    ///
    /// **`one file` is ZERO on both corpora of real C.** That is the bucket the
    /// walk alone controls, and reaching it took five fixes, each measured:
    ///
    ///   32 → 22  tags are their own namespace, so `struct node` and a typedef
    ///            `node` are two names (C 6.2.3). 14 collisions over pljava,
    ///            every one the `typedef struct X { .. } X;` idiom.
    ///   22 → 18  a conditionally redefined macro is a callable plus one arm
    ///            per branch — the split rust uses for `cfg` and C# for `#if`.
    ///   18 → 12  an anonymous aggregate is named by what BINDS it, so two
    ///            typedef'd anonymous structs in one header stop sharing a
    ///            `time` field. The rule TypeScript needed for a callback.
    ///   12 → 11  a definition beats a declaration: a forward declaration
    ///            through a typedef'd function TYPE has a bare declarator, and
    ///            only the definition below it says it was a function.
    ///   11 → 11  a tentative definition (`static T x;` twice, no initialiser)
    ///            is ONE object by C 6.9.2.
    ///
    /// Separately, refusing C++ headers named `.h` took the vendored corpus
    /// from 606 intra-file collisions to 42 and its `different files Function`
    /// bucket from 1,907 to 90, with NO effect on either real-C corpus — 0
    /// unreadable in both, which is the check that the refusal does not
    /// false-positive.
    ///
    /// THE 11 THAT REMAIN ARE ALL CROSS-FILE, and they are one shape:
    /// `JNIEXPORT jboolean JNICALL` on its own line. `JNIEXPORT` and `JNICALL`
    /// are macros, and tree-sitter reads the line as a declaration of a
    /// variable called `jboolean` before parsing the real function below it. A
    /// walk that does not run the preprocessor cannot know better, and
    /// guessing — "this name is used as a type elsewhere in the file, so the
    /// declaration is bogus" — would be a rule invented rather than read. It is
    /// named here so a rise is readable, the way C#'s Syncfusion residue is.
    const A7_BOUND: usize = 11;

    /// Read one C source. `src/parse.c` throughout, so the module a `static`
    /// takes is `src/parse` and a reader can see it in the assertions.
    fn read(text: &str) -> FileFacts {
        // `src/lexer.c`, and NOT `src/parse.c`: the file MODULE is a symbol
        // named after the stem, so a fixture declaring `parse` in `parse.c`
        // makes two symbols of one name and the assertions read the wrong one.
        // A fixture that cannot tell them apart tests nothing.
        let source = Source { package: "pkg", module: "src/lexer.c", path: "src/lexer.c", text };
        walk::read(&source, &TypeHomes::unknown()).expect("the fixture parses")
    }

    /// The same source, read as a HEADER — which changes where a declaration
    /// with no linkage lives.
    fn read_header(text: &str) -> FileFacts {
        let source = Source { package: "pkg", module: "src/lexer.h", path: "src/lexer.h", text };
        walk::read(&source, &TypeHomes::unknown()).expect("the fixture parses")
    }

    fn fqn_of(facts: &FileFacts, name: &str) -> String {
        facts
            .symbols
            .iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("`{name}` was not declared; got {:?}", names(facts)))
            .fqn
            .as_str()
            .to_string()
    }

    fn names(facts: &FileFacts) -> Vec<&str> {
        facts.symbols.iter().map(|s| s.name.as_str()).collect()
    }

    /// **The whole design, in one test.** `static` is the FILE; everything else
    /// is the link unit.
    ///
    /// The empty module on `parse` is what lets a call in another file of the
    /// package meet it with nothing written — which is what the linker does.
    /// The `src/parse` module on `helper` is what keeps two files' helpers apart.
    ///
    /// MUTATION: return `self.module` for both arms of `Walk::declare` — every
    /// extern in the codebase becomes file-scoped and no cross-file call
    /// resolves. Return `""` for both and every `static helper` in the corpus
    /// collides with every other.
    #[test]
    fn linkage_decides_the_module_and_nothing_else_does() {
        let facts = read(
            "void parse(void) { }\n\
             static void helper(void) { }\n\
             int total;\n\
             static int count;\n",
        );
        // NO EMPTY SEGMENT: `fqn::define` drops a module that is not there, so
        // an external-linkage item is `<lang>·<package>·<name>·<reach>` and a
        // file-scoped one carries the path between them. The two shapes have
        // different segment counts, which is why they can never be confused.
        assert_eq!(fqn_of(&facts, "parse"), "c·pkg·parse·item");
        assert_eq!(fqn_of(&facts, "helper"), "c·pkg·src/lexer.c·helper·item");
        assert_eq!(fqn_of(&facts, "total"), "c·pkg·total·item");
        assert_eq!(fqn_of(&facts, "count"), "c·pkg·src/lexer.c·count·item");
    }

    /// A PROTOTYPE IS NOT A DECLARATION, and neither is an `extern`.
    ///
    /// `parse.h` promising `void parse(void);` and `parse.c` defining it must be
    /// ONE node. If the header minted one too, every function in every C
    /// codebase would carry two declarations at one identity.
    ///
    /// MUTATION: drop the `declares_a_function` early return in
    /// `Walk::declaration` — A7 becomes a count of prototypes.
    #[test]
    fn a_prototype_and_an_extern_are_promises_rather_than_declarations() {
        let facts = read(
            "void parse(struct node *n);\n\
             extern int errno;\n\
             void parse(struct node *n) { }\n",
        );
        // ONE `parse`, from the definition on line 3.
        assert_eq!(names(&facts).iter().filter(|n| **n == "parse").count(), 1);
        assert!(!names(&facts).contains(&"errno"), "an extern promises: {:?}", names(&facts));
        // The prototype's parameter type is still a real use site the header
        // makes — the promise is about the NAME, not about what it mentions.
        let node_uses = facts
            .references
            .iter()
            .filter(|r| r.kind == RefKind::TypeUse)
            .filter(|r| {
                matches!(&r.target,
                Resolution::Unresolved { evidence, .. } if evidence.name == "struct node")
            })
            .count();
        assert!(node_uses >= 2, "both the prototype and the definition name it");
    }

    /// An enum constant is an ITEM — the one place C differs from every other
    /// language here.
    ///
    /// `RED` is written bare at file scope. Minting it under `Color` would
    /// produce `c·pkg··Color·RED·field`, which no C use site can compose.
    ///
    /// MUTATION: declare the constants inside the enum's scope — every
    /// reference to a bare constant dangles.
    #[test]
    fn an_enum_constant_is_an_item_because_c_writes_it_bare() {
        let facts = read_header("enum color { RED, GREEN };\n");
        assert_eq!(fqn_of(&facts, "enum color"), "c·pkg·enum color·item");
        assert_eq!(fqn_of(&facts, "RED"), "c·pkg·RED·item");
        assert_eq!(fqn_of(&facts, "GREEN"), "c·pkg·GREEN·item");
    }

    /// A struct's FIELDS are members of it, and they are reached at field reach.
    ///
    /// MUTATION: mint the field at `Reach::Item` — the declaration and the
    /// `p->x` that reads it differ in exactly one segment and can never meet.
    /// Java paid 22,316 declarations and zero field edges for that.
    #[test]
    fn a_struct_owns_its_fields_at_field_reach() {
        let facts = read_header("struct point { int x; int y; };\n");
        assert_eq!(fqn_of(&facts, "struct point"), "c·pkg·struct point·item");
        assert_eq!(fqn_of(&facts, "x"), "c·pkg·struct point·x·field");
        assert_eq!(fqn_of(&facts, "y"), "c·pkg·struct point·y·field");
    }

    /// The declarator is a NEST, and the name is at the bottom of it.
    ///
    /// MUTATION: read the declarator's text instead of unwrapping — the fqn
    /// carries `*handlers[4])(int`, which is not a name.
    #[test]
    fn the_name_is_unwrapped_from_however_many_layers_the_type_has() {
        let facts = read(
            "char *make(void) { return 0; }\n\
             static int *const *deep;\n\
             struct box { char *(*handlers[4])(int); };\n",
        );
        assert_eq!(fqn_of(&facts, "make"), "c·pkg·make·item");
        assert_eq!(fqn_of(&facts, "deep"), "c·pkg·src/lexer.c·deep·item");
        assert_eq!(fqn_of(&facts, "handlers"), "c·pkg·src/lexer.c·struct box·handlers·field");
    }

    /// A call to a `static` resolves in the WALK, because the ladder has no way
    /// to know which file it was declared in.
    ///
    /// MUTATION: drop the `file_scoped` lookup in `Walk::refer_to` — the call
    /// goes up unplaced, the ladder mints the empty module that external
    /// linkage means, and it lands on nothing.
    #[test]
    fn a_call_to_a_file_scoped_function_is_resolved_by_the_walk() {
        let facts = read(
            "static int helper(void) { return 1; }\n\
             int parse(void) { return helper(); }\n",
        );
        let call = facts
            .references
            .iter()
            .find(|r| r.kind == RefKind::Calls)
            .expect("the call is a reference");
        let Resolution::Resolved { fqn, .. } = &call.target else {
            panic!("a static is in this file, so the walk answers: {:?}", call.target)
        };
        assert_eq!(fqn.as_str(), "c·pkg·src/lexer.c·helper·item");
    }

    /// A call to an EXTERNAL name goes up unplaced, so the ladder mints the
    /// empty module and meets the definition in whatever file holds it.
    #[test]
    fn a_call_to_an_external_name_is_left_for_the_ladder() {
        let facts = read("int parse(void) { return emit(); }\n");
        let call = facts
            .references
            .iter()
            .find(|r| r.kind == RefKind::Calls)
            .expect("the call is a reference");
        let Resolution::Unresolved { evidence, .. } = &call.target else {
            panic!("this file does not declare `emit`: {:?}", call.target)
        };
        assert_eq!(evidence.name, "emit");
    }

    /// A LOCAL IS NOT A NODE, but it still types the receiver that reads it.
    ///
    /// MUTATION: drop the `scope.in_body` early return — every local in every
    /// body becomes a row nothing reads, which is the cost TypeScript measured
    /// at 5,074 declarations.
    #[test]
    fn a_local_is_not_a_node_but_it_still_types_a_field_read() {
        let facts = read_header(
            "struct point { int x; };\n\
             int parse(void) { struct point p; return p.x; }\n",
        );
        assert!(!names(&facts).contains(&"p"), "a local is not a node: {:?}", names(&facts));
        let read_x = facts
            .references
            .iter()
            .find(|r| r.kind == RefKind::Reads)
            .expect("the field read is a reference");
        let Resolution::Resolved { fqn, .. } = &read_x.target else {
            panic!("`p` states its type, so the read resolves: {:?}", read_x.target)
        };
        assert_eq!(fqn.as_str(), "c·pkg·struct point·x·field");
    }

    /// `#include` binds a GLOB, because textual inclusion brings in every name
    /// the header declares and nothing in the including file says which.
    #[test]
    fn an_include_binds_a_glob_and_says_which_side_of_the_boundary_it_is_on() {
        let facts = read("#include <stdio.h>\n#include \"parse.h\"\n");
        assert_eq!(facts.imports.len(), 2);
        let system = &facts.imports[0];
        assert_eq!(system.path, "stdio.h");
        assert!(matches!(system.binds, Binding::Glob));
        assert_eq!(system.origin, ImportOrigin::External { package: "libc".to_string() });
        let local = &facts.imports[1];
        assert_eq!(local.path, "parse.h");
        assert_eq!(local.origin, ImportOrigin::Local);
    }

    /// `static` is C's only visibility, and it is the same fact the module
    /// segment carries.
    #[test]
    fn static_is_the_only_thing_c_hides() {
        let facts = read("void open(void) { }\nstatic void shut(void) { }\n");
        let vis = |n: &str| {
            facts.symbols.iter().find(|s| s.name == n).expect("declared").visibility.clone()
        };
        assert_eq!(vis("open"), Visibility::Public);
        assert_eq!(vis("shut"), Visibility::Private);
    }

    /// A `typedef` of an ANONYMOUS struct names it once, on the typedef.
    ///
    /// MUTATION: invent a name for the anonymous struct — a second node appears
    /// that no use site can reach, and its fields hang off it instead of being
    /// reachable through the alias.
    #[test]
    fn a_typedef_of_an_anonymous_struct_is_one_declaration() {
        let facts = read_header("typedef struct { int x; } point_t;\n");
        assert_eq!(fqn_of(&facts, "point_t"), "c·pkg·point_t·item");
        let kinds: Vec<SymbolKind> =
            facts.symbols.iter().filter(|s| s.name == "point_t").map(|s| s.kind).collect();
        assert_eq!(kinds, vec![SymbolKind::TypeAlias]);
        // The field is not lost — it lands under the scope that holds the
        // anonymous body.
        assert!(names(&facts).contains(&"x"), "{:?}", names(&facts));
    }

    /// A `#define` is unit-scoped, and its replacement list is TOKENS.
    ///
    /// MUTATION: walk the macro body — `#define LOG(x) fprintf(stderr, x)`
    /// records a call to `fprintf` that this file never makes.
    #[test]
    fn a_macro_is_a_declaration_and_its_body_is_not_walked() {
        let facts = read_header("#define MAX 10\n#define LOG(x) fprintf(stderr, x)\n");
        assert_eq!(fqn_of(&facts, "MAX"), "c·pkg·MAX·item");
        assert_eq!(fqn_of(&facts, "LOG"), "c·pkg·LOG·item");
        assert!(
            !facts.references.iter().any(|r| r.kind == RefKind::Calls),
            "a replacement list is tokens, not a call"
        );
    }

    /// **A TAG AND A TYPEDEF OF IT ARE TWO NAMES.**
    ///
    /// `typedef struct node { .. } node;` is legal and idiomatic precisely
    /// because C 6.2.3 gives tags a namespace of their own. Reading them as one
    /// name put 14 collisions in the `one file` bucket over pljava alone, and
    /// every one of them was this idiom.
    ///
    /// MUTATION: strip the tag in `type_segment`, or drop the parent check in
    /// `type_names_under` — the struct and the alias collapse onto one identity
    /// and one of the two declarations is lost.
    #[test]
    fn a_tag_and_a_typedef_of_it_are_two_names() {
        let facts = read_header("typedef struct node { int id; } node;\n");
        assert_eq!(fqn_of(&facts, "struct node"), "c·pkg·struct node·item");
        assert_eq!(fqn_of(&facts, "node"), "c·pkg·node·item");
        assert_ne!(fqn_of(&facts, "struct node"), fqn_of(&facts, "node"));

        // And a USE of the tag mints the TAGGED name, so it meets the tag
        // rather than the alias.
        let facts = read_header("struct node *head;\n");
        let use_site = facts
            .references
            .iter()
            .find(|r| r.kind == RefKind::TypeUse)
            .expect("the declared type is a use site");
        let Resolution::Unresolved { evidence, .. } = &use_site.target else {
            panic!("this file declares no `struct node`: {:?}", use_site.target)
        };
        assert_eq!(evidence.name, "struct node");
    }

    /// **A DECLARATION WITH NO LINKAGE IS SCOPED BY THE EXTENSION.**
    ///
    /// A typedef, a tag, an enum constant and a macro are private to the
    /// translation unit. A header's unit is every file that includes it; a
    /// `.c`'s is itself.
    ///
    /// MUTATION: return `Linkage::Unit` from `no_linkage` unconditionally —
    /// every tree-sitter `parser.c` in a grammar collection declares
    /// `enum { sym_identifier, .. }`, and they all collapse onto one identity.
    /// Measured at 189 collisions over one small corpus.
    #[test]
    fn a_no_linkage_declaration_is_private_to_a_c_file_and_shared_by_a_header() {
        let src = "#define TAG 1\ntypedef int id_t;\nenum state { IDLE };\n";
        let header = read_header(src);
        assert_eq!(fqn_of(&header, "TAG"), "c·pkg·TAG·item");
        assert_eq!(fqn_of(&header, "id_t"), "c·pkg·id_t·item");
        assert_eq!(fqn_of(&header, "IDLE"), "c·pkg·IDLE·item");

        let implementation = read(src);
        assert_eq!(fqn_of(&implementation, "TAG"), "c·pkg·src/lexer.c·TAG·item");
        assert_eq!(fqn_of(&implementation, "id_t"), "c·pkg·src/lexer.c·id_t·item");
        assert_eq!(fqn_of(&implementation, "IDLE"), "c·pkg·src/lexer.c·IDLE·item");

        // A FUNCTION is NOT scoped this way — it has real linkage, and only
        // `static` narrows it. The extension says nothing about it.
        assert_eq!(fqn_of(&read_header("void parse(void) { }\n"), "parse"), "c·pkg·parse·item");
        assert_eq!(fqn_of(&read("void parse(void) { }\n"), "parse"), "c·pkg·parse·item");
    }

    /// No two declarations in one file mint one identity — A7, at file grain.
    #[test]
    fn no_two_declarations_in_one_file_mint_one_identity() {
        use std::collections::BTreeMap;

        let facts = read(
            "#include \"parse.h\"\n\
             #define MAX 10\n\
             struct node { int id; struct node *next; };\n\
             typedef struct node node;\n\
             enum state { IDLE, BUSY };\n\
             static int count;\n\
             int total;\n\
             static void helper(node_t *n) { }\n\
             void parse(node_t *n) { helper(n); }\n",
        );
        let mut sites: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for symbol in &facts.symbols {
            sites.entry(symbol.fqn.as_str()).or_default().push(&symbol.name);
        }
        let colliding: Vec<_> = sites.iter().filter(|(_, at)| at.len() > 1).collect();
        assert!(colliding.is_empty(), "{colliding:?}");
    }

    fn seg(raw: &str) -> String {
        CAdapter.type_segment(raw).expect("the fixture names a type")
    }

    /// Every shape a C type expression takes, reduced to what it NAMES.
    ///
    /// MUTATION: stop stripping the `struct` tag — `struct node` mints a
    /// segment no `struct_specifier` name field can equal, so every use of a
    /// tagged struct dangles.
    #[test]
    fn a_type_expression_names_its_own_type() {
        assert_eq!(seg("int"), "int");
        assert_eq!(seg("const char *"), "char");
        assert_eq!(seg("unsigned long"), "unsigned long");
        // THE TAG STAYS. `struct node` is the name; `node` is a different name
        // in a different namespace, and very often a typedef OF this one.
        assert_eq!(seg("struct node"), "struct node");
        assert_eq!(seg("struct node **"), "struct node");
        assert_eq!(seg("enum color"), "enum color");
        assert_eq!(seg("union value"), "union value");
        assert_eq!(seg("const struct node *"), "struct node");
        assert_eq!(seg("struct   node"), "struct node", "whitespace is not part of a name");
    }

    #[test]
    fn a_type_expression_that_names_nothing_is_an_error() {
        assert!(CAdapter.type_segment("").is_err());
        assert!(CAdapter.type_segment("  *  ").is_err());
        assert!(CAdapter.type_segment("*").is_err());
        // A DECORATION WORD ALONE is returned as itself rather than erroring,
        // and that is deliberate. The reduction strips a decoration only when
        // something follows it, so a project type genuinely named `signed` (or
        // a macro that expands to one) survives. The grammar's `type` field
        // always names a type, so a bare keyword never reaches here from a
        // walk — this pins that the stripper is not greedy.
        assert_eq!(seg("const"), "const");
    }

    /// **A7 for C: no two declarations mint one identity.**
    ///
    /// This repository holds no C, so acceptance cannot measure it and the
    /// answer would be an empty denominator; the corpus is an external checkout
    /// named by `SENSEI_CORPUS`, which is why this is `#[ignore]`d.
    ///
    /// PARTITIONED BY REPOSITORY, like every other corpus gate here — an
    /// identity is scoped to a folder because the scan indexes per repo.
    ///
    /// THE SHAPE TO WATCH IS DIFFERENT FROM THE OTHERS. C's risk is not
    /// overloading (it has none) or partial types (none) — it is the
    /// HEADER/IMPLEMENTATION pair. A prototype that minted a symbol would
    /// collide with its definition once per function in the codebase, so a
    /// large `different files` bucket full of function kinds is that defect and
    /// not a property of the corpus.
    ///
    ///     SENSEI_CORPUS=/path/to/c cargo test -p senseid --bin senseid \
    ///       lang::c::tests::no_two_declarations_in_this_corpus_mint_one_identity \
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
            if path.extension().is_none_or(|e| e != "c" && e != "h") {
                continue;
            }
            let shown = path.to_string_lossy().to_string();
            // Build output and vendored dependency trees are a property of a
            // toolchain rather than of this reader.
            if ["/build/", "/node_modules/", "/third_party/", "/vendor/"]
                .iter()
                .any(|skip| shown.contains(skip))
            {
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
            println!("no C under SENSEI_CORPUS — nothing to measure.");
            return;
        }

        let mut files = 0usize;
        let mut declarations = 0usize;
        let mut identities = 0usize;
        let mut unreadable = 0usize;
        let mut colliding: Vec<(String, BTreeSet<String>)> = Vec::new();
        let mut text_of: BTreeMap<String, String> = BTreeMap::new();

        for (repo, sources) in &by_repo {
            let package = repo.rsplit('/').next().unwrap_or("pkg").to_string();
            let mut read_all = Vec::new();
            for (path, text) in sources {
                text_of.insert(path.clone(), text.clone());
                let rel = path.strip_prefix(repo).unwrap_or(path).trim_start_matches('/');
                // THE ADAPTER'S OWN RULE for the module, not a second spelling
                // of it — the walk mints a `static` under exactly this.
                let module = CAdapter.module_path(rel, "");
                let source = Source { package: &package, module: &module, path: rel, text };
                match walk::read(&source, &TypeHomes::unknown()) {
                    Ok(facts) => read_all.push((path.clone(), facts)),
                    Err(_) => unreadable += 1,
                }
            }
            files += read_all.len();

            let mut sites: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
            for (path, facts) in &read_all {
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

        println!("\n── A7: one declaration, one identity (c) ──");
        println!("repositories {}", by_repo.len());
        println!("files        {files} ({unreadable} unreadable)");
        println!("declarations {declarations}");
        println!("identities   {identities}");
        println!("COLLIDING    {}", colliding.len());

        let mut by_kind: BTreeMap<(&str, String), usize> = BTreeMap::new();
        let mut one_file = 0usize;
        let mut copies = 0usize;
        let mut different_files = 0usize;
        let mut one_file_examples: Vec<String> = Vec::new();
        let mut examples: Vec<String> = Vec::new();
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
            // A COPY is identical CONTENT. Asking the basename instead is how a
            // real collision gets filed under the bucket labelled CORRECT —
            // measured on PHP, where it moved 216 of 287.
            let bodies: BTreeSet<&str> =
                paths.iter().filter_map(|p| text_of.get(*p)).map(String::as_str).collect();
            if bodies.len() == 1 {
                copies += 1;
                *by_kind.entry(("copies", kind)).or_default() += 1;
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
        let mut worst: Vec<(&String, &usize)> = one_file_by_path.iter().collect();
        worst.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
        println!("    across {} files, worst:", one_file_by_path.len());
        for (path, n) in worst.iter().take(6) {
            println!("      {n:>4}  {path}");
        }
        for e in &one_file_examples {
            println!("    {e}");
        }
        println!("  copies          {copies} (identical CONTENT — one identity is CORRECT)");
        println!("  different files {different_files}");
        for e in &examples {
            println!("    {e}");
        }
        // THE BOUND SITS AT THE MEASUREMENT. See `A7_BOUND`.
        assert!(
            colliding.len() <= A7_BOUND,
            "{} colliding identities, was {A7_BOUND} over this corpus. Read the decomposition \
             above first: a rise in `different files` with a Function kind is the \
             header/implementation defect, and a rise in `one file` is the walk.",
            colliding.len()
        );
    }

    /// The module is the PATH, so a header and its implementation are two
    /// files, and moving one re-mints what it declares `static`.
    ///
    /// MUTATION: return the stem — `src/parse.c` and `include/parse.h` collapse
    /// onto one module and every `static` in the pair collides.
    #[test]
    fn the_module_is_the_path_so_a_header_and_its_impl_are_two_files() {
        let m = |f: &str| CAdapter.module_path(f, "/repo");
        assert_eq!(m("/repo/src/parse.c"), "src/parse.c");
        assert_eq!(m("/repo/include/parse.h"), "include/parse.h");
        assert_ne!(m("/repo/src/parse.c"), m("/repo/include/parse.h"));
        // THE PAIR IN ONE DIRECTORY, which is the ordinary C layout and the
        // case that made the extension load-bearing.
        assert_ne!(m("/repo/src/parse.c"), m("/repo/src/parse.h"));
        assert!(CAdapter.rename_remints_identity("/repo/src/a.c", "/repo/src/b.c", "/repo"));
        // Same module, different extension is still a move.
        assert!(CAdapter.rename_remints_identity("/repo/a.c", "/repo/sub/a.c", "/repo"));
    }
}
