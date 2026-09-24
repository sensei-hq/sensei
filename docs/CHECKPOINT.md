# Checkpoint

**Slice:** indexer v2 — languages, before the wipe + re-index (issue #130, phase `build`)

## Cut over (5)

`PRODUCTION_LANGUAGES = [Rust, TypeScript, Java, Python, CSharp]`

| language | A7 | gate |
|---|---|---|
| Rust | 19 → **0** | `persist::…_rust_…` |
| TypeScript (+js/svelte/vue) | 511 → **0** | `persist::…_typescript_…` |
| Java | 409 → **14** (named) | `java::corpus::…` |
| Python | 72 → **0** | `python::tests::…` |
| C# | 452 → **392** (all buckets explained) | `csharp::tests::…` |

**~5,600 lines of v1 parser deleted.** C# was additive — 19,404 files that had no language now have a graph.

## Built, not flipped

**Kotlin** (`34325771`) — 245 files, 0 unreadable, 3,038 declarations, A7 = 39. Two shapes block it:

1. **Android product flavours** — `src/{cityofdoral,ecuador,panama}/Color.kt` declare the same package, one compiled per build. rust's `cfg` problem via the *directory*; belongs to **placement**, not the walk.
2. **Anonymous object expressions** — `val M = object : Migration(5,6) { fun migrate }`. This one *is* the walk's: name the scope after the property, as TypeScript does.

## Remaining

| language | files | grammar (ABI-checked) |
|---|---:|---|
| SQL (+ddl) | 7,435 | `tree-sitter-sequel` — v1's is regex-based, so a rewrite |
| PHP | 2,251 | `tree-sitter-php` — additive, nothing has handled it |
| C (+h) | 160 | `tree-sitter-c` |
| Swift | 2 | `tree-sitter-swift` |

Then: deploy → `TRUNCATE sensei.nodes, sensei.edges CASCADE` → re-index → acceptance → wire `Stated::Gone`.

## Next command

```
SENSEI_CORPUS=/Users/Jerry/Work/Alert/repos/base-app-android \
  cargo test -p senseid --bin senseid kotlin::tests -- --ignored --nocapture
```
Never pipe a test or build through `tail` — a pipe reports the pipe's exit status.

## Hard-won rules for a new adapter

1. **A container is a PATH, not a leaf** — every language has needed this.
2. **A body does not declare members of what encloses it** — a local is not a field.
3. **Check the grammar's ABI first**: `grep LANGUAGE_VERSION <crate>/src/parser.c`. The runtime accepts 13–14; `tree-sitter-c-sharp` 0.23.5 emits 15 and fails at *runtime*, on every file.
4. **Trust the tree, not `node-types.json`** — Kotlin's manifest advertises fields the tree never builds.
5. **Write fixtures the way the language is written.** A one-line fixture has now misled me three times.

## Known-broken / not deployed

- `tree-sitter-c-sharp` pinned `=0.23.1` (ABI).
- This repo has no Java/Python/C#/Kotlin/Swift — acceptance cannot measure them. Use `SENSEI_CORPUS`.
- A real finding in Ethico: `Campaign_RiskAssesment.cs` beside `Campaign_RiskAssessment.cs`, same class, one left behind after a filename typo fix.
- The daemon predates every cutover. The wipe is required — reconcile reads `props->'claims'`, which 0 of 467,707 nodes carry.
