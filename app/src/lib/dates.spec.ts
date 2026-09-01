// The one date-portion helper. Extracted when a third caller wanted it — the
// sharing screen's `collectiveDateLabel` and the metric series' private `dayOf`
// were already the same three lines with different empty conventions.
import { describe, it, expect } from 'vitest';
import { isoDay } from './dates.js';

describe('isoDay', () => {
  it('takes the leading date without constructing a Date', () => {
    // Deliberately NOT `new Date(iso).toISOString().slice(0,10)`: that shifts a
    // late-evening UTC timestamp to the previous day for anyone west of it, so a
    // watermark rendered "settled through the 30th" would read as the 29th.
    expect(isoDay('2026-08-31T23:57:55Z')).toBe('2026-08-31');
    expect(isoDay('2026-08-31T19:57:55.439726-05:00')).toBe('2026-08-31');
  });

  it('accepts a bare date', () => {
    expect(isoDay('2026-08-31')).toBe('2026-08-31');
  });

  it('is null for absent or unparseable input rather than a guess', () => {
    expect(isoDay(null)).toBeNull();
    expect(isoDay(undefined)).toBeNull();
    expect(isoDay('')).toBeNull();
    expect(isoDay('not a date')).toBeNull();
    // Length alone is not enough — the old private helper sliced any string of
    // 10+ characters, so "unavailable" became the day "unavailabl".
    expect(isoDay('unavailable')).toBeNull();
  });

  it('tolerates surrounding whitespace', () => {
    expect(isoDay('  2026-08-31T10:00:00Z ')).toBe('2026-08-31');
  });
});
