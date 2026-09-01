/**
 * Display-only date helpers.
 *
 * Extracted when a third caller wanted the same three lines: the collective
 * sharing screen's `collectiveDateLabel`, the metric series' private `dayOf`, and
 * the dōjō sync surface. They differed only in what they returned for absent
 * input, which is exactly the kind of near-duplicate that drifts.
 */

/** Leading `YYYY-MM-DD`. Anchored, so a non-date string matches nothing. */
const LEADING_DAY = /^(\d{4}-\d{2}-\d{2})/;

/**
 * The `YYYY-MM-DD` portion of an ISO-8601 timestamp, or `null`.
 *
 * Taken from the STRING, never through `Date`. `new Date(iso)` renders in the
 * viewer's local zone, so a late-evening UTC timestamp shows as the previous day
 * for anyone west of it — a watermark "settled through 2026-08-31" would read as
 * the 30th, and the reader would go looking for a day of missing data that
 * settled fine.
 *
 * Matched rather than sliced: a plain `length >= 10` check (what the private
 * helper did) turns `"unavailable"` into the day `"unavailabl"`.
 */
export function isoDay(iso: string | null | undefined): string | null {
  if (typeof iso !== 'string') return null;
  const m = iso.trim().match(LEADING_DAY);
  return m ? m[1] : null;
}
