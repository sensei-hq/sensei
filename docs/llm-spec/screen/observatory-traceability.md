# 巻 · Observatory · Traceability

**Segment:** 03 · Observatory — daily use
**Route:** `/traceability`
**Source mockup:** [`lib/traceability.jsx`](../../mockups/Sensei/lib/traceability.jsx) → `ObsTraceability`
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
- Suggestions come from [[pipeline/insight-copy]] with kind
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

- Every open drift item on any project appears.
- Expected-vs-actual diff renders both signatures when both are
  populated.
- Apply-fix on unambiguous renames rewrites the doc file and
  advances the row to `resolved_auto` on the next scan.
- Dismissed rows suppress on subsequent scans by signature.

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
