import { describe, it, expect } from 'vitest';
import { ftrPct, ftrPctLabel, ftrReadLabel, FTR_NONE, FTR_UNAVAILABLE, FTR_GOOD, FTR_OK, ftrBand, ftrBandView, sessionFtrView } from './ftr.js';

describe('ftrPct', () => {
  it('rounds a 0..1 rate to a whole-number percentage', () => {
    expect(ftrPct(0.784)).toBe(78);
    expect(ftrPct(0)).toBe(0);
  });

  it('returns null for an absent rate — never a fabricated 0', () => {
    expect(ftrPct(null)).toBeNull();
    expect(ftrPct(undefined)).toBeNull();
  });
});

describe('ftrPctLabel', () => {
  it('formats a present rate as "NN%"', () => {
    expect(ftrPctLabel(0.82)).toBe('82%');
    expect(ftrPctLabel(0)).toBe('0%'); // a REAL 0 still reads 0%
  });

  it('renders the no-data em dash for an absent rate, not "0%"', () => {
    expect(ftrPctLabel(null)).toBe(FTR_NONE);
    expect(ftrPctLabel(undefined)).toBe(FTR_NONE);
  });
});

// #288: an FTR that could not be read is said so — distinct from the no-data dash.
describe('ftrReadLabel', () => {
  it('is the rate when the read succeeded', () => {
    expect(ftrReadLabel({ data: { ftr14d: 0.826 }, error: null })).toBe('83%');
  });
  it('is the no-data dash when there is no rate, or no read yet', () => {
    expect(ftrReadLabel({ data: { ftr14d: null }, error: null })).toBe(FTR_NONE);
    expect(ftrReadLabel(null)).toBe(FTR_NONE);
  });
  it('is "unavailable" when the read failed', () => {
    expect(ftrReadLabel({ data: null, error: 'Bad Gateway' })).toBe(FTR_UNAVAILABLE);
    expect(FTR_UNAVAILABLE).toBe('unavailable');
  });
});

// Audit item 6 (2026-10-28): the same FTR was banded differently per screen —
// cut-offs 0.8/0.6 here and 0.8/0.5 there, the middle band neutral on the
// workspace dot and ACCENT (the "needs you" colour) in the sidebar, no data
// grey or faint — and an unscored session drew a warning dot on Today.
describe('ftrBandView — one banding for every screen', () => {
  it('bands at 0.8 and 0.6', () => {
    expect([FTR_GOOD, FTR_OK]).toEqual([0.8, 0.6]);
    expect([0.85, 0.8, 0.7, 0.6, 0.59, null].map((r) => ftrBand(r))).toEqual(['good', 'good', 'ok', 'ok', 'low', 'none']);
  });

  it('gives each band one colour and one label', () => {
    expect(ftrBandView(0.9)).toEqual({ band: 'good', dot: 'bg-success', text: 'text-success', label: 'healthy' });
    expect(ftrBandView(0.7)).toEqual({ band: 'ok', dot: 'bg-ink-soft', text: 'text-ink-soft', label: 'ok' });
    expect(ftrBandView(0.3)).toEqual({ band: 'low', dot: 'bg-warning', text: 'text-warning', label: 'needs attention' });
    expect(ftrBandView(null)).toEqual({ band: 'none', dot: 'bg-ink-faint', text: 'text-ink-faint', label: 'no ftr data' });
  });
});

describe('sessionFtrView — one session, whichever shape its FTR comes in', () => {
  it('first try is success, rework a warning', () => {
    for (const first of [true, 1]) {
      expect(sessionFtrView(first)).toEqual({ firstTry: true, tone: 'success', dot: 'bg-success', text: 'text-success', label: 'first try' });
    }
    for (const rework of [false, 0.5, 0]) {
      expect(sessionFtrView(rework)).toEqual({ firstTry: false, tone: 'warn', dot: 'bg-warning', text: 'text-warning', label: 'rework' });
    }
  });

  it('an unscored session is faint, never a warning — absence is not a red signal', () => {
    expect(sessionFtrView(null)).toEqual({ firstTry: null, tone: 'none', dot: 'bg-ink-faint', text: 'text-ink-faint', label: '—' });
    expect(sessionFtrView(undefined).firstTry).toBeNull();
  });
});
