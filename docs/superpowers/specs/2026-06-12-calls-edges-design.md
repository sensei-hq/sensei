# Design — Populate `calls` Edges (Rust, function-granular)

> Issue: #57 — *Code graph: indexer emits no `calls` edges → get_callers/get_callees/call-flow are empty*
> Date: 2026-06-12
> Status: approved (design), pending implementation

## Problem

`sensei.edges` contains only `imports` (~34.5k) and `extends` (~19.9k) edges — **zero
`calls` edges** across ~81k indexed nodes. As a result the `call_graph` view has no call
rows, and the navigation tools `get_callers` / `get_callees` / `call_flow` (MCP + API)
return `[]` for every symbol. These are the tools that should let an agent traverse the
codebase faster than `grep`; today they are dead. Of 11,948 captured tool events, the
graph-navigation tools logged 0 useful calls.

## Root cause

The end-to-end `calls` pipeline already exists and is idle — only the first link is missing:

| Stage | File | Status |
|-------|------|--------|
| Emit unresolved `calls` edges from `result.unresolved_calls` | `tasks/handlers/process.rs:636` | ✅ exists |
| Map `parsed.edges` → `unresolved_calls` | `tasks/processors/code.rs:73` | ✅ exists |
| Resolve `calls` target by name | `tasks/handlers/resolve.rs:52` | ✅ exists |
| `edge_kind` enum has `'calls'` | `database/ddl/enum/sensei/edge_kind.ddl` | ✅ exists |
| `call_graph` view (resolved + `unresolved_target`) | `database/ddl/view/sensei/call_graph.ddl` | ✅ exists |
| `get_callers_by_name` / `get_callees_by_name` | `db/pg_store.rs:769,782` | ✅ exists |
| MCP tools + `/api/graph/*` routes | `crates/mcp`, `api/handlers/codebase.rs` | ✅ exists |
| **Extract call-sites → `ParsedFile.edges`** | `languages/rust_lang.rs:38` | ❌ **hardcoded `edges: vec![]`** |

`rust_lang.rs::parse()` walks the AST for symbols and imports but never recurses into
function bodies, so it never extracts call expressions. `ParsedFile.edges` is always empty
→ `unresolved_calls` is always empty → no `calls` edges are ever written.

A second, related gap: the existing emission sources **every** edge from the *file* node
(`process.rs:638` passes `fid`, dropping `call.caller_name`). Even once call-sites are
extracted, file-granular sourcing would leave `get_callees` dead (it matches on
`source_name`, which would always be a file path, never a function name). This design fixes
both: call-sites are extracted **and** sourced from the caller function node.

## Scope

- **Rust adapter only**, this increment. Python / Svelte / TS adapters are follow-ups; they
  adopt the same contract (below) with their own denylists.
- Everything downstream of `ParsedFile.edges` is reused unchanged — no DDL changes, no
  resolve/view/tool changes.

## Decisions (locked during brainstorming)

1. **Extract free, path, and method calls** — `foo()`, `m::bar()`, `recv.method()`. Method
   extraction is required: the canonical verification target `insert_memory` is reached via
   `pg.insert_memory(…)` (`federation/mod.rs:164`), a method call.
2. **Per-adapter denylist (decision B)** — skip a small set of ubiquitous std/library
   methods to keep unresolvable noise out of `edges`. The list is a private `const` in each
   adapter; no shared/global list, no trait method. Language-specific knowledge stays with
   the language.
3. **Defer the `get_callers` unresolved quick-win (decision A)** — keep `get_callers`
   resolved-only for precision. The verification target resolves anyway (`insert_memory` is
   an indexed method node). Revisit only if useful unresolved callers are found missing.
4. **Function-granular edges (decision A)** — call edges are sourced from the **caller
   function's node**, not the file node. This is what makes *both* `get_callers` and
   `get_callees` work as advertised.
5. **Caller attribution by `(name, line_start)`** — same-named methods across `impl` blocks
   are legal in Rust, so the caller→node lookup keys on `(name, line_start)`. This is exact
   in practice — it would only collide if two same-named definitions began on the *same
   source row* (impossible under normal formatting; the DB node identity is finer, including
   `kind` and `parent`). The key is drift-free because the adapter computes one `caller_line`
   variable and uses it for both the symbol's `line_start` and the emitted edge's
   `caller_line`. Falls back to the file node only when a call has no enclosing named symbol.
6. **Callee names are bare / last-segment** — `Type::assoc()` → `assoc`, `recv.method()` →
   `method`. Nodes are stored under bare names, so a bare callee maximizes resolution
   matches; a qualified callee would never resolve. Same-named callees resolving to one
   node is the inherent limitation of name-based resolution (full resolution needs type
   inference — out of scope per #57).

## Cross-adapter contract (the reusable pattern)

This is the pattern every language adapter follows for call extraction. Writing it down so
follow-up adapters copy it rather than reinventing:

> An adapter's `parse()` populates `ParsedFile.edges: Vec<ParsedEdge>`, where each
> `ParsedEdge` describes one **deduplicated** call-site as language-agnostic *edge intent*:
> the caller's symbol identity (`caller_name`, `caller_line`), the bare `callee_name`, and
> optional `callee_file`. The adapter owns *all* language-specific logic: which AST nodes
> are calls, how to normalize callee names, which calls to drop (its own denylist), and
> dedup. **No call/edge logic lives in the scan pipeline** — `process.rs` only persists the
> adapter's edge intent generically.

## Hard constraint

`edges.source_id` is `NOT NULL`. Node ids are created only inside `process.rs` (after
`upsert_node`); the resolve phase fills only the nullable `target_id`. Therefore the edge
**source must be resolved at insert time** — it cannot be pushed into the adapter (no node
ids there) nor deferred to resolve. This is why `process.rs` gains a small (generic) source
lookup; it is unavoidable, not call-specific.

## Components & changes

### 1. `crates/senseid/src/languages/rust_lang.rs` — extraction (the real work)

- Extend the manual `walk_nodes` descent (the file's existing idiom; no tree-sitter
  `Query` is introduced). When the walk enters a `function_item`, descend into its body and
  collect call-sites, tracking the enclosing function's `(name, line_start)` as the caller.
- **Call node**: `call_expression`. In tree-sitter-rust, `recv.method()` is a
  `call_expression` whose `function` field is a `field_expression`, so free, path, and
  method calls are all the same node kind. Exact node/field names verified against the AST
  tool during implementation.
- **Callee name** from the `function` field:
  - `identifier` → itself (`foo`)
  - `scoped_identifier` (`a::b::c`) → last segment (`c`)
  - `field_expression` (`x.m`) → the `field` (`m`)
  - `generic_function` (turbofish, `foo::<T>()`) → recurse into its inner `function` node, yielding the bare name
  - `macro_invocation` (`println!`, `vec!`, …) → **skipped** (not `call_expression`; noise).
- **Denylist** — private `const RUST_CALL_DENYLIST: &[&str]` (~20 entries):
  `clone, unwrap, expect, into, to_string, to_owned, as_str, as_ref, iter, into_iter, map,
  unwrap_or, unwrap_or_default, ok, len, is_empty, push, collect, next, borrow, borrow_mut,
  lock, read, write`. Callees in the list are skipped at extraction.
- **Dedup** per `(caller_name, caller_line, callee_name)` within the file (`HashSet`).
- Emit `ParsedEdge { caller_name, caller_line, callee_name, callee_file: None }`. `parse()`
  returns these in `edges` instead of `vec![]`.

### 2. `crates/senseid/src/types.rs` — plumbing

- `ParsedEdge` gains `caller_line: u32` (alongside existing `caller_name`, `callee_name`,
  `callee_file`).

### 3. `crates/senseid/src/tasks/processors/types.rs` + `processors/code.rs` — plumbing

- `UnresolvedCall` gains `caller_line: u32`.
- `code.rs:73` maps it through (it already copies `caller_name` / `callee_name`).
- Mechanical only.

### 4. `crates/senseid/src/tasks/handlers/process.rs` — generic function-granular sourcing

- In the symbol-insert loop (`619-626`), capture each upserted symbol id into
  `HashMap<(String, i32), Uuid>` keyed by `(name, line_start)`. `upsert_node` already
  returns the id; today it is discarded for symbols.
- In the call-edge loop (`636-640`), source each edge from
  `map.get((call.caller_name, call.caller_line))`; fall back to `file_node_id` when absent
  (call with no enclosing named symbol). Everything else unchanged.
- Imports / extends / doc-ref loops are **untouched** (genuinely file-level). No
  language-specific logic enters `process.rs`; the map is generic graph-wiring reusable by
  any future symbol-sourced edge kind.

## Data flow (after change)

```
rust_lang.parse()                 process.process_file()              resolve_edges()
─────────────────                 ──────────────────────              ───────────────
walk fn bodies                    upsert file + symbol nodes          for each unresolved
  → call_expression               build (name,line)→id map              calls edge:
  → normalize callee              for each ParsedEdge:                    match target_name
  → drop denylisted                 source = map[(caller,line)]            against folder nodes
  → dedup                                    ?? file_node_id              → set target_id
  → ParsedEdge{caller,line,         insert_edge(source, target_name,
       callee}                          kind='calls')
```

## Testing (TDD — tests first)

**Unit — `rust_lang.rs`** (`cargo test -p senseid --bin senseid`, no DB):
- free call `foo()` → edge `(caller, "foo")`
- path call `m::bar()` → callee `"bar"`
- method call `x.insert_memory()` → callee `"insert_memory"`
- denylisted `x.clone()` → no edge
- macro `println!("{}", x)` → no edge
- repeated calls to same callee in one caller → one edge (dedup)
- two methods named `new` in separate `impl` blocks → calls attribute to distinct
  `caller_line`s
- nested call `a(b(c()))` → edges for each callee with correct caller
- free function calling a method, and method calling a free function → correct caller/callee

**Integration — `process` / `resolve`** (DB-backed, `sensei_test` @ localhost:5432):
- processing a small Rust file writes `calls` edges whose `source_id` is the **caller
  function node**, not the file node
- `resolve_edges` points an intra-file call to its target node (`target_id` set)
- `get_callers_by_name` / `get_callees_by_name` return function-granular rows

**End-to-end verification** (live daemon, rescan `sensei`):
- `select kind, count(*) from sensei.edges group by 1` → `calls > 0`
- `get_callers(insert_memory)` → returns the calling function in
  `crates/senseid/src/federation/mod.rs`
- `get_callees(<a known caller>)` → returns its callees

## Non-goals / known limitations (stated explicitly)

- **Name-based callee resolution** — same-named callees resolve to one node; no type
  inference / method-receiver resolution. Accepted per #57.
- **Not extracted/resolved**: macros, closures stored as values then invoked, function
  pointers, trait-dynamic-dispatch targets.
- **Calls inside macro argument token-trees** (e.g. `vec![make()]`, `assert_eq!(compute(), 3)`)
  are invisible — tree-sitter exposes macro args as an opaque `token_tree`, not `call_expression`.
- **Nested free fns** (`fn outer() { fn inner() {…} }`): `collect_calls` does not descend into a
  nested `function_item`, so the nested fn's calls are dropped rather than mis-attributed to the
  outer fn (closures, which are `closure_expression`, are still captured under the enclosing fn).
- **Other languages** (Python, Svelte, TS) deferred to follow-up issues; they adopt the
  cross-adapter contract above with their own denylists.
- **No `get_callers` unresolved matching** this increment (decision A).

## Out of scope but worth a follow-up issue

- Per-language adapters populating `calls` edges (one issue per language). When they adopt
  the `(name,line)→id` contract, add a `debug_assert!`/`tracing::debug!` on the file-fallback
  branch in process.rs (non-empty `caller_name` but no map hit) — those adapters compute
  symbol lines and caller lines in separate code paths, so the silent fallback could otherwise
  mask a keying mismatch and quietly regress `get_callees`.
- Optional: revisit the `get_callers` unresolved quick-win if precision proves too narrow.
