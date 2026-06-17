---
title: Persist all produced node kinds (fix silent enum drops)
date: 2026-06-16
status: approved
---

# Persist all produced node kinds (fix silent `node_kind` enum drops)

## Problem

`NodeKind::as_str` (`crates/senseid/src/types.rs:72-89`) emits eight strings —
`repo`, `code-group`, `doc-group`, `module`, `struct`, `component`, `hook`,
`doc`, `extension` — but the `node_kind` enum
(`database/ddl/enum/sensei/node_kind.ddl`) defines only sixteen values and
**excludes** `struct`, `component`, `hook`, `doc`, `extension`, `repo`,
`code-group`, `doc-group` (`module` *is* present).

When `upsert_node` (`crates/senseid/src/db/pg_store.rs:525`) binds
`$2::sensei.node_kind` with one of the missing strings, Postgres raises
`invalid input value for enum`, the call returns `Err`, and the error is
**swallowed by `.ok()`** at the persist sites (`process.rs:614` and siblings).
The node is silently dropped — not even recorded in `index_errors`.

### Confirmed impact (live `sensei` DB, 2026-06-16)

Node-kind distribution shows only: `file, method, function, const, class,
module, interface, type, enum`. At **zero rows**: `doc`, `struct`,
`component`, `hook`, `section`, `extension`, `enum_variant`, `package`.

Edge-kind distribution: `imports, extends, calls` only. **`covers` /
`references` = 0**, so doc↔code traceability, the `doc_coverage` view, the
Traceability screen, and doc-drift have no data at all.

The producers are all real and active:
- `doc` + `extension` ← `crates/senseid/src/indexer/doc_indexer.rs` (every
  markdown/README + marketplace item).
- `struct` ← C (`c_lang.rs`), Kotlin data classes (`kotlin.rs`), Swift (`swift.rs`).
- `component` + `hook` ← Svelte (`svelte.rs`), Vue (`vue.rs`), TS (`typescript.rs`).
- `repo` / `code-group` / `doc-group` ← **no producers found** (defined in
  `types.rs`, never constructed).

## Goal

Every node kind the indexer actually produces persists, and the downstream
doc / traceability / component features receive data. Make a future kind↔enum
mismatch impossible to introduce silently.

## Scope decisions (approved)

- **Represent the five produced-but-dropped kinds as first-class enum values**
  (`doc`, `extension`, `struct`, `component`, `hook`) — not remapped. The data
  model already presupposes real `doc` nodes (`doc_type`/`doc_category`
  columns, `covers`/`references` edges, the `doc_coverage` view), and
  `component` is a distinct, queryable concept for a Svelte/Vue-heavy target.
- **Include the production backfill** in this work (clear `scan_state` +
  rescan the live roots), run under explicit in-chat authorization.

## Design

### 1. Schema — `database/ddl/enum/sensei/node_kind.ddl`

Add `struct`, `component`, `hook`, `doc`, `extension`, **appended at the end**
of the enum (do not reorder existing values). Appending lets `dbd` emit simple
`ALTER TYPE … ADD VALUE` migrations; reordering would force a type recreate,
which fails against dependent columns. **Verification step:** confirm `dbd`'s
diff output is `ADD VALUE` (not a recreate) before deploying to prod. Apply via
`dbd deploy` / `dbd apply` — never `dbd combine`.

### 2. Code

- **Stop the silent swallow.** Replace the `.ok()` that discards `upsert_node`
  results (`process.rs:614` and the sibling node/edge persist sites) with an
  error-logging path (`tracing::warn!` with node kind + name + file context).
  Remains non-fatal — one bad node must not abort a scan — but it must be
  visible.
- **Embedding allowlist.** Add `doc`, `struct`, `component`, `hook`,
  `extension` to the kind filters in `nodes_without_embeddings`
  (`pg_store.rs:556-557`) and the duplicate/backfill query (`:647-648`).
  `section` is already present. Embedding `doc` enables semantic doc search and
  underpins `covers` edges; the code-symbol kinds support dedup/search.
- **Remove the three dead variants** (`Repo`, `CodeGroup`, `DocGroup`) from
  `NodeKind` in `types.rs`, including their `from_str` arms — pending a final
  confirmation that nothing constructs or matches them.

### 3. Tests (TDD — written red first)

- **Guard unit test (fast, no DB):** every `NodeKind::as_str()` output (and
  every `SymbolKind → NodeKind` mapping) is a member of the enum value set
  parsed from `node_kind.ddl`. Fails today; permanently couples code↔schema and
  is exactly what would have caught this bug.
- **Integration test (`sensei_test`):** `upsert_node` with `kind = doc` (plus
  `struct`, `component`) persists and round-trips. Fails today.
- Existing language-adapter tests asserting `SymbolKind::Struct/Component/Hook`
  are produced stay green.

### 4. Backfill (production — run under authorization)

After merge + `dbd deploy` + `make install-debug` + `brew services restart
sensei`:
1. Clear `sensei.scan_state` for the registered watch roots (incremental,
   content-hash scan won't otherwise re-emit the now-valid nodes).
2. Rescan the live roots.
3. **Verify:** re-run the kind-distribution and `covers`/`references` queries;
   confirm `doc`/`struct`/`component`/`hook` > 0 and `covers`/`references` > 0.

### 5. Sequence

red guard test → DDL enum (append) + verify `dbd` diff → embedding allowlist →
silent-swallow logging → remove dead variants → zero-errors (clippy + fast
suite + `sensei_test` integration) → commit on `develop` → `dbd deploy` prod +
reinstall + restart → backfill + verify.

## Out of scope (separate follow-ups)

- **Codebase-wide silent-error audit** (directed): find and fix other places
  that discard errors without logging (`.ok()`, `let _ =`, empty catches,
  masking `unwrap_or_default`). This task fixes the `upsert_node` instance; the
  audit is the general sweep.
- `enum_variant` non-emission (Rust enums produce 0 variant nodes — a
  *non-emit*, not a drop).
- The unpersisted IR path, calls-edges-Rust-only, `@rokkit/*` library identity,
  session capture (#31).
