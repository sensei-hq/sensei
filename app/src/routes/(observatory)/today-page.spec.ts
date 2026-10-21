import { describe, it, expect } from 'vitest';
import { todayPageData } from './today-view.svelte.js';
import type { ObservatoryToday, ObservatoryFtr } from '$lib/types.js';

const ok = <T,>(data: T) => ({ ok: true as const, data });
const down = { ok: false as const, error: { status: 503, message: 'Service Unavailable' } };

const today = (over: Partial<ObservatoryToday> = {}): ObservatoryToday => ({
  greeting: 'morning',
  today: 'Tuesday',
  dataMaturity: 'mature',
  hero: { kanji: '観', koan: 'k', body: 'b', impact: null, action: null, source: 's', noticed: 'n' },
  insights: [],
  adopted: [],
  recentSessions: [],
  ...over,
});
const ftr: ObservatoryFtr = { ftr14d: 0.8, ftr14dPrev: 0.7, ftrTrend: [0.7, 0.8], sessions7d: 9 };

// #288: a daemon that cannot answer is an error the screen shows — not the
// early "still listening" state, and not a "—" FTR that reads as no data.
describe('todayPageData', () => {
  it('carries the screen when both reads succeed', () => {
    const d = todayPageData(ok(today()), ok(ftr));
    expect(d.today?.dataMaturity).toBe('mature');
    expect(d.todayError).toBeNull();
    expect(d.ftr).toEqual(ftr);
    expect(d.ftrError).toBeNull();
  });

  it('a failed Today read is an error, never the early state', () => {
    const d = todayPageData(down, ok(ftr));
    expect(d.today).toBeNull();
    expect(d.todayError).toBe('Service Unavailable');
    expect(d.recentSessions).toEqual([]);
  });

  it('a failed FTR read is its own error; the rest of the screen still renders', () => {
    const d = todayPageData(ok(today()), down);
    expect(d.today).not.toBeNull();
    expect(d.ftr).toBeNull();
    expect(d.ftrError).toBe('Service Unavailable');
  });
});
