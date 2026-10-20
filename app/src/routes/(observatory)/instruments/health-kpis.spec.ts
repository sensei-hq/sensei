import { describe, it, expect } from 'vitest';
import { firstTryKpi } from './health-kpis';

// #288: the Health tab's first-try KPI says when it could not be read, rather
// than showing the "—" that means no sessions yet.
describe('firstTryKpi', () => {
  it('is the rate as a whole percentage', () => {
    expect(firstTryKpi({ data: { ftr14d: 0.826 }, error: null })).toBe('83%');
  });
  it('is "—" when there is no rate yet', () => {
    expect(firstTryKpi({ data: { ftr14d: null }, error: null })).toBe('—');
  });
  it('is "unavailable" when the read failed', () => {
    expect(firstTryKpi({ data: null, error: 'Bad Gateway' })).toBe('unavailable');
  });
  it('is "—" while not yet read', () => {
    expect(firstTryKpi(null)).toBe('—');
  });
});
