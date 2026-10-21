import { describe, it, expect } from 'vitest';
import { ftrPct, ftrPctLabel, ftrReadLabel, FTR_NONE, FTR_UNAVAILABLE } from './ftr.js';

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
