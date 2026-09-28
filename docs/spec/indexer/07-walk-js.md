---
status: current
---
# Stage 4b — the JS/TS walk: receiver typing is flow-sensitive

Whole-system spec: `docs/design/indexer.md` D4 (one language at a time).
Depends on stage 3. The Rust walk (`06-walk-rust.md`) is the shape to follow;
this records only what DIFFERS, and the difference is not small.

> **Build order note (D4).** D4 governs CUTOVER, not construction: js/ts/svelte
> follow rust one at a time, and the gate to switch is §6 passing. Building this
> while rust's gate blocks is fine. Shipping it as the live indexer is not.

## 1. The one structural difference from Rust

**A Rust binding is lexical. A JS binding is flow-sensitive.**

```js
let x = new Foo();
x.method();        // Foo
x = makeBar();
x.method();        // NOT Foo — and nothing lexical says so
```

Rust's `let` shadows: the second `let x` introduces a NEW binding, and a
lexically-scoped map is exactly right. JS `x = …` REASSIGNS the same binding, so
the type at a use site is whatever the MOST RECENT assignment before it said —
which is a property of position in the statement list, not of scope.

This is the whole of why the Rust walk's `Scope::bindings` cannot be reused
as-is. Recording at the declaration and reading it anywhere in the block is
correct for Rust and WRONG for JS: it would type the second `x.method()` as
`Foo`, which is a wrong identity, and R4 ranks that below no identity.

**S1.** A binding's type is the most recent assignment at or before the use
site. An assignment the walk cannot type CLEARS the binding rather than leaving
the previous type in place — "I no longer know" is the truth, and keeping a
stale type is the fabrication.

**S2.** A reassignment inside a branch makes the type unknown after the branch
joins, unless every arm assigns the same type. Not "the last arm wins": which
arm ran is not knowable, so the join is the intersection, and an empty
intersection is an honest unknown.

## 2. Measured on this workspace — 6,406 member calls, 925 files

The routes, by how the receiver is introduced:

| | | route | |
|---:|---:|---|---|
| 1,887 | 29.5% | not bound in this file | needs the module graph |
| 967 | 15.1% | `const x = <expr>` | needs the expression's type |
| 848 | 13.2% | `import { x }` | the MODULE states it — cross-file |
| 796 | 12.4% | `const x = new T()` | **the constructor names the type** |
| 696 | 10.9% | `this` / a global | container, or not ours |
| 634 | 9.9% | `const x: T` | **TS annotation** |
| 306 | 4.8% | `fn(x: T)` | **TS signature** |
| 272 | 4.2% | `const x = await f()` | needs the RETURN type |

The measurement is a file-wide regex — an upper bound, not scope-aware, and its
route ordering is arbitrary where a name matches two. It sizes the buckets; it
does not settle any single case.

**S3.** Build the STATED routes first: `new T()`, a TS annotation, a TS
signature. Together 1,736 of 6,406 (27%), all of them things the source writes
down, none needing a second pass.

This ordering is not a preference. On the Rust side every route built without
sizing it first reached almost nothing — a `for`-binding route that was correct,
tested, and moved the corpus by one. Size, then build.

**S4.** `import { x }` (848) and "not bound in this file" (1,887) are the same
question: the type lives in another module. That is the cross-file lookup, and
it belongs with `ReceiverHint`'s successor (issue #174), not here.

## 3. What JS has that Rust does not

- **No type annotations in `.js` at all.** The `const x = new T()` route is the
  only in-file one that works there, which is why it leads S3.
- **`.svelte` files hold three languages.** Script, markup and style. The
  markup's `{expr}` interpolations are use sites and are READ — §7 records that
  decision and why.
- **A module is a value.** `import * as api` then `api.thing()` — the receiver
  is a namespace, not a type, and its members are the module's exports. A
  different lookup from a method on a type, and conflating them mints members on
  a thing that has none.

## 4. Failure modes

| input | this stage does |
|---|---|
| `x` reassigned with an untypable value | CLEAR the binding. A stale type is worse than none (R4). |
| a branch assigns two different types | unknown after the join; not "last arm wins". |
| `import * as ns` | a namespace, not a type. Members are exports, resolved as such. |
| a `.svelte` markup expression | a use site, READ — see §7. |
| `.js` with no annotations anywhere | the `new T()` route only, and the rest honestly unresolved. |

## 5. Verification

| test | mutation that must break it |
|---|---|
| a reassignment changes the type at the next use site | record at the declaration only, Rust-style |
| an untypable reassignment CLEARS rather than keeps | leave the previous type in place |
| two branches assigning different types leave it unknown | take the last arm |
| `const x = new T(); x.m()` resolves to `T::m` | — |
| a `.js` file with no annotations still resolves `new T()` | gate the route on TS |
| a namespace import's member is an EXPORT, not a type member | resolve it as a method |

## 6. Definition of done — MET

- The three STATED routes ship, with the flow-sensitive binding map beneath
  them.
- A reassignment is proven to change the answer, and an untypable one to clear
  it.
- `.svelte` markup expressions have an explicit decision recorded here (§7).
- The cross-file routes are NOT attempted — they wait on #174.

## 7. The decisions this spec left open, made

### `.svelte` markup expressions are READ

`{store.load()}` is a call with the same name, the same receiver and the same
imports it would have inside the script. In this workspace's three SvelteKit
applications most components do their work there, so skipping the markup would
report them inert. So each interpolation is parsed as an expression, in the
dialect the `<script>` block states, and walked with the script's bindings in
scope — ONE walk over the blocks and the markup, because two would each start
from an empty binding map and the markup would type nothing.

What is NOT read is stated as explicitly: a closing tag and a bare `{:else}`
name nothing and emit nothing; a tag whose contents will not parse emits an
`UnhandledForm` miss, so the histogram says what was not understood (R2, S8).
`{#each xs as x}` reads `xs` and CLEARS `x`, because an element type is not
something this file states.

### A trailing `index` is KEPT in a module path

Dropping it would mirror Rust's `mod.rs` rule and it collides: `src/index.ts`
and `src/index/index.ts` would both reduce to `index`, and two files claiming
one identity is the failure §2 exists to prevent. Keeping it costs a MISS
instead — `import x from './a'`, which Node resolves to `a/index.ts`, mints `a`
and matches nothing. R4 prefers the miss. Closing it needs the resolver to try
both spellings, which is the cross-file lookup deferred to #174.

### A closure keeps what the file never reassigns

A nested function can run at any time, so carrying the enclosing binding map
into it would be exactly the stale type S1 refuses. But a name the file never
assigns to ANYWHERE cannot have changed, so its type still holds inside the
closure. That is a proof rather than an optimism, and it is what types Svelte's
`{() => store.load()}` where `store` is a module-level `const`.

## 8. Measured after building, over 998 real files

`app/`, `dojo/` and `website/`, read through the adapter each extension
dispatches to:

| | |
|---:|---|
| 998 | files, **0 unreadable** |
| 13,832 | symbols |
| 60,391 | references |
| 4,021 | relations |
| 5,713 | imports |
| 3,299 of 33,028 | member receivers typed by a STATED route |

The reason histogram: `ReceiverTypeUnknown` 29,729, `Unplaced` 29,133,
`DynamicDispatch` 1,364, `UnhandledForm` 165.

**Two things to read carefully before sizing the next route.**

The 33,028 is NOT the 6,406 of §2 grown. §2's file-wide regex counted member
CALLS; this counts every member ACCESS — a read and a write as well as a call.
The denominators are different populations, so 3,299/33,028 (10%) and §2's
projected 27% of calls are not the same fraction and must not be compared.

`UnhandledForm` at 165 of 60,391 is the number that says the walk is not
missing a common shape. `ReceiverTypeUnknown` at 29,729 is the cross-file
question — §2 already sized it at 45% of member calls between `import { x }`
and "not bound in this file" — and it is #174's, not this stage's.
