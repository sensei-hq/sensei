# Checkpoint

**Slice:** indexer v2 — languages, before the wipe + re-index (issue #130, phase `build`)

## Cut over

`PRODUCTION_LANGUAGES = [Rust, TypeScript, Java, Python, CSharp]`.

| language | A7 | gate |
|---|---|---|
| Rust | 19 → **0** | `persist::…_rust_…` |
| TypeScript (+js/svelte/vue) | 511 → **0** | `persist::…_typescript_…` |
| Java | 409 → **14** (named) | `java::corpus::…` (`SENSEI_CORPUS`) |
| Python | 72 → **0** | `python::tests::…` (`SENSEI_CORPUS`) |
| **C#** | **414** (every bucket explained) | `csharp::tests::…` (`SENSEI_CORPUS`) |

**~5,600 lines of v1 parser deleted.** `languages/jvm.rs` stays — only Kotlin uses it.

C# was **additive**: v1 never had a parser, so 19,404 `.cs` files carrying no language and no nodes now have a graph. 137,484 declarations over 8,647 files, 0 unreadable.

## The two rules that recur

Check any new adapter against both **before** measuring. Every language has needed them:

1. **A container is a PATH, not a leaf** — `Outer.Inner`.
2. **A body does not declare members of what encloses it** — a local is not a field.

## Remaining

1. SQL 7,435 · Kotlin 247 (takes `jvm.rs`) · C 160 · Swift 2 · PHP 2,251.
2. Deploy → `TRUNCATE sensei.nodes, sensei.edges CASCADE` → re-index → acceptance → wire `Stated::Gone`.

## Next command

```
SENSEI_CORPUS=/Users/Jerry/Work/Ethico cargo test -p senseid --bin senseid \
  csharp::tests -- --ignored --nocapture
```
Never pipe a test or build through `tail` — a pipe reports the pipe's exit status.

## Open questions

- **C#'s residue is conditional compilation** (74 of 414): `#if`/`#else` declarations are rust's `cfg` problem in C#'s syntax. The answer is the same callable-plus-arm split; not applied yet, and the `#if` block appears to break containment (these land at file scope with no type segment).
- C#'s base list gives a base class and an interface one syntax. Interfaces get `Extends`, classes `Implements`. Resolvable from the *target's* declaration, which is scan-level.

## Known-broken / not deployed

- **`tree-sitter-c-sharp` is pinned `=0.23.1`** — 0.23.5 emits ABI 15 against a 13–14 runtime and fails at `set_language` at *runtime*, on every `.cs` file.
- This repo has no Java/Python/C#/Kotlin/Swift, so acceptance cannot measure them. Use `SENSEI_CORPUS`; an empty denominator is not a pass.
- A real finding in Ethico: `Campaign_RiskAssesment.cs` and `Campaign_RiskAssessment.cs` are two 28-line files declaring the same class — one left behind when a filename typo was fixed.
- The daemon predates every cutover. The wipe is required — reconcile reads `props->'claims'`, which 0 of 467,707 nodes carry.
