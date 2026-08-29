# 巻 · Observatory · Traceability

**Segment:** 03 · Observatory — daily use
**Route:** `/traceability`
**Source mockup:** [`lib/observatory/traceability.jsx`](../../mockups/Sensei/lib/observatory/traceability.jsx) → `ObsTraceability`
**App file:** `app/src/routes/(observatory)/traceability/+page.svelte`

## Purpose

Traceability is the doc-drift dashboard across every project.
Answers *"where are my docs lying?"* Rows are drift items with
confidence scores; the user reviews, applies auto-fixes for
unambiguous renames, or drills into an Expected-vs-Actual diff.

The [[pipeline/traceability]] pipeline owns the data. This screen
is the browsable view.

Kanji is 巻 — *scroll*.

## Data invariants

- `GET /api/traceability` returns drift items across all
  projects with filter query params (`project`, `confidence`,
  `state`).
- Each row: `doc_path`, `line_number`, `mentioned_identifier`,
  `expected` vs `actual`, `confidence`, `suggestion`, `state`.
- Suggestions come from [[pipeline/narration-cache]] with kind
  `drift_fix`.

## Signals shown

| Element | Value |
|---|---|
| Project filter | all / per-project chips |
| Confidence filter | high / medium / low |
| State filter | open / fixed / resolved_auto / dismissed |
| Search | filter by doc path or identifier |
| Row | project · doc path · line · identifier · confidence chip · state chip |
| Expand: expected-vs-actual diff | side-by-side or inline diff view |
| Actions | Apply fix (when suggestion available) · Dismiss · Investigate |

## Done gate

- Every open drift item on any project appears; the visible
  count equals `select count(*) from sensei.drift_items where
  state = 'open'`.
- Expected-vs-actual diff renders both signatures when both are
  populated.
- Apply-fix on unambiguous renames (git-follow_count >= 3 AND
  name_similarity >= 0.85) rewrites the doc file and advances
  the row to `resolved_auto` on the next scan.
- Confidence chip renders `green` iff `confidence >= 0.8`,
  `amber` for `0.5–0.8`, `grey` below.
- Dismissed rows suppress on subsequent scans by signature.

Optional check:
```
curl -s http://localhost:7744/api/traceability \
  | jq '.items | group_by(.confidence) | map({conf: .[0].confidence, n: length})'
```

## Wrong gate

- **Row shows no expected AND no actual.** Useless card.
- **Apply-fix rewrites a doc despite weak evidence.** Auto-fix
  threshold too loose.
- **Dismissed rows re-appear on the next tick.** Suppression
  broken.
- **Confidence chip green for a low-confidence row.**

## Related

- [[pipeline/traceability]] — data source
- [[screen/project-traceability]] — scoped variant
- [[screen/project-overview]] — stat consumer
