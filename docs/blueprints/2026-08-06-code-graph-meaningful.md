# Blueprint — make the code graph meaningful (resolution · structure · API · UI)

> Status: **design / for review**. Follow-up to the code-graph indexing ship,
> from live-app feedback (2026-08-06): the graph "looks like a sea of circles with
> no connections and no organisation." Root-caused to three layers — wrong
> *edges*, a view that ignores *structure*, and API↔data *disconnects*. One
> honest interim fix already landed (`2c520f2d`, resolution ambiguity guard); the
> rest is designed here. No further code until reviewed.

## Symptom → root cause (grounded on the live sensei graph)

| Symptom in the app | Root cause | Layer |
|---|---|---|
| `new` connected to hundreds of nodes; ~1/3 of edges suspect | **bare-name call resolution** — 1,727 of 5,394 resolved calls point at an *ambiguous* name; adapters (`load`×50, `parse`×18, `GET`/`POST`, `run`, `new`×23) collapse onto one arbitrary node | **resolution** |
| "Relations 0" on the community view | community view draws one circle/community with **no inter-community edges** — nothing aggregates node-edges → community-edges | **API + UI** |
| Sea of circles, `Nodes==Communities==1791`, no grouping | Atlas calls `graph/nodes` + `communities/info` only — **never `/tree`**, so no repo→subtree→crate→module structure, no docs-vs-code split | **UI** |
| "0 EXPORTS" | `is_exported` is `false` for all 6,547 symbols — the parser never sets it; `call_flow` filters on it | **data** |
| marketplace/homebrew missing from scope | git-*subtree*-merged dirs have no nested `.git`; `detect_git_subtrees` misses them | **data** |
| "showing 200 of 15364 · by connections", no controls | hard cap, no sidebar filters | **UI** |

## Fix 1 (the big one) — fully-qualified names for correct resolution

**Goal: no ambiguity, no unresolved.** With a proper FQN every definition is
unique and every call names exactly one target. **"Unresolved" is not a valid
state at all** — a call always references *something*; if we can't point it at a
node, that's a modelling failure, not an honest empty. The interim guard
(`2c520f2d`) that leaves ambiguous calls unresolved is therefore also wrong as an
end state — it's a temporary artifact of the old name-matching model, and this
design **removes it** along with the separate `resolve_edges` pass.

### The model — get-or-create nodes by FQN (a symbol table)

Adopt the SCIP/LSIF *moniker* model: **every symbol has a stable FQN, and both a
definition and a reference get-or-create the node for that FQN.** There is no
"resolve later" step — the edge is linked to a real node at emit time.

1. **Every AST reference emits an FQN.** Processing a file, for each tree item —
   a definition (`fn`, `impl fn`, `struct`, …) AND each reference (a call target,
   a type mention, an import) — compute its canonical FQN and
   **`upsert_node_by_fqn(fqn) → node id`**: fetch the existing node or create a
   *stub* (kind + fqn only), returning its id. The edge is emitted node→node
   immediately. A stub carries just enough to exist; when the file that *defines*
   that FQN is processed, the same node is **enriched** (signature, body,
   line span, is_exported, doc) — a get-or-create keyed on FQN, filled in on
   definition. Order-independent: a call seen before its definition creates the
   stub; the definition later fills it (D3 upsert-then-prune already gives us
   idempotent enrichment).
2. **External symbols get FQN nodes too — a `lib` group.** The ONLY legitimate
   reason a target isn't an internal definition is that it lives in an external
   library (`serde::de::from_str`, `std::vec::Vec::new`). Those get-or-create a
   node under a **library namespace** (`kind='lib_symbol'`, grouped by crate/pkg),
   so a call to a dependency resolves to the lib node — not dropped, not
   unresolved. This also gives the graph a real "what we depend on and how much"
   signal for free.
3. **No `resolve_edges`, no `target_name` limbo.** Edges are `source_id →
   target_id` from the start (target is the get-or-created FQN node). The whole
   unresolved/resolve machinery — and the ambiguity guard — goes away.

### The engine — canonical, cross-file-consistent FQN

The one hard requirement is that a **definition and a reference compute the SAME
FQN**, or they'd create two separate stubs. That needs per-language **name
resolution** (the real work, LSP-grade but bounded):

- **Definition FQN** from AST context: crate/module path + enclosing `impl`/type
  + name → `crate::widget::Widget::new`. (Also gives `parent_id` nesting = the
  deferred **D5c** structure the UI wants.)
- **Reference FQN** by resolving the call's path against the file's scope:
  - explicit path `Foo::new()` → expand `Foo` via the file's `use`/imports to its
    canonical path (`use crate::widget::Widget; Widget::new()` → `crate::widget::Widget::new`);
  - `self.method()` / same-`impl` → enclosing type;
  - local `foo()` → module scope;
  - `x.method()` on a local → light per-function binding→type map (`let x =
    Foo::new()`, typed params/fields, `let x: Foo`) → `Foo::method`;
  - a path that resolves to an imported *external* crate → the `lib` FQN (case 2).
  - Truly dynamic dispatch (`dyn Trait` with no static type) is the only residual —
    and even then we emit a reference to the *trait method* FQN (still a node),
    not nothing.

So "no ambiguity" (FQN is unique) and "no unresolved" (get-or-create always
yields a node, internal or lib) both hold. The engineering is the name-resolution
that makes the two sides agree on the FQN — per language, incrementally
improvable, and each increment shrinks stubs/lib-fallbacks toward the true set.

**Why this is the right cut:** it's the correct code-intelligence model, it makes
edges correct instead of guessed, it makes external dependencies first-class, and
the same FQN/enclosing-type work delivers the crate/module/impl structure the UI
needs — one change, three of the symptoms fixed.

### Layering — shared FQN core, per-language resolvers (decided)

The FQN and the symbol-table machinery are **shared and language-agnostic**; only
the *name resolution that produces an FQN* is per-language:

- **Shared core:** a canonical FQN moniker (structured, e.g.
  `lang · package/crate · module-path · Type · member`, encoded to one stable
  string), `upsert_node_by_fqn`, the `lib` namespace, node→node edge emit, and the
  storage/query layer. One identity + edge model for every language, so a
  cross-language reference in a polyglot repo still merges on the same FQN.
- **Per-language (each `LanguageAdapter`):** walk the AST and emit definitions +
  references already tagged with the shared FQN, applying that language's
  import/scope/enclosing-type/binding-type rules. Adapters plug a `resolve_fqn`
  into the shared core; they never own identity or edge logic.

This mirrors SCIP (common moniker format + language-specific indexers): maximal
reuse, and a new language only implements its resolver.

## Fix 2 — structure in the view (consume `/tree`) + community edges

- **Atlas must consume `GET /api/graph/{repoId}/tree`** (already built, Phase 7)
  and render the nesting: `repo → subtree → workspace_member/crate → module →
  file → {class/impl → method} · doc → section`. The hierarchy is already in the
  data (`folders.kind` + `nodes.parent_id`); the view just ignores it. Top-level
  split **docs vs code**; within docs, group by `doc_type`; within code, by the
  folder/crate spine.
- **Community view needs inter-community edges.** Add a retrieval that aggregates
  node-level edges to **community-level** (`src.community_id → tgt.community_id`,
  weight = count), so communities render as a connected map, not confetti. Best
  as a DB view (below). Also expose community membership on drill-down.
- **Sidebar controls** (right panel): filter by node kind, by docs/code, by
  subtree/crate; choose layout (structural tree vs community vs call-flow); a
  cap/paging control with an explicit ordering label ("top N by degree") instead
  of a silent 200.

## Fix 3 — DB views to simplify the API (leverage the existing pattern)

The `sensei` schema already has **19 views** — extend the pattern so the graph
handlers stop doing ad-hoc in-Rust assembly and read a stable, consistent shape.
Proposed views (in `database/ddl/view/…`):

- `sensei.graph_edges_resolved` — edges joined to source/target FQN + kind,
  resolved-only, for the node/call view (one source of truth for "an edge").
- `inference.community_edges` — the node→community edge aggregation above
  (community_id pairs + weight), so `communities/info` can return real relations.
- `sensei.folder_tree` — the folder hierarchy with kind/role/parent for `/tree`
  (moves the recursion into SQL; the handler just shapes JSON).
- `sensei.node_exports` — exported symbols per file (once `is_exported` is fixed),
  so `call_flow` reads a view instead of an O(n²) in-handler filter.

This directly answers the "API seems disconnected" concern: a view is the
contract, the handler is a thin projection, and the UI can't drift from the data.

## Data-bug fixes (backend prerequisites)

- **`is_exported`** — set it in each language adapter from visibility (`pub` in
  Rust; `export` in TS/JS; `__all__`/leading-underscore in Python). TDD per
  adapter. Unblocks exports everywhere.
- **Subtree detection** — `detect_git_subtrees` only finds nested `.git`.
  git-*subtree* merges (marketplace/homebrew) are recorded in `.git/config`
  (`[remote] … ` + subtree merge refs) or a `.gittrees`/prefix marker. Detect via
  the repo's configured subtree prefixes (or a `folders_to_watch.subtree_prefixes`
  knob) and register `kind='subtree'` (the D5a write path already exists).

## Proposed build sequence (each its own reviewed chunk)

1. **Data bugs** — `is_exported` + subtree detection (small, testable, immediate
   UI wins: exports populate, scope dropdown shows subtrees).
2. **FQN symbol-table model** — the core. Add `nodes.fqn` + `upsert_node_by_fqn`
   get-or-create; the parser emits definitions AND references as FQN nodes with
   node→node edges (retire `resolve_edges`, `target_name` limbo, and the interim
   guard `2c520f2d`); external targets → `lib` nodes; per-language name resolution
   (imports/scope/enclosing-type + light binding→type) to canonicalise FQNs, plus
   D5c enclosing-type nesting for structure. Build the name-resolution per language,
   Rust first; each increment shrinks the stub/lib-fallback set.
3. **DB views** — the four views above; repoint the handlers to them.
4. **Community-edge aggregation** — the view + `communities/info` returning
   relations.
5. **Atlas structural view** — consume `/tree`, render nested clustering +
   docs/code split + sidebar filters. (Frontend; you verify visually.)

## Open questions

- FQN scheme: full crate path (`crate::mod::Type::method`) vs `file::Type::method`?
  Full path is more correct cross-crate but needs module resolution; file-scoped
  is simpler and unambiguous within a repo. (Lean: `Type::method` within a file's
  crate, store the raw scoped path the parser sees.)
- Cross-crate calls (a call in crate A to `B::thing`): resolve across the whole
  project scope by FQN, or only within-crate first? (Lean: within-crate, then
  project-scope by FQN.)
- How much type inference is worth it for `x.method()` receivers, or accept those
  stay unresolved? (Lean: accept unresolved — the associated-call + same-impl
  cases already cover the bulk.)
