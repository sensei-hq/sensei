# Capability refactor — the complete work map

Companion to [`2026-09-02-adr-language-capability-traits.md`](./2026-09-02-adr-language-capability-traits.md).
Status: **survey complete, nothing implemented.**

Purpose: size the WHOLE surface before starting, so no chunk is discovered
mid-refactor. Every number below was measured on 2026-09-02 against the live
graph (728,714 nodes) and the tree at `f02976c2`.

---

## 1. The pattern is already established THREE times

| precedent | shape | impls |
|---|---|---|
| `ManifestAdapter` (`adapters/manifest.rs:30`) | trait + `accepts()` discovery | 10 ecosystems |
| `LanguageAdapter` (`languages/mod.rs:21`) | trait, monolithic + mixed-concern | 12 languages |
| `FileClassifier` (`classifiers.rs:17`) | trait + `&'static dyn` accessor | 1, created **to consolidate two inline extension lists** |

`FileClassifier` matters most: it exists *because* per-language lists had leaked
into `helpers.rs` and `scan_logic.rs`, and someone pulled them back behind a
trait with exactly the accessor shape this ADR proposes. This refactor is
finishing a job the codebase already started twice.

## 2. Capability matrix TODAY (derived from the code, not aspirational)

| adapter | fqn_output | inheritance extracted | host delegation | components |
|---|---|---|---|---|
| rust_lang | ✓ | ✓ trait impls | — | — |
| java | ✓ | ✓ extends + implements | — | — |
| python | ✓ | ✓ base class | — | — |
| typescript | ✓ | — | *is a host* | — |
| sql | ✓ | n/a | — | n/a |
| svelte | ✓ | — | ✓ → typescript | ✓ |
| vue | ✓ | — | ✓ → typescript | ✓ |
| **kotlin** | **✗** | ✗ | — | — |
| **swift** | **✗** | ✗ | — | — |
| **c_lang** | **✗** | ✗ | n/a | n/a |

Three languages produce **no FQN nodes at all** and nothing records whether that
is a decision or an omission — the gap the derived matrix + pinning test closes.

Three languages **already extract inheritance that is never persisted** (0
`implements` edges). That is the smallest, highest-value first slice.

## 3. Work inventory by layer

### 3a. Business layer — capability traits

| capability | producers today | consumers today | work |
|---|---|---|---|
| `Inheritance` | java, python, rust (extract only) | **none** | wire persistence; add ts/kotlin/swift |
| `ImportPaths` | `import_target` free fns + rust special-case | `process.rs` | move to trait; delete special-case |
| `LibraryOrigin` | `libraries.rs:62-119` inline skip-list | `props.libs` (dead field) | move to trait; unblocks 136,642 externals |
| `Components` | svelte/vue mention it | 2 component + 1 hook nodes | new |

### 3b. The contract — `GraphFacts`

Five persistence policies, hand-written per concern in one function:

| concern | policy | site |
|---|---|---|
| `calls` | get-or-create by FQN | process.rs |
| `imports` | lookup-first, stub on total miss | process.rs |
| `references` (file) | lookup-first, never create | process.rs |
| `references` (symbol) | lookup-first, only if unambiguous | process.rs |
| `extends` | none — raw insert, `target_id = None` | process.rs |

Collapse to three `OnMiss` values. **Blast radius:** 79 node/edge write calls
outside `pg_store`, concentrated — `process.rs` 44, `scan.rs` 19,
`index_audit.rs` 6, `routes.rs` 6 (tests), `fqn.rs` 2.

### 3c. Data layer — typed rows

```
PgStore:  551 public methods · 139 return untyped serde_json::Value
Stringly-typed field reads outside pg_store:  462
```

Top readers: `routes.rs` 90 · `observatory.rs` 35 · `codebase.rs` 28 ·
`sessions.rs` 21 · `runs.rs` 21 · `mcp.rs` 19 · `process.rs` 17 ·
`metrics/autonomy.rs` 17.

**This is the largest single chunk and the one most likely to be
under-estimated.** It is NOT a rewrite: new code uses typed rows, existing
methods convert when touched. `FolderRow` first — `scan.rs` reads
`r["kind"]`/`r["abs_path"]` with `.unwrap_or("")`, so a renamed column silently
skips a folder rather than failing.

### 3d. Schema capacity already paid for, unused

```
edge_kind   11 declared,  4 used   → implements, depends_on, traces_to,
                                      covers, rationale_for, duplicates,
                                      similar_to  ALL ZERO
node_kind   23 declared, 18 used   → enum_variant, package, property, field,
                                      parameter ALL ZERO
                                      component 2, hook 1
```

Only `implements` has an obvious owner in this refactor. The rest are listed so
they are not mistaken for new work later.

## 4. Slice plan with blast radius

| # | slice | files | risk | gate |
|---|---|---|---|---|
| 1 | `all_adapters()` + `capability_matrix()` + pinning test | 1 + test | none — additive | matrix test |
| 2 | `Inheritance` trait; persist java/python/rust; retire 7,901 bogus `extends` | ~5 | **medium** — removes edges a UI draws | graph view renders |
| 3 | `GraphFacts` + persister; migrate `Inheritance` onto it | ~3 | low | suite |
| 4 | Migrate the 4 existing policies to `OnMiss` | process.rs | **medium** — touches all resolution | resolution % unchanged |
| 5 | `ImportPaths` trait | ~6 | low — behaviour-preserving | suite + resolution % |
| 6 | `LibraryOrigin` trait | ~4 | low | suite |
| 7 | Externals → `lib_symbol` (136,642 edges) | ~3 | **high** — mints nodes | lookup-first test; node count |
| 8 | `Components` (svelte/vue/tsx) | ~4 | low | component node count > 3 |
| 9 | `FolderRow` typed row + convert `scan.rs` readers | ~4 | low | suite |
| 10 | `NodeRow`/`EdgeRow`; convert on touch | ongoing | low | suite |

Slices 2, 4 and 7 are the ones that can change graph CONTENT; each needs a
before/after measurement, not just a green suite.

## 5. Deliberately NOT in scope

* The 6 other unused edge kinds (`traces_to`, `covers`, …) — no owner identified.
* The 5 unused node kinds — same.
* Converting all 551 store methods — boy-scout only.
* `metrics/*`, `dojo/*`, `gateway/*` — untouched by this refactor.

## 6. Open questions surfaced by the survey

1. **`RelationKind::TraitImpl`** — own edge kind, or `implements` + discriminant?
   Rust trait impls are not Java interface implementation, but "what implements
   X" wants both.
2. **RESOLVED BY THIS SURVEY, and it is not a refactor issue — 39.7% of the
   graph is vendored C headers.** `const` is the largest node kind (292,555 —
   more than `function` 96,302 and `method` 60,166 combined), and 289,377 of
   those are `language='c'`. Of those, **289,258 come from
   `docs/proposal/deck-node/include/node/openssl/...`** — Node.js's bundled
   OpenSSL headers, vendored inside a `docs/proposal` directory, across 477
   header directories.

   `c_lang.rs:107-115` is doing the right thing (`#define` → `SymbolKind::Const`).
   The problem is that vendored third-party headers are indexed as first-party
   code. Consequences: node counts, every per-kind statistic, and the "internal
   92.5%" locality figure are dominated by third-party macros; `c_lang` also has
   no FQN support, so none of it can resolve anything.

   This smells like scan exclusions not covering vendored paths — and
   `graph_nodes.ddl` already records a prior incident where "the scan exclusion
   resolver gated the watcher while pruning nothing". **Separate slice, separate
   investigation; do it BEFORE any measurement that reasons about node kinds or
   locality percentages, because those numbers are currently 40% noise.**
3. **Retiring `extends` removes data `codebase.rs:55` renders.** Confirm the
   graph view before/after.
4. **Does `kotlin`/`swift`/`c_lang` having no FQN support reflect a decision?**
   The matrix will force an answer.
