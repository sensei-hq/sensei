# Stage 4b — the JS/TS walk: receiver typing is flow-sensitive

Whole-system spec: `docs/design/indexer.md` D4 (one language at a time).
Depends on stage 3. The Rust walk (`04-walk-rust.md`) is the shape to follow;
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
- **`.svelte` files hold three languages.** Script, markup and style. Only the
  script block is this walk's business; the markup's `{expr}` interpolations are
  use sites too, and are a separate decision — NOT silently skipped.
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
| a `.svelte` markup expression | a use site. Decide it explicitly; do not skip it silently. |
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

## 6. Definition of done

- The three STATED routes ship, with the flow-sensitive binding map beneath
  them.
- A reassignment is proven to change the answer, and an untypable one to clear
  it.
- `.svelte` markup expressions have an explicit decision recorded here.
- The cross-file routes are NOT attempted — they wait on #174.
