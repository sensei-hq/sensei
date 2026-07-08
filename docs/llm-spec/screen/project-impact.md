# 果 · Project window · Impact

**Segment:** 04 · The project window
**Route:** `/project/[id]/impact`
**Source mockup:** [`lib/impact.jsx`](../../mockups/Sensei/lib/impact.jsx) → project variant
**App file:** `app/src/routes/project/[id]/impact/+page.svelte`

## Purpose

Project-scoped impact / verdict view. Same primitive as
[[screen/observatory-impact]] filtered to this project. Focuses on
this project's applied recommendations, memory adoptions, and
their measured effect on this project's FTR.

Kanji is 果 — *result*.

## Data invariants

- `GET /api/impact?project=<id>` returns scoped verdicts and
  regressions.
- Additional field: `project_ftr_before` / `project_ftr_after`
  taken from `sensei.project_ftr_metrics` at the applied /
  measured timestamps.
- Regressions surface as [[screen/observatory-impact]] with the
  same Revert / Keep / Investigate actions.

## Signals shown

Same as [[screen/observatory-impact]] plus a project FTR trend
chart at the top showing the trajectory across accepted
recommendations (annotated markers per apply event).

## Done gate

- Every applied recommendation for this project appears with
  its verdict.
- FTR trend chart annotations align to `applied_at` timestamps.
- Regressions surface with the same distinct nav-entry story.

## Wrong gate

- **Trend chart lacks annotations at apply events.** Applied-rec
  join missing.
- **Cross-project verdicts leak.** Scope regressed.
- Every failure mode from [[screen/observatory-impact]] applies.

## Related

- [[pipeline/impact]] — pipeline
- [[pipeline/ftr]] — trend chart data
- [[screen/observatory-impact]] — multi-project peer
