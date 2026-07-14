# 覚 · Project window · Memories

**Segment:** 04 · The project window
**Route:** `/project/[id]/memories`
**Source mockup:** [`lib/project-lite-panes.jsx`](../../mockups/Sensei/lib/project-lite-panes.jsx) → `ProjMemoriesLite`
**App file:** `app/src/routes/project/[id]/memories/+page.svelte`

## Purpose

Project-scoped memory curation with a **ready-to-share lane**.
Same anatomy primitive as [[screen/observatory-memories]] but
filtered to this project's memories, and adds an entry point for
promoting project-generalisable memories up the scope ladder (see
[[pipeline/memory]] promotion ladder).

Kanji is 覚 — *awareness / to remember*.

## Data invariants

- `GET /api/memories?project=<id>` returns memories with
  `scope_project_id == id` OR relevant user/org-scope overlays.
- Additional field: `generalised: bool` — sensei's assessment
  of whether the memory has been rewritten stack-agnostic and
  is ready to share.
- `POST /api/memories/{id}/generalise` triggers the
  generalisation step (model rewrites the memory to remove
  project-specific references); response includes both
  original and generalised text.

## Signals shown

Same as [[screen/observatory-memories]] plus:

| Element | Value |
|---|---|
| Ready-to-share hero card | count of memories generalised + ready + "review next batch" action |
| Rail badge: `送` | on memories queued for a batch |
| Anatomy strip addition | `generalised: yes/no` chip |
| Widen-scope submenu | now the primary action for project-scope memories that pass generalisation |

## Done gate

- Rail lists only this project's memories + inherited overlays.
- Ready-to-share card counts memories with `generalised == true`
  AND `queued_for_batch == true`.
- Generalise action rewrites cleanly and displays both versions
  side-by-side for user confirmation.
- Widen to `user` scope from this screen honors the
  [[pipeline/memory]] scope contract.
- Client-project memories default to the `dereferenced` share
  path when widening beyond project scope (see
  [[pipeline/dojo-lifecycle]] attribution).

## Wrong gate

- **User-scope memories from other projects contaminate this
  view.** Scope query too broad.
- **Generalise reproduces the original text verbatim.**
  Insight-copy model wasn't reached OR the generalisation
  prompt didn't apply.
- **Client-project memory offers `named` attribution on widen.**
  Confidentiality regression.
- **Ready-to-share count differs from queued items.** Two
  derivations of the same set.
- Every failure mode inherited from
  [[screen/observatory-memories]] applies.

## Related

- [[pipeline/memory]] — LLM-primary consumer + promotion ladder
- [[pipeline/dojo-lifecycle]] — attribution rules for widen
- [[screen/observatory-memories]] — the multi-project peer
- [[screen/observatory-share-review]] — batch review target
