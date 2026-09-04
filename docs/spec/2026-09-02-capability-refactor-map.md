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
| kotlin | ✓ *(slice 1b)* | ✗ | — | — |
| swift | ✓ *(slice 1b)* | ✗ | — | — |
| c_lang | ✓ *(slice 1b)* | ✗ | n/a | n/a |

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
| 0 | **Exclude vendored third-party sources** (removes ~289k C-header consts, 40% of the graph) | ~2 | low | node count; kind mix |
| 1 | `all_adapters()` + `capability_matrix()` + pinning test — test asserts FQN for ALL | 1 + test | none — additive | matrix test |
| 1b | **FQN for kotlin, swift, c_lang** (now required, not optional) | ~3 | low | matrix test goes green |
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

## 6. Decisions (settled 2026-09-02)

1. **`TraitImpl` rides `implements` with a discriminant in `props` — DECIDED.**
   Java declares the relation at the class site (`class Dog extends Animal
   implements Runnable`); Rust's `impl Runnable for Dog` is a separate item with
   its own location, possibly in another file. The distinction is real, but the
   primary query — "what implements X" — wants BOTH, and a consumer that forgets
   one of two edge kinds returns a silently incomplete answer. That is the exact
   failure mode this session spent its time fixing (`get_callers` returning `[]`
   by querying the resolved-only column). A discriminant that gets ignored is
   cosmetic; a forgotten edge kind is a wrong answer. `edge_kind` also already
   carries 7 unused values — adding an eighth for a nuance `props` can hold is
   the wrong trade.
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
4. **FQN support is REQUIRED for every language — DECIDED, and it reverses the
   earlier framing.** The matrix was going to "force an answer"; the answer is
   that optional FQN is itself the bug. Today kotlin/swift/c symbols ARE created
   (via the line-based path) but carry no `fqn`, so an FQN lookup can never find
   them — while `sole_definition_id_by_name` DOES match them by bare name. The
   same symbol is therefore findable by one mechanism and invisible to another,
   with nothing telling a caller which they are getting. That is worse than
   either extreme: unmatched references at best, mismatched at worst.

   The obvious objection — "C has no namespaces, what would its FQN be?" — does
   not hold. C symbols are globally unique by linkage, or file-scoped when
   `static`, so `c·<package>·<file>·<name>` is constructible and MORE precise
   than name-matching. With decision 2 removing the vendored headers, C's volume
   collapses and this becomes cheap.

   Consequence for the plan: `fqn_output()` stops being an `Option`-returning
   default. Slice 1's matrix test asserts EVERY adapter supports it, which turns
   the three current gaps into build-visible work rather than silent absence.

5. **Vendored third-party headers are excluded from indexing — DECIDED.** They
   are effectively external libraries; their constants are not wanted. See §6.2.

6. **Retiring the bogus `extends` edges requires a before/after check of the
   graph view (`codebase.rs:55`) — DECIDED.**


## Slice 1b — CLOSED (030e34db)

Every adapter now has an FQN producer and the `KNOWN_GAPS` allowlist is gone
rather than left at zero. `every_adapter_supports_fqn_with_no_exceptions` is the
gate; adding a language without a producer is now a build failure.

`LanguageAdapter::fqn_output` gained `rel_path`. C and Swift scope on layout
rather than a package declaration, and the indexer already held the correct
anchor without passing it — `rel_path` is folder-relative and is the same value
stored in `nodes.file_path`, so a C module now equals its `file_path` minus the
extension.

Two things worth carrying forward:

- **Real data refuted a fallback that reasoning had approved.** The no-build-root
  case originally used the bare file stem. A C project in the corpus has parallel
  `Cpp/` and `Hpp/` trees with no build file, so a header and its implementation
  shared a module — and a header/impl pair declares and defines the same names by
  construction, making the collision certain rather than unlikely. Hand-written
  fixtures had agreed with the bug.
- **A passing corpus test does not mean the branch you changed is covered.**
  Mutating the no-build-root fallback did NOT fail
  `no_produced_fqn_embeds_an_absolute_path`, because this repo has a root
  `Makefile` and every `.c` file here resolves through the build-root branch
  instead. The fallback is covered by unit tests only. The coverage boundary is
  now written into that test's doc comment.

NOT YET LIVE IN THE GRAPH: the running daemon is the previously installed 0.9.1
binary. Kotlin/C/Swift FQNs require `make install-debug` + a reindex, batched
with the later slices rather than run per-slice.


## Slice 3 — SCOPE TRIMMED, with the user's approval (2026-09-04)

Slice 3 as mapped was `GraphFacts` + a persister + migrating every emit policy
onto `OnMiss`. It is trimmed to increments 0-4: the golden differential, the
contract types, the persister, and migrating INHERITANCE and CALLS. Imports and
doc references stay as they are.

The reason is measured, not aesthetic. The DRY duplication that actually exists
across the emit paths is ONE resolution ladder written twice — calls
(`process.rs:1443`) and inheritance (`process.rs:1524`) — plus one lib-package
derivation written twice. Both collapse at increment 4. The import and
doc-reference arms each have exactly one lookup strategy and one miss action,
are already the best-covered by tests, and migrating them would reopen paths
carrying 730,967 live edges for no measurable gain.

Recorded here rather than in a source comment, per the CLAUDE.md rule on
documenting deviations.

### Corrections to this map, found while designing slice 3

- There are **SIX** emit paths, not five. The sixth is the non-FQN legacy call
  arm at `process.rs:1591`, reached when a file has no manifest so
  `result.fqn` is None. Any claim of "all emit paths" that lists five is wrong.
- The ADR's `OnMiss { LeaveUnresolved, CreateStub, RequireUnambiguous }` has a
  variant with no real user and is missing one real behaviour. Use the
  distinct-behaviour count from the code, not the ADR's three.
- `nodes.language` is last-writer-wins on the reference path
  (`graph.rs:392` uses `COALESCE(EXCLUDED.language, nodes.language)` without the
  `CASE WHEN EXCLUDED.resolved` guard its neighbours have), while the calls,
  imports and inheritance stub arms all pass the REFERRING file's language. A
  cross-language reference can therefore relabel a node. Pre-existing; not
  introduced by slice 2.

### The gate's design was WRONG and is redesigned

The first design of increment 0 drew eight blocking objections. Three mattered:

1. **Two-order equality is false by construction.** Doc references resolve at
   emit and never heal — there is no stub arm — so processing the doc file first
   vs last yields different records. Order-invariance may only be asserted over
   the arms that have it (calls, inheritance), with the doc file held in a fixed
   position and the reason written down.
2. **The anti-vacuity check was a tautology.** A map built by counting observed
   rows, then asserting each count >= 1, cannot fail: a missing arm is an absent
   key, not a zero. Iterate a HARDCODED arm list instead.
3. **Four arms are not derivable from final state.** `imports/probe-hit` vs
   `imports/stub` and `inh/in-file` vs `inh/stub` converge to identical rows once
   the fixture is fully processed. Arms must be instrumented AT THE EMIT SITE and
   recorded as their own row type, or the gate cannot prove the branches
   increments 3-4 delete.
