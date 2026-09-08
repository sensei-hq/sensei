# Code-graph coverage: what is missing, why, and what it costs

All figures measured on the live graph (48,654 files) on 2026-09-08. No estimates.

## 1. The one-line diagnosis

The AST is not the limitation. **We parse type information and then discard it.**
Three confirmed instances, each independently verified:

| discarded | evidence |
|---|---|
| TS/JS type annotations | `collect_binding` (typescript.rs:1305) reads ONE form, `const x = new Foo()`. Not param types, not `const x: T`, not class property types, not return types — all of which oxc parses into `TSTypeAnnotation` nodes. |
| rust return types | `extract_return_type` ran on every fn, filled `IRFunction.return_type`, and was dropped at persistence. Fixed 2026-09-08 (`5aeb5caf`). |
| struct/class fields | 1,801 rust structs -> **0** field nodes. 1,245 TS classes -> **0** property nodes. `rust_lang.rs` has no `field_declaration` arm at all. |

## 2. Where the graph stands

Edges, by language:

| | resolved | unresolved | ghost (points at an invented node) |
|---|---:|---:|---:|
| rust `calls` | 46,545 | 29,875 | 4,542 |
| typescript `calls` | 104,634 | 64,050 | — |
| javascript `calls` | 7,327 | 11,851 | — |
| svelte `calls` | 1,790 | 1,118 | — |
| **`imports` (all)** | **46,325 + 3,718** | **0** | — |
| TS `extends` | 215 | 111 | — |
| rust `implements` | 726 | 27 | — |

`imports` is 100% in every language. That is the proof the approach works when the
AST names a target: an import names a file, so it always resolves.

## 3. Causes of unresolved calls, ranked by measured size

| # | cause | size | recoverable from AST? |
|---|---|---:|---|
| 1 | TS type annotations never read | ~64,050 | **YES — fully.** The types are written in the source. |
| 2 | plain JS, no annotations | 11,851 | No. Needs real inference (what `tsc` does). |
| 3 | rust receiver chains deeper than one link | 12,385 | Partly — needs resolved types fed back into the binding map. |
| 4 | class/struct fields not indexed (`self.field.method()`) | 234 rust sites in this repo alone; TS unmeasured | **YES — fully.** The field's type is declared. |
| 5 | external crate/package types (`.bind()`, `.execute()`, sqlx/std) | large share of rust's top-20 | No. The type lives in a dependency we do not index. |
| 6 | unplaceable types -> ghost stubs (glob imports, `pub use`, macro impls) | 4,542 | No, not from one file. |

Causes 1 and 4 are ours. Causes 2, 5, 6 are genuine limits of syntax-only parsing.

## 4. What matters, graded

Grade = value to an LLM agent working in the codebase.

| capability | grade | state |
|---|---|---|
| "where is X defined" | A | **works** — defs complete, imports 100% |
| "who calls X" | A | **works** — `get_callers` complete on real symbols |
| "what does X depend on" | A | **61%** — cause 1 is the bulk |
| "what shape is this data" (fields/variants) | A | **impossible** — 0 field nodes, 0 enum-variant nodes |
| "what implements this interface" | B | mostly works (rust 726/27; TS extends 215/111) |
| "what is the call depth / complexity here" | B | derivable once calls are complete |
| OO design patterns (adapter, factory, ...) | C | not detected; inheritance edges now exist as substrate |
| macros / generated code | C | invisible by nature; 2 `macro_rules!` in this repo |

## 5. Order of work, by yield

1. **TS type annotations** — params, `const x: T`, class properties, return types.
   Largest single recoverable block in the graph (~64,050). Extends `collect_binding`
   from one form to four; no new infrastructure.
2. **Field / property nodes** — unlocks "what shape is this data" (grade A, currently
   impossible) AND `self.field.method()` in every language. One new node kind.
3. **Enum variant nodes** — 428 rust enums, 27 TS enums, 0 variants.
4. **Feed resolved receiver types back into the binding map** — reaches rust's 12,385.

## 6. Not recoverable without new inputs

Causes 2, 5, 6. Closing them needs either indexing dependency sources, or a real
type-checker (`tsc` / `rust-analyzer`) rather than a syntax tree. That is a
different architecture and should be a deliberate decision, not a drift.
