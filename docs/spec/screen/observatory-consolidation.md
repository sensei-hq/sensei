# 統 · Observatory · Consolidation

**Segment:** 03 · Observatory — daily use
**Route:** `/consolidation`
**Source mockup:** [`lib/observatory/consolidation.jsx`](../../mockups/Sensei/lib/observatory/consolidation.jsx) → `ObsConsolidation`
**App file:** `app/src/routes/(observatory)/consolidation/+page.svelte`

## Purpose

Consolidation is the dedup + merge surface. Sensei clusters similar
memories, similar patterns, similar rules — surfaces "these three
are the same idea; merge them?" or "this is a duplicate of X".
Reduces surface area over time so the memory shelf stays
navigable.

Kanji is 統 — *unify*.

## Data invariants

- `GET /api/consolidation/candidates` returns clusters:
  ```json
  { "clusters": [ { "id": "…", "kind": "memory|pattern|rule",
                    "items": [ … ], "similarity": 0..1,
                    "suggestion": "merge|dedupe|no_action" }, … ] }
  ```
- Each cluster carries a similarity score and a suggested
  action.
- Merge writes back to the source table with the merged
  representative kept; the others archived with
  `merged_into: representative_id`.

## Signals shown

| Element | Value |
|---|---|
| Cluster row | kind chip + item count + similarity chip + summary |
| Expand: item list | each item with content preview + strength/scope |
| Actions | Merge (pick representative) · Dismiss · Keep separate |
| Search / filter | by kind |

## Done gate

- Every candidate cluster with `similarity >= 0.7` surfaces
  with the right kind and member count.
- Merging preserves the strongest member's identity; archived
  members carry `merged_into: representative_id` back-link and
  are hidden from `get_memories` retrieval.
- Dismissed clusters don't re-appear unless a new member
  joins with a materially different signature.
- Similarity chip renders the numeric value (e.g. `0.87`) so
  the user can compare across clusters.

Optional check:
```
curl -s http://localhost:7744/api/consolidation/candidates \
  | jq '.clusters | group_by(.suggestion) | map({s: .[0].suggestion, n: length})'
```

## Wrong gate

- **Two memories from different scopes merged.** Scope
  precedence violated — cross-scope merge should require
  explicit widen first.
- **Merge loses references from the archived members.** Back-
  links needed to preserve traceability.

## Related

- [[pipeline/memory]] — the primary consumer of consolidated memories
- [[pipeline/insights]] — pattern consolidation
- [[pipeline/analyzer]] — schedules consolidation
