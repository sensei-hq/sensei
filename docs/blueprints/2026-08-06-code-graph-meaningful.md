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
unique and every call names exactly one target, so resolution should be
*complete* — "unresolved" is not an acceptable end state (it's only the honest
interim `2c520f2d` uses while bare-name matching is in place). The single
legitimate exception is **true dynamic dispatch** (trait objects / `dyn`, fn
pointers, reflection) — unresolvable even for a compiler, and rare; those stay
honestly unresolved. Everything else must resolve.

Node *identity* is already unique (`folder, file_path, kind, name, parent_id,
line_start`). The defect is purely **call resolution matching by bare name**. The
correct model is FQN→FQN matching, built from the parser's AST — and to reach
*complete* resolution the parser must qualify the call SITE, not just the
definition:

- **Definition FQN** — each symbol gets a qualified name from its AST context:
  the enclosing module/`impl`/type path + name, e.g.
  `crate::watcher::root_watcher::RootWatcher::new`. At minimum `<impl-type>::<name>`
  disambiguates every adapter method. Store as `nodes.fqn` (new column) *and*
  attach the method to its enclosing type via `parent_id` — this is the deferred
  **D5c symbol nesting**; it also fixes the "no crate/module grouping" complaint
  for free (the tree gains `impl`/type containers).
- **Call-site qualifier capture** — the parser reads the full AST, so it can
  qualify almost every call. Tiers, most-static first:
  - associated call `Foo::new()` → `scoped_identifier` → target `Foo::new`
    (covers most `new`/`default`/`from`);
  - `self.method()` / same-`impl` call → the enclosing `impl` type;
  - local free call `foo()` → the module/use scope;
  - `x.method()` on a local → **light intra-function type tracking**: record each
    `let x = <expr>` binding's type where the RHS reveals it (`Foo::new()`,
    `Foo { .. }`, a typed param/field, an annotated `let x: Foo`), then qualify
    `x.method()` as `Foo::method`. This is NOT a full type checker — just a
    single-pass binding→type map per function, which covers the overwhelming
    majority of real method calls.
  - **only** a receiver whose type is genuinely dynamic (`dyn Trait`, a returned
    boxed trait object with no annotation) stays unqualified → honestly
    unresolved. This should be a small tail, not a third of the graph.
- **Resolution** — match the qualified call against `nodes.fqn` (exact), scoped
  within-crate then project-wide. The unique-name guard (`2c520f2d`) remains only
  as the fallback for a call the parser couldn't qualify — and the whole point of
  this work is to shrink that fallback set toward zero. Never fabricate: a call we
  cannot qualify AND cannot uniquely name stays unresolved, but that set should be
  tiny (true dynamic dispatch), not the adapter methods.

**Why this is the right cut:** it turns the adapter methods from *wrong* edges
(or dropped edges under the interim guard) into *correct* edges, and the same AST
work (enclosing-type capture) delivers the structural containers the UI needs.

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
2. **FQN + D5c symbol nesting** — parser emits enclosing-type containers +
   `nodes.fqn`; resolver matches FQN, keeps the unique-name guard as fallback.
   (The largest chunk; makes edges correct AND adds code structure.)
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
