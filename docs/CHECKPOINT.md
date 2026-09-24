# Checkpoint

**Slice:** indexer v2 — languages, before the wipe + re-index (issue #130, phase `build`)

## Cut over

`PRODUCTION_LANGUAGES = [Rust, TypeScript, Java, Python]`.

| language | A7 descent | gate |
|---|---|---|
| Rust | 19 → 0 | `persist::…_rust_…` |
| TypeScript (+js/svelte/vue) | **511 → 0** | `persist::…_typescript_…` |
| Java | **409 → 14** (bounded, named) | `java::corpus::…` (`SENSEI_CORPUS`) |
| Python | **72 → 0** | `python::tests::…` (`SENSEI_CORPUS`) |

**~5,600 lines of v1 parser deleted.** `languages/jvm.rs` stays — only Kotlin uses it now.

## Built, not yet flipped

**C#** (`37548d23`) — 1,654 lines, built on Java's shape. The language neither indexer ever had.

Measured over Ethico's 8 repos: **8,647 files, 0 unreadable, 129,498 declarations, 452 colliding (0.35%)** — 230 copies of one file, 110 cross-file (almost all `partial` types), 112 residue.

Both recurring rules were built in from the first line, with tests written before the corpus was read. **Both passed first run** — the first time that's happened.

## The two rules that recur

Check any new adapter against both **before** measuring:

1. **A container is a PATH, not a leaf** — `Outer.Inner`.
2. **A body does not declare members of what encloses it** — a local is not a field.

## Remaining

1. **C#**: record `partial` on the symbol → sharpen the gate; add A2/A3 measurements; then add to `PRODUCTION_LANGUAGES` (additive — v1 never had a C# parser, so this flip cannot regress a language).
2. SQL 7,435 · Kotlin 247 (takes `jvm.rs`) · C 160 · Swift 2 · PHP 2,251.
3. Deploy → `TRUNCATE sensei.nodes, sensei.edges CASCADE` → re-index → acceptance → wire `Stated::Gone`.

## Next command

```
SENSEI_CORPUS=/Users/Jerry/Work/Ethico cargo test -p senseid --bin senseid \
  csharp::tests -- --ignored --nocapture
```
Never pipe a test or build through `tail` — a pipe reports the pipe's exit status.

## Open questions

- C#'s **base list** gives a base class and an interface one syntax. Interfaces get `Extends`; classes get `Implements`. Resolvable from the *target's* declaration, which is scan-level — left to a later pass.
- C# `partial` is unrecorded, so the gate's cross-file bucket mixes the language's own answer with real faults.

## Known-broken / not deployed

- **`tree-sitter-c-sharp` is pinned `=0.23.1`** — 0.23.5 emits ABI 15 against a 13–14 runtime and fails at `set_language` at *runtime*, on every `.cs` file.
- This repo has no Java/Python/C#/Kotlin/Swift, so acceptance cannot measure them. Use `SENSEI_CORPUS`; an empty denominator is not a pass.
- The daemon predates every cutover. The wipe is required — reconcile reads `props->'claims'`, which 0 of 467,707 nodes carry.
