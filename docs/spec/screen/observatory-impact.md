# 果 · Observatory · Impact

**Segment:** 03 · Observatory — daily use
**Route:** `/impact` (default view) · `/impact` with `?alert=1` for the regression state
**Source mockup:** [`lib/observatory/impact.jsx`](../../mockups/Sensei/lib/observatory/impact.jsx) → `ObsImpact` (default) + `ObsNegativeAlert` (regression state)
**App file:** `app/src/routes/(observatory)/impact/+page.svelte`

## Purpose

Impact is the receipt log. Every applied recommendation eventually
lands here with a **verdict** (positive / neutral / negative /
insufficient_data) and a **before-after FTR delta**. This is
where the user verifies that "the pair is getting better" isn't
just talk.

The **Regressions** sub-view is a distinct nav entry (per the
journey map's safety-screen story) — negative verdicts must not
hide inside the summary. Every negative verdict is triable
(Revert / Keep / Investigate).

Kanji is 果 — *result*.

## Data invariants

- `GET /api/impact` returns:
  ```json
  {
    "summary": { "positive": N, "neutral": N, "negative": N, "insufficient_data": N, "window_days": 30 },
    "verdicts": [
      { "id": "…", "recommendation_title": "…",
        "applied_at": iso, "measured_at": iso,
        "verdict": "positive|neutral|negative|insufficient_data",
        "before": {…}, "after": {…}, "delta": {…},
        "note": "…", "confidence": 0..1,
        "project": "…", "state": "open|revert|keep|investigate" }, …
    ],
    "regressions_open": N
  }
  ```
- Reads from `sensei.impact_verdicts` +
  `sensei.impact_regressions` per [[pipeline/impact]].
- Regression alerts include an `investigate_snoozed_until`
  timestamp when the user chose to defer.

## Signals shown

| Element | Value |
|---|---|
| Summary strip | 4 chips: positive · neutral · negative · insufficient (30d window) |
| Regressions nav entry | small red pill with unresolved count; always visible |
| Verdict card | rec title · verdict chip · delta · before/after mini chart · note |
| Verdict detail (expand) | full before/after JSON + evidence · action row |
| Regression actions | Revert · Keep · Investigate |

## Done gate

- Every applied recommendation with a measured verdict appears
  in the list; sort by `measured_at` desc.
- Summary chip counts match the underlying rows.
- Regressions nav entry is always visible even when 0
  (empty pill, not hidden), per the discoverability-of-depth
  theme.
- Revert on a regression archives the underlying memory,
  dismisses the recommendation, and records the reason.
- Verdict notes come through narration-cache when the model is
  available.
- Delta chart values match `before` and `after` exactly (no
  chart-vs-number drift).

## Wrong gate

- **Regression alerts hide when the count is 0.** Discoverability
  regression.
- **Revert action doesn't archive the memory.** Feedback loop
  broken.
- **Chart delta disagrees with `after - before`.** Chart
  derivation buggy.
- **Verdict list includes `insufficient_data` rows in the
  positive/negative counts.** Bucketing mislabel.
- **A verdict shows for a recommendation the user didn't
  apply.** Join broken; only applied recs get verdicts.

## Related

- [[pipeline/impact]] — data source
- [[pipeline/insights]] — where applied recs come from
- [[pipeline/ftr]] — before/after snapshot fields
- [[screen/observatory-today]] — surfaces unacknowledged
  regressions as a red banner
- [[screen/project-impact]] — project-scoped version
