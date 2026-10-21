// Shared FTR presentation helpers.
//
// FTR (a 0..1 rate) is honest-null when a project or window has no analyzed
// sessions — the daemon returns `null`, never a fabricated 0. These helpers keep
// that honesty at the edge: they render the no-data em dash, never a "0%" a
// reader can't tell from a real 0%.

/** The no-data marker rendered when an FTR rate is absent. */
export const FTR_NONE = '—';

/** Whole-number FTR percentage (0–100), or null when the rate is absent. */
export function ftrPct(rate: number | null | undefined): number | null {
  return rate == null ? null : Math.round(rate * 100);
}

/** FTR percentage label — "NN%", or the no-data em dash when the rate is absent. */
export function ftrPctLabel(rate: number | null | undefined): string {
  const pct = ftrPct(rate);
  return pct == null ? FTR_NONE : `${pct}%`;
}

/** Rendered when an FTR read failed (#288) — distinct from the no-data dash,
 *  which means there are no analyzed sessions to measure. */
export const FTR_UNAVAILABLE = 'unavailable';

/** An FTR read as a page holds it — the data, or why it could not be read —
 *  labelled: "unavailable" on a failed read, else the rate or the no-data dash
 *  (also while not yet read, `null`). */
export function ftrReadLabel(
  read: { data: { ftr14d: number | null } | null; error: string | null } | null,
): string {
  return read?.error ? FTR_UNAVAILABLE : ftrPctLabel(read?.data?.ftr14d);
}
