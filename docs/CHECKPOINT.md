# Checkpoint

**Slice:** indexer v2 — languages, before the wipe + re-index (issue #130, phase `build`)

## Cut over

`PRODUCTION_LANGUAGES = [Rust, TypeScript, Java, Python]` — ten-plus extensions, all produced by `crate::indexer::lang`.

| language | A7 descent | gate |
|---|---|---|
| Rust | 19 → 0 (earlier) | `persist::…_rust_…` |
| TypeScript (+js/svelte/vue) | **511 → 0** | `persist::…_typescript_…` |
| Java | **409 → 14** (bounded, named) | `java::corpus::…` (`SENSEI_CORPUS`) |
| Python | **72 → 0** | `python::tests::…` (`SENSEI_CORPUS`) |

**~5,600 lines of v1 parser deleted** across the three cutovers. `languages/jvm.rs` stays — Kotlin shares it.

## The two rules that recur

Every language so far has needed both. **Check a new adapter against them before measuring:**

1. **A container is a PATH, not a leaf.** `Outer.Inner`, not `Inner`. (TS `module_of`, Java nested types, Python nested classes.)
2. **A body does not declare members of what encloses it.** A local is not a field; a declaration inside a function is named under that function. (`container_at` / `fn_depth` / `fn_scope`.)

## Remaining

1. **C#** — 19,404 files, the biggest gap, never had an adapter. `tree-sitter-c-sharp` dependency is in. Build on `lang/java/`.
2. SQL 7,435 · Kotlin 247 (takes `jvm.rs`) · C 160 · Swift 2 · PHP 2,251.
3. Deploy → `TRUNCATE sensei.nodes, sensei.edges CASCADE` → re-index → acceptance → wire `Stated::Gone`.

## Next command

```
SENSEI_CORPUS=/Users/Jerry/Work/Dayamed cargo test -p senseid --bin senseid \
  java::corpus -- --ignored --nocapture
```
Never pipe a test or build through `tail` — a pipe reports the pipe's exit status.

## Open questions

- C# **partial classes** — one type across several files is a real identity question with no Java analogue.
- C# **properties** are first-class; they want `SymbolKind::Property`, not `Field`.

## Known-broken / not deployed

- This repo has no Java/Python/C#/Kotlin/Swift, so acceptance cannot measure them — use `SENSEI_CORPUS`. An empty denominator is not a pass.
- Java's residual 14: declarations inside enum-constant or anonymous-class bodies.
- Python's residual 3 on llm-gateway: module-level shadowing, which Python itself collapses.
- The daemon predates every cutover. The wipe is required — reconcile reads `props->'claims'`, which 0 of 467,707 nodes carry.
