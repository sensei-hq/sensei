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

## Remaining

| language | files | grammar (ABI-checked, pinned) |
|---|---:|---|
| SQL (+ddl) | 7,435 | `tree-sitter-sequel 0.3` — v1's is regex-based, so a rewrite |
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
8. **A flip is two registries**: `indexer::lang` (who parses) and `crate::languages` (what language is this). PHP needed a `DetectionOnly` entry or `.php` carried no language at all.

## Known-broken / not deployed

- Grammars pinned for ABI: `tree-sitter-c-sharp =0.23.1`, `tree-sitter-php =0.23.11`, `tree-sitter-c =0.23.4`, `tree-sitter-swift =0.6.0`.
- This repo has no Java/Python/C#/Kotlin/PHP/Swift — acceptance cannot measure them. Use `SENSEI_CORPUS`, pointed at a checkout tree that holds them.
- A real finding in Ethico: `Campaign_RiskAssesment.cs` beside `Campaign_RiskAssessment.cs`, same class, one left behind after a filename typo fix.
- The daemon predates every cutover. The wipe is required — reconcile reads `props->'claims'`, which 0 of 467,707 nodes carry.
