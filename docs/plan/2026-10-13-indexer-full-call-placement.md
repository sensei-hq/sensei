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

## Design

The tasks → language adapter → nodes/edges flow stays. Each adapter gains one
semantic step, and the engine is the ONE resolver of call sites for its
language — never a fallback after the walk's ladder (no two-engine chain).

- **The engine runs as a task, per workspace/package, as a subprocess**
  (`rust-analyzer scip`, the TS program). Not in the daemon: rust-analyzer peaks
  at 5.9 GB, which a background daemon must not hold. Its output is a per-file
  table `(file, span) → symbol`.
- **Files are still walked in parallel.** A call site's edge is written with its
  span; placement joins it to the engine's symbol by `(file, span)` whenever both
  exist — so an edge is placed as soon as its file and the engine's answer for
  that file are in, whichever arrives first. Re-running the engine for a changed
  workspace re-places only the files it changed.
- **Symbol → FQN.** One mapping from a SCIP symbol (`<manager> <package> <version>
  <descriptors>`) to our FQN minter. First-party symbols must map onto the very
  FQNs the walk already minted (asserted, see gates). Internal and external
  symbols mint `lib·…` nodes — the library surface — on demand, by the same
  function.
- **Reason codes are root causes**, written on the edge when the engine has no
  symbol for a span: `engine_not_run` (the file's workspace has no index yet),
  `not_in_project` (outside the tsconfig / cargo targets), `cfg_inactive`,
  `receiver_any`, `no_occurrence` (the engine emitted nothing at that span),
  `symbol_unmapped` (a symbol our mapping cannot yet name). The old walk
  reasons stop being produced for these languages.

## Slices (each red-first, measured on the live graph before and after)

1. **Rust engine task + span join** — run `rust-analyzer scip` per Cargo
   workspace, load the occurrence table, place `calls` by `(file, span)`;
   first-party symbols must equal the walk's FQNs. Gate: Rust `calls` placed ≥
   99% on `sensei`, every miss with a root-cause code.
2. **Library surface from symbols** — mint internal/external nodes from SCIP
   symbols (crate, version, path) with the FQN minter. Gate: no `calls` edge
   whose engine symbol is internal/external is left without a target.
3. **TS/JS engine task** — same, per `tsconfig` project, `allowJs` on.
4. **Svelte through svelte2tsx** — components into the same TS program, spans
   mapped back through the source map. Gate: TS+Svelte `calls` ≥ 99%.
5. **The remainder, one root cause at a time** — `cfg_inactive`, `super`,
   `receiver_any`, the walk's `it.each` naming — until 100%.
6. **Screens** — the coverage line reads placed / missed-by-reason; nothing is
   counted as "unplaced" without its reason.

Then `references` and `imports` through the same join.
