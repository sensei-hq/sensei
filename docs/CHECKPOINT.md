# Checkpoint

**Slice:** indexer v2 — languages, before the wipe + re-index (issue #130, phase `build`)

## Cut over (7)

`PRODUCTION_LANGUAGES = [Rust, TypeScript, Java, Python, CSharp, Php, C]`

| language | A7 | gate |
|---|---|---|
| Rust | 19 → **0** | `persist::…_rust_…` |
| TypeScript (+js/svelte/vue) | 511 → **0** | `persist::…_typescript_…` |
| Java | 409 → **14** (named) | `java::corpus::…` |
| Python | 72 → **0** | `python::tests::…` |
| C# | 452 → **392** (all buckets explained) | `csharp::tests::…` |
| PHP | **312**, `one file` = **0** | `php::tests::…` |
| C | **11**, `one file` = **0** | `c::tests::…` |

**~5,600 lines of v1 parser deleted.** C# and PHP were ADDITIVE — 19,404 + 2,251
files that had no language now have a graph.

PHP's 312 are all CROSS-FILE: 204 are two checked-in copies of one
protoc-generated tree in grpc, 108 are CakePHP 2.x global-namespace reuse (the
app class, the code-gen skeleton, the test fixture). The global ones are
deliberately unpatched — the ladder has no directory, so a path-derived identity
would trade them for thousands of dangling edges.

**Fixed on the way:** `language_for_ext_slug` consulted a second table beside the
registry and it had drifted — `.cs` resolved to `other`, so every C# node carried
the wrong `nodes.language`. The table is gone; the adapter answers for itself, and
`every_registered_extension_reports_its_own_adapters_language` pins it.

## Built, not flipped

**SQL — T-SQL half only.** `lang::sql` detects the dialect and dispatches;
`lang::sql::tsql` is this crate's own lexer + statement-head reader. Built
because nothing off the shelf can declare a stored procedure —
`tree-sitter-sequel`'s `grammar.js` says `// TODO: procedure`, and `sqlparser`'s
`MsSqlDialect` fails on the parenless `CREATE PROCEDURE @p int AS` form.

Measured over Ethico (9 repos, 2,419 files, 14 unreadable):

| | |
|---|---:|
| read as T-SQL | 2,154 |
| declaring an object | 1,458 (2,742 objects — 1,285 procedures) |
| declaring nothing, correctly | 695 |
| **missed** | **1** — dynamic SQL, the definition is inside an `sp_executesql` string |
| references | 43,737 (28,626 placed) |
| colliding | 544 — **0 within a file** |

The 544 are release folders (`4.2.1/` beside `4.3.0.2/`, one named
`DO NOT USE_4.1/`) — one procedure, several versions.

NOT FLIPPED: the Postgres half waits on
[dbd#19](https://github.com/sensei-hq/dbd/issues/19). v1's `sql.rs` keeps
producing, so nothing regresses.

Three rules this settled:
1. **Dialect is STATED or DETECTED** — `design.yaml` says `source.dialect`; everything else is scored, and a tie or a blank is `Unstated`, never a guess.
2. **`CREATE` declares, `DROP` refers, `ALTER` is BOTH** — the object kind decides. `ALTER PROCEDURE X AS <body>` carries the whole definition; `ALTER TABLE X ADD` does not.
3. **A qualified call is a hard edge; a bare one is a built-in** — T-SQL requires a scalar UDF to be schema-qualified and never qualifies a built-in, so dbd's soft/hard split is readable off the grammar here. Worth 4,628 function edges.

**Kotlin** (`34325771`) — 245 files, 0 unreadable, 3,038 declarations, A7 = 39. Two shapes block it:

1. **Android product flavours** — `src/{cityofdoral,ecuador,panama}/Color.kt` declare the same package, one compiled per build. rust's `cfg` problem via the *directory*; belongs to **placement**, not the walk.
2. **Anonymous object expressions** — `val M = object : Migration(5,6) { fun migrate }`. This one *is* the walk's: name the scope after the property, as TypeScript does.

**C narrowed on the way through.** v1's adapter claimed `.cpp`/`.hpp`/`.cc` and
read them line-by-line; `tree-sitter-c` parses C, so v2 claims `.c`/`.h` only and
**nothing** claims the C++ extensions. That absence must be total: a
`DetectionOnly` entry for an extension no v2 adapter claims is a PANIC, not a
skip. `c::walk::is_cpp` also refuses a `.h` holding C++ — 1,817 of 3,776 vendored
files, with 0 false positives on real C.

`CMakeManifestAdapter` landed with it (`project(name)` is the one place a C
project states its name). **158 of 165 C files already placed** through an
enclosing `pom.xml`/`Cargo.toml`/`composer.json`/`package.json`; the 7 that do
not have no manifest above them at all, and no CMakeLists.txt either — so the
adapter rescues nothing today and is there for the next C project.

**UTF-16 is read now.** `classifiers::decode_source` owns the decode and both
the scan gate and the parse path call it; the BOM is checked BEFORE the
null-byte test, because UTF-16LE ASCII is `X 00 X 00` and the null test was
calling all 754 of them `binary_content`. Worth 377 more files, +329 stored
procedures, +7,560 references — and it is why the T-SQL numbers above rose.

**`DbdManifestAdapter` reads `design.yaml`** — `project.name` names the package,
extensions are dependencies, and `source.dialect` STATES the dialect (5 of 9
manifests do; the other 4 predate the key and get `None`, not an assumption).
Before it, no manifest above a `.ddl` file named a package — the workspace
`Cargo.toml` has no `[package]` — so every dbd DDL file was "not indexed".

The dialect is **not yet threaded** to the SQL reader: `Placement`/`Source` carry
package and module only, and the consumer (a Postgres reader over dbd's
`parse_sql`, dbd#19) does not exist yet. Detection covers it meanwhile —
605 PostgreSQL / 5 Unstated / **0 wrong** over 610 dbd DDL files.

Fixed on the way: a backtick was a MySQL marker, and it is also what people
write around words in comments. 2,132 of them across 178 files of sensei's own
Postgres DDL made **204 files (a third) detect as MySQL**. Fourth instance of
one shape — `"CXX".contains("C")`, `go` in `logo`, `VIEW` in `ViewedBy`.

## Remaining

| language | files | grammar (ABI-checked, pinned) |
|---|---:|---|
| SQL — Postgres half | ~2,600 | dbd-core, pending [dbd#19](https://github.com/sensei-hq/dbd/issues/19) |
| Swift | 2 | `tree-sitter-swift =0.6.0` |

Then: deploy → `TRUNCATE sensei.nodes, sensei.edges CASCADE` → re-index → acceptance → wire `Stated::Gone`.

## Next command

```
cargo test -p senseid --bin senseid 2>&1 | tail -5   # NO — see below
cargo test -p senseid --bin senseid                  # yes
```
Never pipe a test or build through `tail` — a pipe reports the pipe's exit status.

## Hard-won rules for a new adapter

1. **A container is a PATH, not a leaf** — every language has needed this.
2. **A body does not declare members of what encloses it** — a local is not a field.
3. **Check the grammar's ABI first**: `grep LANGUAGE_VERSION <crate>/src/parser.c`. The runtime accepts 13–14; `tree-sitter-c-sharp` 0.23.5, and php/c/swift 0.24, emit 15 and fail at *runtime*, on every file.
4. **Trust the tree, not `node-types.json`** — Kotlin's manifest advertises fields the tree never builds.
5. **Write fixtures the way the language is written.** A one-line fixture has now misled me three times.
6. **A copy is identical CONTENT, not an identical filename.** Reading the basename as proof filed 216 real PHP collisions under a bucket labelled CORRECT.
7. **Measure placement with the right precedence.** A first count put 53 C files
   under "Makefile only" and unplaceable; `placement_on_disk` never looks at a
   Makefile, so the real figure was 7. The wrong number came from putting build
   SCRIPTS in the same priority list as manifests.
8. **Two lints can conflict, and the type is the way out.** clippy wanted
   `unwrap_or_default()` where the no-fabrication guard forbids it. Carrying
   BOTH facts — the module string and whether the source wrote one — satisfied
   each and was clearer than either.
9. **A flip is two registries**: `indexer::lang` (who parses) and `crate::languages` (what language is this). PHP needed a `DetectionOnly` entry or `.php` carried no language at all.

## Known-broken / not deployed

- Grammars pinned for ABI: `tree-sitter-c-sharp =0.23.1`, `tree-sitter-php =0.23.11`, `tree-sitter-c =0.23.4`, `tree-sitter-swift =0.6.0`.
- This repo has no Java/Python/C#/Kotlin/PHP/Swift — acceptance cannot measure them. Use `SENSEI_CORPUS`, pointed at a checkout tree that holds them.
- A real finding in Ethico: `Campaign_RiskAssesment.cs` beside `Campaign_RiskAssessment.cs`, same class, one left behind after a filename typo fix.
- The daemon predates every cutover. The wipe is required — reconcile reads `props->'claims'`, which 0 of 467,707 nodes carry.
