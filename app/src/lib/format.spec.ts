import { describe, it, expect } from 'vitest';
import { fmtMs, fmtCount } from './format';

// Audit item 15 (2026-10-28): a duration was formatted three ways ("120ms",
// "120 ms", "1.20 s" / "1.2s") by three private helpers.
describe('fmtMs', () => {
  it('reads milliseconds under a second, seconds to one place above', () => {
    expect([0, 45.4, 999, 1000, 1500, 3249].map(fmtMs)).toEqual(['0 ms', '45 ms', '999 ms', '1.0 s', '1.5 s', '3.2 s']);
  });
  it('a duration not measured is a dash', () => {
    expect(fmtMs(null)).toBe('—');
    expect(fmtMs(undefined)).toBe('—');
  });
});

describe('fmtCount', () => {
  it('groups thousands, a dash when not counted', () => {
    expect(fmtCount(12345)).toBe('12,345');
    expect(fmtCount(null)).toBe('—');
  });
});
