import { describe, it, expect } from 'vitest';
import type { SessionRow } from '$lib/types.js';
import {
  quality,
  sessionTitle,
  durationMinutes,
  compactDuration,
  timeOfDay,
  relativeWhen,
  dayKey,
  shortDayLabel,
  enrich,
  buildDays,
  aggregateByDay,
  computeTotals,
  median,
  replayHref,
  nextMiniMode,
  rangeDays,
  compactTokens,
  activeMinutes,
  CHART_VARIANTS,
  MINI_MODES,
  SESSION_RANGES,
} from './sessions-digest.js';

// Fixed reference "now" (local July 8 2026, noon) so relative labels + day
// buckets are deterministic regardless of the runner's timezone.
const NOW = new Date(2026, 6, 8, 12, 0, 0);
function isoDaysAgo(n: number, hour = 10, min = 5): string {
  const d = new Date(2026, 6, 8, hour, min, 0);
  d.setDate(d.getDate() - n);
  return d.toISOString();
}

const wire = (o: Partial<SessionRow> = {}): SessionRow => ({
  id: 's1',
  project: 'sensei',
  task: 'Fix auth',
  summary: null,
  outcome: 'completed',
  ftr: true,
  turns: 10,
  corrections: 0,
  startedAt: isoDaysAgo(0),
  completedAt: null,
  agent: 'claude',
  ...o,
});

describe('token + active-duration helpers', () => {
  it('compactTokens abbreviates by magnitude, dash for absent', () => {
    expect(compactTokens(820)).toBe('820');
    expect(compactTokens(7732)).toBe('7.7k');
    expect(compactTokens(45000)).toBe('45k');
    expect(compactTokens(1_300_000)).toBe('1.3M');
    expect(compactTokens(132_332_559)).toBe('132M');
    expect(compactTokens(null)).toBe('—');
    expect(compactTokens(undefined)).toBe('—');
  });
  it('activeMinutes converts daemon durationSecs, null for absent/negative', () => {
    expect(activeMinutes(1800)).toBe(30);
    expect(activeMinutes(90)).toBe(2); // rounds
    expect(activeMinutes(null)).toBe(null);
    expect(activeMinutes(-5)).toBe(null);
  });
});

describe('token enrichment + aggregation', () => {
  it('enrich carries in/out + sum; both absent ⇒ null (never a fabricated 0)', () => {
    const [a] = enrich([wire({ tokensIn: 6000, tokensOut: 1732 })], NOW);
    expect(a.tokensIn).toBe(6000);
    expect(a.tokensOut).toBe(1732);
    expect(a.tokens).toBe(7732);
    expect(a.tokensLabel).toBe('7.7k');
    const [b] = enrich([wire({ tokensIn: null, tokensOut: null })], NOW);
    expect(b.tokens).toBe(null);
    expect(b.tokensLabel).toBe('—');
    // one side present ⇒ the other counts as 0 in the sum (a real partial capture)
    const [c] = enrich([wire({ tokensIn: 100, tokensOut: null })], NOW);
    expect(c.tokens).toBe(100);
  });
  it('aggregateByDay sums token volume per day', () => {
    const sessions = enrich(
      [
        wire({ id: 'a', startedAt: isoDaysAgo(0), tokensIn: 100, tokensOut: 20 }),
        wire({ id: 'b', startedAt: isoDaysAgo(0), tokensIn: 300, tokensOut: 80 }),
      ],
      NOW,
    );
    const days = buildDays('7d', NOW);
    const buckets = aggregateByDay(sessions, days, NOW);
    const today = buckets[buckets.length - 1];
    expect(today.tokensIn).toBe(400);
    expect(today.tokensOut).toBe(100);
    expect(today.tokens).toBe(500);
  });
  it('computeTotals reports total + median tokens, null when none carry usage', () => {
    const withTokens = enrich(
      [wire({ id: 'a', tokensIn: 100, tokensOut: 0 }), wire({ id: 'b', tokensIn: 300, tokensOut: 0 })],
      NOW,
    );
    const t = computeTotals(withTokens);
    expect(t.totalTokens).toBe(400);
    expect(t.medianTokens).toBe(200);
    const none = computeTotals(enrich([wire({ tokensIn: null, tokensOut: null })], NOW));
    expect(none.totalTokens).toBe(null);
    expect(none.medianTokens).toBe(null);
  });
});

describe('quality mapping (outcome → tone)', () => {
  it('good = completed AND ftr', () => {
    expect(quality({ outcome: 'completed', ftr: true })).toBe('good');
  });
  it('completed WITHOUT ftr is neutral, not good', () => {
    expect(quality({ outcome: 'completed', ftr: false })).toBe('neutral');
    expect(quality({ outcome: 'completed', ftr: null })).toBe('neutral');
  });
  it('bad = corrected (regardless of ftr)', () => {
    expect(quality({ outcome: 'corrected', ftr: false })).toBe('bad');
    expect(quality({ outcome: 'corrected', ftr: true })).toBe('bad');
  });
  it('ugly = abandoned', () => {
    expect(quality({ outcome: 'abandoned', ftr: null })).toBe('ugly');
  });
  it('blocked and partial render neutral', () => {
    expect(quality({ outcome: 'blocked', ftr: null })).toBe('neutral');
    expect(quality({ outcome: 'partial', ftr: null })).toBe('neutral');
  });
  it('an unknown outcome falls back to neutral', () => {
    expect(quality({ outcome: 'mystery', ftr: true })).toBe('neutral');
  });
});

describe('sessionTitle', () => {
  it('prefers task, then summary, then a generic word', () => {
    expect(sessionTitle({ task: 'Fix', summary: null })).toBe('Fix');
    expect(sessionTitle({ task: '', summary: 'reset flow' })).toBe('reset flow');
    expect(sessionTitle({ task: '  ', summary: '' })).toBe('session');
  });
});

describe('duration', () => {
  it('durationMinutes returns whole minutes between start and end', () => {
    expect(durationMinutes('2026-07-08T10:00:00Z', '2026-07-08T10:42:00Z')).toBe(42);
  });
  it('is null with no completion or a non-positive span', () => {
    expect(durationMinutes('2026-07-08T10:00:00Z', null)).toBeNull();
    expect(durationMinutes('2026-07-08T10:00:00Z', '2026-07-08T09:00:00Z')).toBeNull();
    expect(durationMinutes('bad', 'also-bad')).toBeNull();
  });
  it('compactDuration formats "12m" / "1h 04m" / "2h" / "—"', () => {
    expect(compactDuration(12)).toBe('12m');
    expect(compactDuration(64)).toBe('1h 04m');
    expect(compactDuration(120)).toBe('2h');
    expect(compactDuration(null)).toBe('—');
  });
});

describe('timeOfDay', () => {
  it('renders a HH:MM 24h string, empty for a bad date', () => {
    expect(timeOfDay('2026-07-08T10:05:00Z')).toMatch(/^\d{2}:\d{2}$/);
    expect(timeOfDay('nonsense')).toBe('');
  });
});

describe('relativeWhen', () => {
  it('labels by calendar-day distance from now', () => {
    expect(relativeWhen(isoDaysAgo(0), NOW)).toBe('today');
    expect(relativeWhen(isoDaysAgo(1), NOW)).toBe('yesterday');
    expect(relativeWhen(isoDaysAgo(3), NOW)).toBe('3 days ago');
    expect(relativeWhen(isoDaysAgo(7), NOW)).toBe('last week');
    expect(relativeWhen(isoDaysAgo(14), NOW)).toBe('2 weeks ago');
    expect(relativeWhen('bad', NOW)).toBe('');
  });
});

describe('shortDayLabel', () => {
  it('reads Today / Yest, then a real mm/dd date — never a −N offset', () => {
    expect(shortDayLabel(dayKey(isoDaysAgo(0)), NOW)).toBe('Today');
    expect(shortDayLabel(dayKey(isoDaysAgo(1)), NOW)).toBe('Yest');

    // Older days carry the date itself, so a bar can be matched against a
    // commit or a calendar instead of forcing arithmetic off an implicit today.
    const older = dayKey(isoDaysAgo(4));
    const [, m, d] = older.split('-');
    expect(shortDayLabel(older, NOW)).toBe(`${m}/${d}`);
    expect(shortDayLabel(older, NOW)).not.toContain('−');
  });

  it('zero-pads month and day so the axis stays aligned', () => {
    expect(shortDayLabel('2026-03-07', new Date('2026-04-01T12:00:00'))).toBe('03/07');
  });

  it('is empty for a missing key', () => {
    expect(shortDayLabel('', NOW)).toBe('');
  });
});

describe('enrich', () => {
  it('carries every derived display field', () => {
    const [s] = enrich([wire({ startedAt: isoDaysAgo(1), completedAt: null, task: 'Fix auth' })], NOW);
    expect(s.title).toBe('Fix auth');
    expect(s.when).toBe('yesterday');
    expect(s.time).toMatch(/^\d{2}:\d{2}$/);
    expect(s.duration).toBe('—'); // no completedAt
    expect(s.quality).toBe('good');
    expect(s.agent).toBe('claude');
  });
  it('gives an unlinked (null project) session an empty project string', () => {
    const [s] = enrich([wire({ project: null })], NOW);
    expect(s.project).toBe('');
  });
});

describe('buildDays', () => {
  it('spans rangeDays, oldest→newest, ending today', () => {
    const days = buildDays('7d', NOW);
    expect(days).toHaveLength(7);
    expect(days[days.length - 1]).toBe(dayKey(isoDaysAgo(0)));
    expect(days[0]).toBe(dayKey(isoDaysAgo(6)));
    expect(buildDays('30d', NOW)).toHaveLength(30);
    expect(buildDays('90d', NOW)).toHaveLength(90);
  });
});

describe('aggregateByDay', () => {
  it('buckets sessions into the day axis with per-quality counts', () => {
    const rows = [
      wire({ id: 'a', outcome: 'completed', ftr: true, startedAt: isoDaysAgo(0), completedAt: isoDaysAgo(0, 10, 45) }),
      wire({ id: 'b', outcome: 'completed', ftr: true, startedAt: isoDaysAgo(0) }),
      wire({ id: 'c', outcome: 'corrected', ftr: false, startedAt: isoDaysAgo(0) }),
      wire({ id: 'd', outcome: 'abandoned', ftr: null, startedAt: isoDaysAgo(1) }),
    ];
    const buckets = aggregateByDay(enrich(rows, NOW), buildDays('7d', NOW), NOW);
    const today = buckets[buckets.length - 1];
    expect(today.total).toBe(3);
    expect(today.good).toBe(2);
    expect(today.bad).toBe(1);
    expect(today.ftr).toBeCloseTo(2 / 3);
    expect(today.goodMins).toBe(40); // only row 'a' has a completion
    const yesterday = buckets[buckets.length - 2];
    expect(yesterday.total).toBe(1);
    expect(yesterday.ugly).toBe(1);
    expect(yesterday.ftr).toBe(0);
  });
  it('chartedMins = good+bad+ugly minutes and excludes neutral (blocked/partial) time', () => {
    const rows = [
      wire({ id: 'g', outcome: 'completed', ftr: true, startedAt: isoDaysAgo(0), completedAt: isoDaysAgo(0, 10, 20) }),
      wire({ id: 'n', outcome: 'blocked', ftr: null, startedAt: isoDaysAgo(0), completedAt: isoDaysAgo(0, 10, 40) }),
    ];
    const today = aggregateByDay(enrich(rows, NOW), buildDays('7d', NOW), NOW).at(-1)!;
    expect(today.neutral).toBe(1);
    expect(today.chartedMins).toBe(today.goodMins + today.badMins + today.uglyMins);
    // neutral session time is counted in `mins` but never drawn, so chartedMins < mins.
    expect(today.chartedMins).toBeLessThan(today.mins);
  });
  it('leaves empty days with a null ftr so the trend line skips them', () => {
    const buckets = aggregateByDay([], buildDays('7d', NOW), NOW);
    expect(buckets.every((b) => b.total === 0 && b.ftr === null)).toBe(true);
  });
});

describe('computeTotals', () => {
  it('counts by quality, distinct projects, and the median duration', () => {
    const rows = [
      wire({ id: 'a', project: 'sensei', outcome: 'completed', ftr: true, startedAt: isoDaysAgo(0), completedAt: isoDaysAgo(0, 10, 30) }),
      wire({ id: 'b', project: 'rokkit', outcome: 'corrected', ftr: false, startedAt: isoDaysAgo(0), completedAt: isoDaysAgo(0, 10, 20) }),
      wire({ id: 'c', project: 'rokkit', outcome: 'abandoned', ftr: null, startedAt: isoDaysAgo(0) }),
    ];
    const totals = computeTotals(enrich(rows, NOW));
    expect(totals.count).toBe(3);
    expect(totals.projects).toBe(2);
    expect(totals.good).toBe(1);
    expect(totals.bad).toBe(1);
    expect(totals.ugly).toBe(1);
    // durations: row a 10:05→10:30 = 25m, row b 10:05→10:20 = 15m; abandoned
    // has none. median of [15, 25] = 20.
    expect(totals.medianMins).toBe(20);
  });
});

describe('median', () => {
  it('handles empty, odd, and even lengths', () => {
    expect(median([])).toBe(0);
    expect(median([5])).toBe(5);
    expect(median([1, 2, 3])).toBe(2);
    expect(median([1, 2, 3, 4])).toBe(2.5);
  });
});

describe('replayHref', () => {
  it('deep-links to the Instruments Replay tab with the session preselected', () => {
    expect(replayHref('abc')).toBe('/instruments?tab=replay&session=abc');
    expect(replayHref('a b')).toBe('/instruments?tab=replay&session=a%20b');
  });
});

describe('chart / cycler vocab', () => {
  it('the chip group is exactly the five charts — pulse is never a chip', () => {
    expect(CHART_VARIANTS).toEqual(['trend', 'stream', 'constellation', 'bands', 'tokens']);
    expect(CHART_VARIANTS).not.toContain('pulse');
  });
  it('pulse exists only on the mini-cycler', () => {
    expect(MINI_MODES).toContain('pulse');
    expect(MINI_MODES).toEqual(['numbers', 'trend', 'stream', 'constellation', 'bands', 'pulse']);
  });
  it('nextMiniMode advances and wraps past pulse back to numbers', () => {
    expect(nextMiniMode('numbers')).toBe('trend');
    expect(nextMiniMode('bands')).toBe('pulse');
    expect(nextMiniMode('pulse')).toBe('numbers');
  });
  it('rangeDays and SESSION_RANGES agree', () => {
    expect(SESSION_RANGES).toEqual(['7d', '30d', '90d']);
    expect(rangeDays('7d')).toBe(7);
    expect(rangeDays('30d')).toBe(30);
    expect(rangeDays('90d')).toBe(90);
  });
});
