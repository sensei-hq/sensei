---
title: Every call edge placed — a compiler engine per language
date: 2026-10-13
status: approved (decision 2026-10-12: compiler engine per language)
languages: rust, typescript/javascript, svelte
---

# Every call edge placed

**Definition of done:** 100% of `calls` edges in Rust, TS/JS and Svelte have a
`target_id`. A call into our code lands on a first-party node; into the
language/framework (Rust `core`/`std`/`alloc` and its macros, ES built-ins and
`lib.*.d.ts`, Svelte runes) on an **internal** node; into a dependency on an
**external** node. All three kinds of node are minted by the same FQN function.
A call that is not placed carries a **root-cause reason code** — the code exists
so the miss can be found and fixed, not to excuse it — and the coverage line
reports misses by reason.

## Why the walk alone cannot get there (measured 2026-10-12, project `sensei`)

Unplaced `calls`: Rust 34,101 of 61,168 (44.3% placed); TS/JS/Svelte 6,946 of
12,613 (44.9%). **~90% of the unplaced call sites are method calls on a
receiver** (Rust 50,664 of 57,804; TS 15,642 of 17,765). The adapters use syntax
parsers — tree-sitter-rust, oxc_parser — which give the FORM (`x.unwrap()`), not
the MEANING (which `unwrap`). `listener.local_addr().unwrap()` needs
`TcpListener::bind` → `io::Result<TcpListener>` → `local_addr` →
`io::Result<SocketAddr>` → `Result::unwrap`: generics, `.await`, trait lookup,
auto-deref. That is a type checker. Spec R5 ("name, never open") and §5 ("type
inference is a non-goal") are what this plan replaces.

## The engine resolves them (measured, same day, same graph)

| engine | run | unplaced sites scored | resolved |
|---|---|---|---|
| rust-analyzer 1.99 `scip`, root workspace | 73 s, 5.9 GB peak | 57,804 | **96.8%** — 99.7% within the workspace it indexed |
| `scip-typescript` (tsc), `dojo/` | 5 s, 0.9 GB | 7,555 `.ts`/`.js` | **98.1%** |

The worked example resolves exactly: `local_addr` → `tokio … TcpListener::local_addr()`,
`unwrap` → `core … impl Result<T, E>::unwrap()`. Every reason bucket is answered —
`plumbing`, `external_boundary` and `ambiguous_candidates` included.

What remained, each a root cause with a fix:

| remainder | sites | cause → fix |
|---|---|---|
| `tools/session-report`, `app/src-tauri` | 1,687 | separate Cargo workspaces the run did not cover → run the engine per workspace |
| `gateway_embedded.rs`, `gateway_init.rs`, … | 179 | code compiled out under the active features → `cfg_inactive`; index with the features the build uses |
| `super(...)` | ~50 | no occurrence on the keyword → place on the base class constructor (the class `extends` it) |
| `fireEvent.click` | ~80 | dependency typed `any` → `receiver_any` |
| `rokkit.config.js` | few | file outside the tsconfig → `not_in_project`; index with `allowJs` |
| `it.each([...])(…)` | few | the walk mis-names the callee → walk fix |
| `.svelte` components | 162 (dojo) | not read by the TS engine → svelte2tsx into the same TS program (slice 4) |

## Design — compiler + AST inside each language adapter (decision 2026-10-13)

The pattern does not change: tasks → the repo pipeline (`indexer::index`) →
the language adapters → nodes and edges. Consistent task processors handle every
language; adding a language is adding an adapter and listing it as supported.
So the compiler step lives IN the adapter, beside its AST walk, and the pipeline
gains only a language-neutral hook:

- **Per file** — `read()` stays pure (AST, text in).
- **Once per repository** — after the type barrier and resolution, the pipeline
  already hands each file back to its own adapter in the scope pass
  (`settle_imports`). Call placement joins that pass: each adapter is given, once,
  the repository it is TOLD about (it climbs no directory) to open its compiler
  session, and each file's scope pass asks its own adapter's session to place
  the calls its walk saw, by `(file, span)`. Default for an adapter with no
  engine: nothing — the walk's answer stands.
- **Rust adapter**: rust-analyzer, one session per Cargo workspace or standalone package the pipeline's manifests name — a repository holds several (`sensei`: the root workspace, `tools/session-report`, `app/src-tauri`, the `graph-scan` fixture), and one session at the root sees only the root's members. **TypeScript/JavaScript adapters**: the
  TypeScript compiler. **Svelte adapter**: svelte2tsx into the same TS program,
  spans mapped back through its source map.
- **Library surface.** Calls into the language/framework (internals) or a
  dependency (externals) land on `lib·…` nodes minted from the engine's symbol by
  the same FQN function. First-party symbols must map onto the very FQNs the walk
  minted (asserted).
- **Reason codes are root causes** — each miss keeps one in the data, for the
  cycle below to work through; none is an end state.

## Scope of 100%

Nodes: every declaration in every indexed file of the corpus — functions,
methods, types, modules, constants — placed. Edges: every call (and then every
reference and import) placed. NOT local variables or function parameters: they
get neither nodes nor edges. Every graph and diagram reads 100% placed.

## The cycle, per language (Rust first, then TS/JS, then Svelte)

1. Index the corpus (`sensei` first) and report placement by reason code.
2. Take the largest reason. Pull its call sites, find the root cause, fix it in
   the adapter (or the walk), and verify every site of that reason is now placed —
   on the live graph, not on a fixture.
3. Repeat for every reason code until none remain. 100% nodes and edges is the
   end of the cycle.

## Slices (each red-first, measured on the live graph before and after)

1. **Rust adapter: rust-analyzer + span join** — the adapter opens its session
   once per repository (per Cargo workspace in it) and places `calls` by
   `(file, span)` in the scope pass;
   first-party symbols must equal the walk's FQNs. Gate: Rust `calls` placed ≥
   99% on `sensei`, every miss with a root-cause code.
2. **Library surface from symbols** — mint internal/external nodes from SCIP
   symbols (crate, version, path) with the FQN minter. Gate: no `calls` edge
   whose engine symbol is internal/external is left without a target.
3. **TS/JS adapters: the TypeScript compiler** — same, per `tsconfig` project, `allowJs` on.
4. **Svelte through svelte2tsx** — components into the same TS program, spans
   mapped back through the source map. Gate: TS+Svelte `calls` ≥ 99%.
5. **The remainder, one root cause at a time** — `cfg_inactive`, `super`,
   `receiver_any`, the walk's `it.each` naming — until 100%.
6. **Screens** — the coverage line reads placed / missed-by-reason; nothing is
   counted as "unplaced" without its reason.

Then `references` and `imports` through the same join.
