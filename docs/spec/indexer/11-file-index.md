# Stage 11 — one file in, nodes and edges out

Whole-system spec: `docs/design/indexer.md` §2 (identity), §3 (fact types),
R1, R4, R6, R7, R9. Supersedes the two-pass composition in
`crates/senseid/src/indexer/index.rs`. Rust first; the other adapters follow
one at a time against §10.

## 1. Purpose

**Index ONE file with no knowledge of any other file, and emit the nodes and
edges it implies.** No barrier, no second parse, no repo-scoped table. The
output is language-independent — fqn strings — so the persistence layer that
consumes it is shared and the AST reading is the only per-language part.

This replaces a design where each file was parsed TWICE around a repo-wide
type table. That table answered two questions, and neither belonged to it:

| table | question | who actually owns it |
|---|---|---|
| `TypeHomes` | what do I call it | **the file** — `use crate::a::Widget` says so |
| `declared_members` | does it exist | **the persistence layer** — `nodes_unique_fqn` |

MEASURED, over the sensei repo: of every member reference the barrier
resolves, 41.0% are types the file declares, 38.0% types it imports by name,
17.6% reachable through a wildcard import, 1.6% spelled inline — **98.1%
stated by the file itself**. Pass two cost 14.8s of a 59.5s stage.

## 2. Inputs and outputs

    index_file(input: FileInput) -> FileIndex        PURE

    FileInput  { repo, path, mode, package, module, text }
    FileIndex  { mode, repo, file, language, package, module, nodes[], edges[] }

`language` is NOT an input. The driver resolves it from the extension via
`lang::adapter_for_ext` and reports what it used — a caller that could name a
language would be a caller that could name the wrong one.

PURE, and it must stay pure: `text` rather than a path to open, so the whole
suite runs on string literals and `lang::tests::no_adapter_reads_the_filesystem`
keeps it that way.

### 2.1 Mode

    mode ∈ { New | Update | Delete }

One code path for an initial scan and an incremental one; only the mode
differs. An initial scan is every file at `New`, in whatever order the walk
yields them.

| mode | what the walk does | what persistence does (stage 12) |
|---|---|---|
| `New` | reads the file | insert/update nodes, then edges |
| `Update` | reads the file | **delete first**: rows for this file whose fqn is absent from `nodes[]`; then insert/update |
| `Delete` | nothing; `nodes` and `edges` are EMPTY | delete every row whose file is this file |

`Delete` returns an empty result rather than skipping the call, so the
persistence layer has one input shape and no special case.

## 3. Requirements

### The driver (language-agnostic)

- **S1.** The driver NEVER matches on a language. It resolves an adapter from
  the extension, calls it, and stamps the result. `lang::tests::nothing_outside
  _this_module_dispatches_on_a_language_by_hand` is the existing guard and it
  covers this module.
- **S2.** An extension no adapter claims yields `nodes: [], edges: []` and a
  stated reason — never a partial read and never an error. Most of a repo is
  not source: 942 of 2,309 files here.
- **S3.** The driver adds no facts of its own. Everything in `nodes[]` and
  `edges[]` comes from the adapter, the file's own path, or the `package` and
  `module` it was handed.

### The adapter (per language, Rust first)

- **S4** (R1). ONE parse. The adapter is called once and returns everything.
- **S5** (R7). The adapter receives NO cross-file table. Its signature loses
  the `TypeHomes` parameter entirely:

      fn read(&self, source: &Source<'_>) -> Result<FileFacts, ReadError>

  Deleting the parameter is the point: with it present, a walk can reach for
  repo-wide knowledge and the barrier grows back.

- **S6.** A type's module is read from THIS FILE, in this order: its own
  declarations, then its imports, then an inline qualified path. Only a
  package-rooted import answers — `use serde::Serialize` names a library we do
  not open (R5), and a `self`/`super` path is relative to a module the walk is
  not told.

- **S7** (R4). Every identity the walk mints is tagged with how it knows:

  | tag | meaning | may become an edge |
  |---|---|---|
  | `Named(fqn)` | this file's text establishes it | **yes**, unconditionally |
  | `Candidate(fqn)` | a name match only | only against known declarations |

  This is the split that removes the existence check. `Named` needs no
  corroboration because the file proves it; `Candidate` still cannot become an
  edge on its own, which is what
  `a_bare_name_matching_another_files_declaration_is_not_proof_of_anything`
  protects.

### The fqn — the part that must change

- **S8. THE TRAIT STAYS IN THE FQN, AND THE CALL SITE LEARNS TO WRITE IT.**

  `<Box2 as Draw>::draw` and `<Box2 as Paint>::draw` are two methods and must
  be two identities. The declaration side already does this —
  `Form::TraitMember` mints `Report·Draw·draw`. The defect is on the USE side,
  which mints `Report·draw` and drops the discriminator, and every workaround
  downstream (`SuppliedMembers`, the collapsed spelling, the barrier that
  builds it) exists to paper over that one missing segment.

  **The trait IS in the AST at a qualified call, and the walk does not look for
  it.** `bracketed_type` and `qualified_type` appear twice in the whole Rust
  walk. Three call shapes, three answers:

  | written | trait available | mints |
  |---|---|---|
  | `<Box2 as Draw>::draw(&b)` | YES — `qualified_type` | `Box2·Draw·draw` |
  | `Draw::draw(&b)` | YES — the path head, and it is a trait in scope | `Box2·Draw·draw` |
  | `b.draw()` | NO — method syntax names no trait | `Box2·draw` |

  The first two are S8's work and are pure AST reading. The third is a fact
  about Rust: method-call syntax writes no trait, and `rustc` itself resolves
  it by looking at which in-scope trait supplies the name.

- **S8b.** For method-call syntax the use site mints the UNQUALIFIED
  `Box2·draw`, and the link to `Box2·Draw·draw` is made by stage 12 through
  the `TraitImpl` edge ALREADY IN THE GRAPH — `impl Draw for Box2` emits
  `TraitImpl: Draw → Box2` from the declaring file. That is a lookup over
  persisted edges, not a barrier: it needs no second parse and no repo-scoped
  table held in memory, and it is exactly the healing the stub model does
  everywhere else.

  Where two traits in scope supply one name, `TraitImpl` gives two answers and
  the edge stays unlinked with `AmbiguousCandidates` — which is also what
  `rustc` does (`E0034`).

- **S9.** A reference to a symbol this file does not declare is a node with
  `status: "referenced"` and NOTHING ELSE — no kind, no span, no visibility.
  Stage 12 inserts it only if absent, so a partial row can never overwrite a
  real declaration.

## 4. The output

### 4.1 Node

    fqn            the key. composite unique per (repo, fqn, language)
    kind           SymbolKind — null when status is "referenced"
    name           the trailing identifier
    status         "declared" | "referenced"
    visibility     Public | Crate | Restricted | Private
    declared_type  the type the source STATES, or null
    docstring      the doc comment, or null
    span           [start_line, start_col, end_line, end_col]
    params         [{ name, position, type }]

### 4.2 Edge

    kind      RelationKind | RefKind
              Extends Implements TraitImpl Mixin Decorates Owns Contains
              Calls Reads Writes Constructs TypeUse MacroInvokes Imports
    from      fqn of the symbol the use site sits in
    to        fqn, or null when unlinked
    status    "linked" | "unlinked"
    via       Rung, when linked — DeclaredHere ThroughAnImport DeclaredByItsType
              ThroughAGlob RootedInThisPackage FullyQualifiedExternal InThePrelude
    reason    Reason, when unlinked
    name      what the source wrote
    reach     Item | Field | Macro | Mod
    saw       the evidence — Receiver, ImportInScope, UnplacedType,
              BoundToTheResultOf, Candidate, Named
    at        [line, col]

### 4.3 Where `lost` lives

**`lost` is NOT a field of this stage, and that is deliberate.** It means
"a declaration exists and an edge that was meant to reach it is missing",
which needs the whole corpus — a single file cannot know it.

What this stage emits instead is the raw material, per edge:

- `status: "unlinked"` with a `reason` — the edge was meant and is not there
- `saw` — the identity the walk considered, so the miss is actionable

`lost` is then a **query** over the persisted graph, not an indexer output:

    lost_exact   a node with no inbound edge, whose EXACT fqn appears in
                 some unlinked edge's `saw`
    lost_by_name a node with no inbound edge, whose bare name appears in
                 some unlinked edge at that node's own reach

Both columns are reported, never summed into one number: `lost_by_name`
moves whenever a narrowing is added, and reading that as resolver progress is
a mistake this repo has already made — about 79% of one sweep's TypeScript
reduction and 56% of its Rust one was the definition moving.

## 5. Worked output — one real file

Generated by running the current walk, not hand-written. Input:

```rust
use crate::shape::Box2;
use crate::draw::Draw;

/// A report over shapes.
pub struct Report {
    pub total: u32,
    seen: Vec<Box2>,
}

impl Report {
    pub fn new() -> Self { Report { total: 0, seen: Vec::new() } }
    pub fn add(&mut self, b: &Box2) -> u32 {
        self.total += b.area();
        self.total
    }
}

impl Draw for Report {
    fn draw(&self) -> u32 { self.total }
}
```

```json
{
  "mode": "new",
  "repo": "/w/demo",
  "file": "crates/demo/src/report.rs",
  "language": "rust",
  "package": "demo",
  "module": "report",
  "nodes": [
    { "fqn": "rust·demo·report·mod",
      "kind": "Module", "name": "report", "status": "declared",
      "visibility": "Private", "declared_type": null, "docstring": null,
      "span": [1,1,20,2], "params": [] },
    { "fqn": "rust·demo·report·Report·item",
      "kind": "Struct", "name": "Report", "status": "declared",
      "visibility": "Public", "declared_type": null,
      "docstring": "A report over shapes.", "span": [5,0,8,1], "params": [] },
    { "fqn": "rust·demo·report·Report·total·field",
      "kind": "Field", "name": "total", "status": "declared",
      "visibility": "Public", "declared_type": "u32", "span": [6,0,6,14] },
    { "fqn": "rust·demo·report·Report·seen·field",
      "kind": "Field", "name": "seen", "status": "declared",
      "visibility": "Private", "declared_type": "Vec<Box2>", "span": [7,0,7,15] },
    { "fqn": "rust·demo·report·Report·new·item",
      "kind": "Method", "name": "new", "status": "declared",
      "visibility": "Public", "declared_type": "Self", "span": [11,0,11,62],
      "params": [] },
    { "fqn": "rust·demo·report·Report·add·item",
      "kind": "Method", "name": "add", "status": "declared",
      "visibility": "Public", "declared_type": "u32", "span": [12,0,15,1],
      "params": [ {"name":"self","position":0,"type":null},
                  {"name":"b","position":1,"type":"&Box2"} ] },

    { "fqn": "rust·demo·report·Report·Draw·draw·item",
      "kind": "Method", "name": "draw", "status": "declared",
      "visibility": "Private", "declared_type": "u32", "span": [19,0,19,36],
      "params": [ {"name":"self","position":0,"type":null} ] },

    { "fqn": "lib·std·vec::Vec",              "status": "referenced" },
    { "fqn": "lib·std·vec::Vec::new",         "status": "referenced" },
    { "fqn": "rust·demo·shape·Box2·item",     "status": "referenced" },
    { "fqn": "rust·demo·shape·Box2·area·item","status": "referenced" },
    { "fqn": "rust·demo·draw·Draw·item",      "status": "referenced" }
  ],
  "edges": [
    { "kind": "Contains", "from": "rust·demo·report·mod",
      "to": "rust·demo·report·Report·item",
      "status": "linked", "via": "DeclaredHere", "at": [5,0] },
    { "kind": "Owns", "from": "rust·demo·report·Report·item",
      "to": "rust·demo·report·Report·total·field",
      "status": "linked", "via": "DeclaredHere", "at": [6,0] },
    { "kind": "Owns", "from": "rust·demo·report·Report·item",
      "to": "rust·demo·report·Report·seen·field",
      "status": "linked", "via": "DeclaredHere", "at": [7,0] },
    { "kind": "Owns", "from": "rust·demo·report·Report·item",
      "to": "rust·demo·report·Report·new·item",
      "status": "linked", "via": "DeclaredHere", "at": [11,0] },
    { "kind": "Owns", "from": "rust·demo·report·Report·item",
      "to": "rust·demo·report·Report·add·item",
      "status": "linked", "via": "DeclaredHere", "at": [12,0] },
    { "kind": "Owns", "from": "rust·demo·report·Report·item",
      "to": "rust·demo·report·Report·Draw·draw·item",
      "status": "linked", "via": "DeclaredHere", "at": [19,0] },
    { "kind": "TraitImpl", "from": "rust·demo·draw·Draw·item",
      "to": "rust·demo·report·Report·item",
      "status": "linked", "via": "DeclaredHere", "at": [18,0] },

    { "kind": "TypeUse", "from": "rust·demo·report·Report·seen·field",
      "to": "lib·std·vec::Vec",
      "status": "linked", "via": "InThePrelude", "at": [7,6] },
    { "kind": "TypeUse", "from": "rust·demo·report·Report·seen·field",
      "to": "rust·demo·shape·Box2·item",
      "status": "linked", "via": "ThroughAnImport", "at": [7,10] },
    { "kind": "Constructs", "from": "rust·demo·report·Report·new·item",
      "to": "rust·demo·report·Report·item",
      "status": "linked", "via": "DeclaredHere", "at": [11,23] },
    { "kind": "Calls", "from": "rust·demo·report·Report·new·item",
      "to": "lib·std·vec::Vec::new",
      "status": "linked", "via": "InThePrelude", "at": [11,48] },
    { "kind": "TypeUse", "from": "rust·demo·report·Report·add·item",
      "to": "rust·demo·shape·Box2·item",
      "status": "linked", "via": "ThroughAnImport", "at": [12,26] },
    { "kind": "Writes", "from": "rust·demo·report·Report·add·item",
      "to": "rust·demo·report·Report·total·field",
      "status": "linked", "via": "DeclaredHere", "at": [13,0] },

    { "kind": "Calls", "from": "rust·demo·report·Report·add·item",
      "to": "rust·demo·shape·Box2·area·item",
      "status": "linked", "via": "NamedByThisFile", "at": [13,14] },

    { "kind": "Reads", "from": "rust·demo·report·Report·add·item",
      "to": "rust·demo·report·Report·total·field",
      "status": "linked", "via": "DeclaredHere", "at": [14,0] },
    { "kind": "Reads", "from": "rust·demo·report·Report·Draw·draw·item",
      "to": "rust·demo·report·Report·total·field",
      "status": "linked", "via": "DeclaredHere", "at": [19,24] }
  ]
}
```

**The three lines that are the whole change**, against what the code emits today:

1. `Report·Draw·draw·item` keeps its trait — unchanged. What changes is the
   USE side: a qualified `Draw::draw(&r)` must mint the same string, which the
   walk cannot do today because it never reads `qualified_type`. S8.
2. `Box2·area·item` linked `NamedByThisFile` — today `unlinked`,
   `NoImportInScope`, carrying `Candidate("rust·demo·shape·Box2·area·item")`.
   The walk had the right string and the gate refused it. S7.
3. `Box2·area·item` present as a `referenced` node — the stub `a.rs` promotes.

## 6. TDD order

Each step is RED first, and each RED must be an assertion failure, not a
compile error, so the message says what is wrong.

| # | test | mutation that must break it |
|---|---|---|
| 1 | `a_qualified_trait_call_mints_the_declarations_identity` — `<Box2 as Draw>::draw` and `Draw::draw` both reach `Box2·Draw·draw` | drop the trait from the qualified path |
| 2 | `two_traits_supplying_one_name_mint_two_identities` — `Box2·Draw·draw` ≠ `Box2·Paint·draw` | drop the trait segment |
| 3 | `one_file_reaches_a_member_of_a_type_it_imported` — single file, no table | tag it `Candidate` instead of `Named` |
| 4 | `a_name_match_alone_still_cannot_become_an_edge` | make `Candidate` unconditional |
| 5 | `read_takes_no_cross_file_table` — the signature | re-add the parameter |
| 6 | `the_driver_never_matches_on_a_language` — source guard | add a `Language::Rust =>` arm |
| 7 | `an_unclaimed_extension_yields_empty_with_a_reason` | return an error instead |
| 8 | `delete_mode_yields_no_nodes_and_no_edges` | read the file anyway |
| 9 | `the_order_files_arrive_in_changes_nothing` — index A,B and B,A | accumulate state between files |
| 10 | `every_def_and_class_line_yields_one_node` — over a real corpus | double-walk a subtree |

Step 10 is the conservation property, and it is not optional: twelve passing
tests and four mutation probes were blind to a 3× double-count in the python
walk, and only a corpus total caught it. Assert a property derived a
DIFFERENT way than the code derives its answer.

## 7. Failure modes

| input | this stage does |
|---|---|
| an extension no adapter claims | empty result with a reason. Not an error |
| a file the grammar rejects | `ReadError`, and the caller counts it. Never a partial node set |
| a type named nowhere in the file | the edge is `unlinked` with its evidence. 1.9% of members here — an imported factory's return type |
| a wildcard import in scope | the target module's export list is a stage-12 lookup, not a barrier |
| two traits supplying one name on one type | TWO nodes, correctly. A method call naming it stays unlinked with `AmbiguousCandidates`, which is what `rustc` does (`E0034`) |
| a path outside the package root | no module. The filesystem must not enter an identity |

## 8. Verification

| check | mutation |
|---|---|
| a qualified trait call mints the declaration's own string | ignore `qualified_type` |
| `Box as Draw` and `Box as Paint` stay two identities | drop the trait segment |
| a single file resolves a member of an imported type | drop the import rung from `home_of` |
| a bare name match alone does not | make `Candidate` unconditional |
| `read` cannot see another file | re-add the table parameter |
| the driver names no language | add a match arm |
| corpus: every `def`/`class` line is one node | double-walk |
| corpus: no module path claimed by two files | drop the extension from the module rule |
| corpus: no import resolves to the importing file, except `use super::*` | assert `is_empty()` instead of the shape |

## 9. Watch out

**The trait segment is not the defect — not reading it at the call site is.**
`Box as Draw` and `Box as Paint` are genuinely two methods and an fqn that
merged them would be wrong. The AST carries the trait at a qualified call and
the walk looks for it twice in 1,700 lines. Fix the reader, not the identity.

**`Named` is not a licence.** It says the file's text establishes the
identity. A receiver whose type the source never states is still
`ReceiverTypeUnknown`, and a bare name that merely matches something is still
`Candidate`. R4 is unchanged: a wrong edge remains worse than a missing one.

**Do not let `read` keep a cross-file parameter "for now".** It is the one
door the barrier can come back through.

**Assert shapes, not counts, for corpus detectors.** The self-edge detector
found 309 legitimate `use super::*` hits; asserting emptiness would have meant
deleting the detector to make it pass, and it is what caught python's relative
imports resolving to the importing file.

## 10. Definition of done

- A qualified trait call mints the declaration's own identity; `Box as Draw`
  and `Box as Paint` remain distinct
- `SuppliedMembers` has no caller — method-syntax linking goes through the
  persisted `TraitImpl` edge — verified with `rg --no-ignore -g '!target'` and
  a confirmed non-truncated count
- `LanguageAdapter::read` takes no cross-file table
- `index_file` is one parse, and the driver names no language
- All three modes return the same shape
- Indexing A then B equals B then A, over the corpus
- The corpus conservation property holds
- Resolved-reference count is **≥** the two-pass design's 96,304 over this
  repo — measured, with the before/after recorded
- Stage 12 (persistence) is not started until this passes

## 11. Then the other languages

Rust first and alone (D4). Each next adapter is the same three changes —
drop the table parameter, read the type's home from the file, tag `Named` vs
`Candidate` — and is done when §10 passes for it. Order by corpus size:
typescript/javascript/svelte, python, java. The trigger to move on is the
current language passing, not the calendar.
