import { describe, it, expect } from 'vitest';
import { ftrPct, ftrPctLabel, FTR_NONE } from './ftr.js';

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
