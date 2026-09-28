# 紋 · Project window · Patterns

**Segment:** 04 · The project window
**Route:** `/project/[id]/patterns`
**Source mockup:** [`lib/project/project-pages.jsx`](../../mockups/Sensei/lib/project/project-pages.jsx) → patterns pane
**App file:** `app/src/routes/project/[id]/patterns/+page.svelte`

## Purpose

Patterns cover four families (see [[pipeline/patterns]] for the
full model):

- **Design patterns** — classical OO / architectural patterns in
  use (adapter, plugin, observer/subscriber, trait, strategy,
  factory, repository, decorator).
- **Custom patterns** — project- or team-specific shapes that
  emerged (e.g. sensei's `AssistantAdapter` trait pattern).
- **Anti-patterns** — duplication, spaghetti coupling, god
  objects, broken layering, dead code.
- **Optimization opportunities** — n+1 queries, sync-on-hot-path,
  missing indexes / caches.

This screen is where the user reviews detected patterns per
family, sees evidence, and promotes design/custom patterns to
**rules** so future assistant work adheres to them. Promoted
patterns feed [[pipeline/governance]] as enforceable rules — the
assistant sees them at session start via `get_patterns` /
`get_pattern_for` and follows the established shape instead of
reinventing.

Kanji is 紋 — *pattern / crest*.

## Data invariants

- `GET /api/patterns?project=<id>` returns:
  ```json
  { "patterns": [
      { "id": "…", "kind": "emerging|promoted|anti_pattern",
        "title": "…", "why": "…",
        "instances": N, "confidence": 0..1,
        "ftr_delta_observed": float,
        "example_snippets": ["…"],
        "sessions": ["…"],
        "state": "detected|promoted|dismissed" }, … ] }
  ```
- Reads from `inference.detected_patterns` +
  `sensei.promoted_patterns`.
- Every pattern's title / why come through
  [[pipeline/narration-cache]] with `kind = pattern_title` /
  `pattern_why`.

## Signals shown

| Element | Value |
|---|---|
| Family filter | design / custom / anti / opt / all (default: design + custom, since those constrain new work) |
| State filter | detected / promoted / dismissed |
| Pattern row | family kanji · title · instances · confidence chip · ftr-delta chip |
| Expand | why + example snippets + evidence sessions + list of instance nodes |
| Actions on design/custom | Promote to rule · Rename · Dismiss |
| Actions on anti / opt | Investigate · Dismiss (Promote makes less sense — anti-patterns become rules by their inverse) |
| Ladder picker (on Promote) | picks target ladder + priority (see [[pipeline/governance]]) |

## Done gate

- Detected patterns render with real instance counts and
  ftr-delta from observation.
- Promote action creates a `sensei.rules` row with
  `source: promoted:pattern:{id}` at the chosen ladder + priority.
- Dismiss suppresses re-detection until materially different
  evidence.

## Wrong gate

- **Ftr-delta chip green for a negative observed delta.**
  Anti-pattern candidate mislabeled as positive.
- **Promote picks default ladder without letting user choose.**
  Ladder picker skipped.
- **Dismissed pattern re-detects immediately.** Suppression
  broken.
- **Example snippet extract wrong.** Snippet extraction pulled
  from wrong file.

## Related

- [[pipeline/insights]] — patterns are a rec source
- [[pipeline/governance]] — where promoted patterns land as rules
- [[pipeline/memory]] — memory ↔ pattern relationship (same
  evidence, different form)
- [[screen/observatory-insights]] — patterns land in Soon column
