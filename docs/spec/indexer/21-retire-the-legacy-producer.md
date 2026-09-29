---
name: Retire the legacy producer
description: Full inventory of what remains of the pre-cutover indexer, and the sequence that removes it — ending with no first/second-producer vocabulary anywhere
date: 2026-09-29
status: draft
---

# Retire the legacy producer

Two producers have been writing to one set of tables. The newer one
(`indexer/`) owns seven of nine languages; the older one (`languages/`) still
owns the rest and still supplies helpers that the newer one and the DB layer
call. This states exactly what remains, in what order it comes out, and what
"done" means.

Measured 2026-09-29. Every count here is from the tree or the live graph.

## 1. What actually remains

| tree | files | lines | role |
|---|---:|---:|---|
| `crates/senseid/src/indexer/` | 44 | 54,167 | the producer that stays |
| `crates/senseid/src/indexer/lang/` | — | 27,748 | its nine adapters |
| `crates/senseid/src/languages/` | 10 | 10,032 | what comes out |

`languages/` is **not** 10,032 lines of legacy walking. It is four things, and
only two of them are legacy:

| file | lines | what it is |
|---|---:|---|
| `rust_lang.rs` | 3,637 | legacy walker — **but rust is already cut over**; it survives for two helpers (below) |
| `kotlin.rs` | 1,443 | legacy walker, still the producer for Kotlin |
| `swift.rs` | 733 | legacy walker, still the only Swift producer |
| `sql.rs` | 481 | legacy walker, still the producer for SQL |
| `jvm.rs` | 156 | shared by the Java (deleted) and Kotlin walkers |
| `corpus_tests.rs` | 455 | tests for the above |
| `import_target.rs` | 1,253 | **not legacy** — the import classifier, called from the live path |
| `fqn.rs` | 464 | the second identity encoder — see §3 |
| `mod.rs` | 1,182 | the legacy adapter registry **and** five language-agnostic utilities |
| `common.rs` | 228 | helpers for the walkers |

## 2. The frontier

One declaration decides which producer runs, and the dispatcher reads it rather
than restating it — `lang::PRODUCTION_LANGUAGES`:

```
Language enum   Rust  TypeScript  Java  Python  CSharp  Kotlin  Php  C  Sql
in production   ✅    ✅          ✅    ✅      ✅      ❌      ✅   ✅  ❌
```

**Kotlin and SQL already have adapters** — `indexer/lang/kotlin/` (971-line walk)
and `indexer/lang/sql/` — both registered in `all_adapters()`. They are absent
from `PRODUCTION_LANGUAGES` and nothing in the code records why. That is the
first thing to establish: either they are ready and the frontier is simply
stale, or there is a known gap that was never written down.

**Swift has no adapter.** `languages/swift.rs` is 733 lines for 2 files in the
watched roots. Building an adapter for two files is not obviously worth it;
dropping Swift is a decision to take deliberately rather than by omission.

## 3. The second encoder — the live defect

Two functions mint identities for the same symbols:

```
indexer::fqn::encode    [lang, package, module, name, REACH]   5 segments
languages::fqn::item    [lang, package, module, name]          4 segments
```

The reach segment (`item`, `mod`, `field`) is what keeps a field and a
same-named method apart. **100% of module nodes in the live graph carry it, across
all six languages present; not one is in any other shape.** So any lookup built
with the four-segment encoder could not miss sometimes — it had to miss always.

Two such lookups were found and fixed (`5be6a995`, and this doc's commit):

- `local_import_candidates` — why `imports` sat at **2.3% placed** against
  **48.5% for `references`**, the difference being that references go through
  the ladder and the ladder uses the five-segment encoder.
- `resolve_receiver_calls` — why `receiver_type_unknown` is a top fault code.

Remaining call sites of the four-segment encoder: **4**, tracked in #201.
`is_external`, `sql_is_external` and `finders` are predicates that READ an fqn,
not minters, and are not part of this.

## 4. The five strands

They are not one migration. Strand C is the only one with a hard dependency.

**A — the second encoder (#201).** Independent of everything else. Route the
four remaining mint sites through `indexer::fqn::refer` and delete the minting
half of `languages/fqn.rs`. The `-`/`_` crate-name fold currently has three
copies (the ladder, the import probe, and the receiver chase); it belongs beside
the encoder.

**B — the shared utilities.** `languages/mod.rs` exports five functions with no
producer flavour at all — `language_for_path`, `language_for_ext_slug`,
`text_language_from_content`, `is_test_path`, `compute_complexity` — and they
have 21 callers across the daemon. They are not legacy; they are badly located.
They move to a neutral module and stop being a reason to keep `languages/`
alive. `import_target.rs` is the same case: it is the live import classifier.

**C — the remaining languages.** Kotlin and SQL flip (adapters exist); Swift is
built or dropped. **Every other deletion below is gated on this**, because the
dispatcher's fallback arm cannot go while any language needs it.

**D — the walkers.** Once C lands: delete the legacy arm of `process_file`, then
`rust_lang.rs`, `kotlin.rs`, `sql.rs`, `swift.rs`, `jvm.rs`, `common.rs`,
`corpus_tests.rs` and the legacy adapter registry in `mod.rs`. `rust_lang.rs`
can go earlier than the rest — rust is already cut over and it survives only for
`ReceiverType` / `concrete_receiver_type`, which are pure string parsing and
belong in `indexer/lang/rust/`.

**E — the vocabulary.** With one producer there is no first and second, so the
words stop being meaningful and become archaeology. 853 mentions of `v1`/`v2`
exist across 214 files, but **most are unrelated** — dojo auth versions, gateway
API versions, design docs. Only the indexer ones are in scope, concentrated in
`tasks/handlers/process.rs` (33), `languages/mod.rs` (18) and the indexer specs.
The comments that explain a *decision* keep their content and lose the labels:
"v1 minted four segments" becomes "an earlier encoder minted four segments".

## 5. Sequence

1. **Establish Kotlin and SQL readiness.** Run each adapter over the corpus and
   compare against the legacy walker's output. This is the gate for C, and it is
   the one step whose answer is not yet known.
2. **Decide Swift** — adapter or drop. Two files; say which and why.
3. **A** — the encoder. Independent; can run in parallel with 1.
4. **B** — relocate the shared utilities. Independent.
5. **C** — flip Kotlin and SQL into `PRODUCTION_LANGUAGES`.
6. **D** — delete the fallback arm and the walkers.
7. **E** — the vocabulary scrub, last, when there is genuinely one producer.
8. **Re-index and re-measure.** The point of all of it: `imports` placement, the
   cross-crate edge count, and the fault-code distribution.

## 6. Done

Three greps, all of which must return nothing:

```
rg 'languages::fqn::(item|lib|method)'          # one encoder
rg 'crate::languages::(rust_lang|kotlin|sql|swift)'   # one walker set
rg -i '\b(v1|v2)\b' crates/senseid/src/indexer crates/senseid/src/tasks
```

Plus: `crates/senseid/src/languages/` no longer exists, `PRODUCTION_LANGUAGES`
covers every `Language` variant the build reads (so `production_adapter_for_ext`
and `adapter_for_ext` mean the same thing and one of them goes), and the live
graph shows edges crossing a package boundary — which is the measurement this
whole exercise is for.
