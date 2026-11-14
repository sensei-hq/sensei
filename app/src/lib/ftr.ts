// Shared FTR presentation helpers.
//
// FTR (a 0..1 rate) is honest-null when a workspace or window has no analyzed
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

// ── Bands (audit 2026-10-28, item 6) ─────────────────────────────────────────
// One banding for every screen. The middle band is neutral: accent is the
// "needs you" colour, and an ok rate does not need you. No data is faint —
// absence is not a red signal, and not the same grey as "ok".

/** At or above: good. */
export const FTR_GOOD = 0.8;
/** At or above (below good): ok. Below: low. */
export const FTR_OK = 0.6;

export type FtrBand = 'good' | 'ok' | 'low' | 'none';

export function ftrBand(rate: number | null | undefined): FtrBand {
  if (rate == null) return 'none';
  if (rate >= FTR_GOOD) return 'good';
  if (rate >= FTR_OK) return 'ok';
  return 'low';
}

export interface FtrBandView {
  band: FtrBand;
  /** Fill for a status dot. */
  dot: string;
  /** Text colour for the rate. */
  text: string;
  label: string;
}

const BAND_VIEWS: Record<FtrBand, Omit<FtrBandView, 'band'>> = {
  good: { dot: 'bg-success', text: 'text-success', label: 'healthy' },
  ok: { dot: 'bg-ink-soft', text: 'text-ink-soft', label: 'ok' },
  low: { dot: 'bg-warning', text: 'text-warning', label: 'needs attention' },
  none: { dot: 'bg-ink-faint', text: 'text-ink-faint', label: 'no ftr data' }
};

export function ftrBandView(rate: number | null | undefined): FtrBandView {
  const band = ftrBand(rate);
  return { band, ...BAND_VIEWS[band] };
}

/** One session's first-try standing, whether its FTR is a boolean or a rate
 *  (1 = first try). Unscored is faint, never a warning. */
export interface SessionFtrView {
  firstTry: boolean | null;
  tone: 'success' | 'warn' | 'none';
  dot: string;
  text: string;
  label: string;
}

export function sessionFtrView(ftr: boolean | number | null | undefined): SessionFtrView {
  if (ftr == null) return { firstTry: null, tone: 'none', dot: 'bg-ink-faint', text: 'text-ink-faint', label: FTR_NONE };
  const firstTry = typeof ftr === 'boolean' ? ftr : ftr >= 1;
  return firstTry
    ? { firstTry, tone: 'success', dot: 'bg-success', text: 'text-success', label: 'first try' }
    : { firstTry, tone: 'warn', dot: 'bg-warning', text: 'text-warning', label: 'rework' };
}

/** A session's corrections, in words (audit 2026-10-28, item 7: it read
 *  "first-try right", "1 corrections", "3× rework" and "no corrections" on four
 *  screens). Whether it was first-try is `sessionFtrView`'s to say. */
export interface CorrectionsView {
  label: string;
  text: string;
}

export function correctionsView(n: number | null | undefined): CorrectionsView {
  if (n == null) return { label: FTR_NONE, text: 'text-ink-mute' };
  if (n <= 0) return { label: 'no corrections', text: 'text-success' };
  return { label: n === 1 ? '1 correction' : `${n.toLocaleString('en-US')} corrections`, text: 'text-warning' };
}
