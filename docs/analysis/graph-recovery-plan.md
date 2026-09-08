# Recovery plan: build for the two stated outcomes, stop optimising a metric

## What went wrong, in one line

The work optimised **call-resolution percentage** — a number nobody asked for —
while the two actual goals rest on parts of the graph that are already complete.
Each fix was locally reasonable, added a bail, and accounted for nothing it
silently dropped. There was no spec to deviate from, so deviations compounded.

## The two goals (these are the acceptance criteria; nothing else is)

**G1. An LLM agent can navigate an unfamiliar repo.**
"Where is X", "who calls X", "what does X depend on", "what shape is this data".

**G2. A human can see the lay of a repo and judge whether it is in trouble.**
Project/folder/module structure, what depends on what, where the risk is
concentrated.

## What is already good enough to build on — measured 2026-09-08

| | state |
|---|---|
| `imports` (module + package dependency graph) | **141,980 resolved / 4 unresolved** |
| containment (symbol -> parent) | 126,916 / 143,840 = **88%** |
| defs (functions, methods, classes, structs, interfaces) | present in every language |
| `calls` | rust 61%, typescript 62% |

**G2 needs almost none of `calls`.** Repo layout, module dependency structure,
cycles, fan-in/fan-out hotspots and orphan detection are all computable from
containment + imports, which are effectively complete. G2 is not blocked.

**G1 is partly blocked** — "what does X depend on" needs calls, and "what shape is
this data" is impossible (0 field nodes, 0 enum-variant nodes anywhere).

## Order of work

1. **Instrument every bail.** `resolve_call` returning `None` emits NO EDGE — the
   call vanishes and never appears in the unresolved count. Four of seven bail
   sites do this. Until each bail emits an unresolved edge with a REASON CODE,
   every coverage number (including all of today's) understates the gap by an
   unknown amount. This is the change that makes the rest measurable.
2. **Ship G2 on what exists.** Structure + dependency views from containment and
   imports. No new resolution required.
3. **Field / property / enum-variant nodes.** Closes the one grade-A G1 capability
   that is currently impossible, and simultaneously unlocks `self.field.method()`
   in every language.
4. **TS type annotations.** `collect_binding` reads one form (`new Foo()`) and
   discards param types, `const x: T`, class property types and return types —
   all parsed by oxc. Largest single recoverable block in `calls`.
5. Only then, chip at the residue against per-cause numbers from step 1.

## Rules that would have prevented today

- **No bail without an edge.** A resolver that cannot place a call emits
  `unresolved` with a reason. Never `None`. A dropped edge is invisible and
  therefore untestable.
- **No resolver change without a measured before/after** on the live graph, and
  the reason-code histogram as the unit of measure.
- **Verify before asserting.** Four claims this session were stated then
  corrected (the `Array` leak, the glob theory, external types being
  unrecoverable, a fixture guard that did not fire). Each cost more than the one
  command that would have checked it.
- **Acceptance is G1/G2, not a percentage.** A change that raises call resolution
  but serves neither goal is not progress.

## Known-good, do not redo

- fqn scheme `<lang>·<package>·<module>·<Type>·<member>`; return type is NOT part
  of it and does not affect it.
- Externals map to `lib·<package>·<member>` with no library internals. This works:
  218,310 edges onto 18,240 lib symbols. Library sources are never needed.
- Absence is NOT evidence of externality — it is scan-order dependent. The import
  is the signal, and it already works.
