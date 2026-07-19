---
name: Memory-anchoring — design
date: 2026-07-18
status: approved (brainstorm) — implementation plan next
---

# Memory-anchoring — design

The last Operating-model **Phase 1 (Foundations)** sub-unit (spec
[`operating-model.md`](operating-model.md) §3.2 + §9 + §620). Anchors memories to the
**spine slots** so retrieval can be slot-scoped — the foundation of *push not pull*.

## Problem

Memories live in a DB side-store (`sensei.memories`) and are **pulled** — an agent must
call `get_layered_context` / `search` / `context_pack` to see them, so it skips them
(§3.2: "the real fix for why the agent skips memories"). The fix (§3.2): the spine **is**
the memory — anchor memories to the doc slots so they surface *at the point of work*.
§620 scopes this Phase-1 unit as *"low risk, high leverage — makes push-not-pull real."*

## Decisions (brainstorm, 2026-07-18)

1. **Anchoring model = slot metadata + slot-scoped retrieval** (not physical
   materialization into files, not a work-time push hook — those are deferred).
2. **Storage = typed columns** on `sensei.memories` (a `spine_slot` enum + a `feature`
   text), not tags and not a join table.
3. **Scope = anchor + heuristic auto-anchor + slot-aware retrieval.** New analyzer
   memories are auto-anchored by a pure heuristic (no LLM); retrieval becomes
   slot-scoped. Auto-inferring the slot from the file/cwd being worked on (the fully
   automatic push) is a deferred follow-up.

## 1. Data model — `sensei.memories` (via `dbd reconcile`, additive)

- **New enum** `sensei.spine_slot`: `vision`, `personas`, `journeys`, `roadmap`,
  `design`, `mockups`, `decisions`, `brief`, `plan`, `tests` — the doc-slot names the
  scaffolders produce (project spine §3.2 + feature dossier).
- **New nullable columns:** `spine_slot sensei.spine_slot`, `feature text`.
- **`feature` disambiguates scope:** `spine_slot=design, feature=NULL` → the *project's*
  design slot; `spine_slot=design, feature='auth'` → *auth's* design slot (maps to
  `docs/features/auth/design.md`). One memory anchors to one slot (nullable = unanchored).
- **Scope rules** (enforced in the write path, not a DB CHECK — keeps it forgiving):
  - project-only slots (`vision`, `personas`, `journeys`, `roadmap`, `mockups`) → `feature` MUST be NULL;
  - feature-only slots (`brief`, `plan`, `tests`) → `feature` MUST be set;
  - `design`, `decisions` → valid at both scopes.
- **Index:** `memories_spine_slot_idx on memories(project_id, spine_slot) where status='active'`.
- Existing columns (`scope`=federation ladder, `category`, `type`, `tags`, `namespace_id`)
  are untouched — `spine_slot` is an orthogonal dimension.

## 2. Acquisition — how a memory gets its anchor

- **Manual (MCP `save_memory` / `propose_memory`, `knowledge.rs`):** accept optional
  `spine_slot` + `feature`; validate against the scope rules; persist. Unknown/omitted →
  NULL (unanchored, allowed).
- **Auto (analyzer generate path, `generate.rs` / `pg_store.rs` INSERT):** set a
  **heuristic default** via a pure `default_slot(category, type) -> SpineSlot`. First cut:
  - `pattern` (category or type) → `design`
  - `convention` → `design`
  - `decision` / `correctness` / `preference` / `continuity` / `question` → `decisions`
  - default project scope (`feature = NULL`).
  So new analyzer memories are anchored with **no LLM** and no gateway dependency. (LLM
  slot-classification is a deferred L2 refinement.)
- **Existing memories:** left NULL (unanchored). A one-shot heuristic backfill is an
  optional follow-up, out of scope here.

## 3. Retrieval — the "push"

- **New `pg_store::list_memories_for_slot(project_id, slot, feature) -> Vec<memory>`** —
  the focused query behind the index.
- **`assemble_context` becomes slot-aware:** an optional `(slot, feature)` hint; when
  present, the slot-anchored memories are surfaced prominently in the assembled context
  (they lead, ahead of the general blend). When absent, behavior is unchanged
  (backward-compatible).
- **MCP surface:** `context_pack` + `get_layered_context` gain an optional `slot` /
  `feature` argument that flows into `assemble_context`. Passing the slot returns that
  slot's memories — the caller (or a later hook) supplies the slot.
- **Deferred:** auto-inferring the slot from the file/cwd being edited (the hook that
  removes the need to pass a slot at all) — that's what makes push *fully* automatic and
  is the natural next unit.

## Units & interfaces (isolation)

| Unit | Responsibility | Interface | Depends on |
|---|---|---|---|
| DDL | the `spine_slot` enum + columns + index | `sensei.memories` schema | dbd |
| `default_slot` (pure) | category/type → default slot | `fn default_slot(category,&type)->SpineSlot` | nothing (pure, unit-testable) |
| write paths | persist slot + validate scope | `save_memory`/`propose_memory` params; analyzer INSERT | `default_slot`, DDL |
| `list_memories_for_slot` + slot-aware `assemble_context` | slot-scoped retrieval | pg_store methods | DDL |
| MCP plumbing | expose `slot`/`feature` on `context_pack`/`get_layered_context` | tool args → handler → assemble_context | retrieval |

## Testing

- Pure `default_slot` mapping — one assertion per category/type → expected slot.
- DDL applies via `dbd reconcile` (enum + columns + index present).
- `list_memories_for_slot` returns only the matching slot/feature's active memories.
- `assemble_context` with a slot hint leads with slot-anchored memories; without → unchanged.
- Write paths persist `spine_slot`/`feature` and reject a scope-rule violation.
- MCP: `context_pack`/`get_layered_context` accept + forward `slot`/`feature`.

## Scope / deferred

**In:** the enum+columns+index, `default_slot` heuristic, write-path anchoring +
validation, `list_memories_for_slot`, slot-aware `assemble_context`, MCP param plumbing.
**Out (own follow-ups):** LLM slot-classification; auto-inject-by-cwd (the hook);
one-shot backfill of existing memories; Dōjō/federation slot propagation.
