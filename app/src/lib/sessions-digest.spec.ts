import { describe, it, expect } from 'vitest';
import type { SessionRow } from '$lib/types.js';
import {
  sessionsCopy,
  medianLabel,
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
  SESSION_RANGES, dayTone, rollingFtr, recencyRange, labelStride, bandsView, activityByDay, unmeasuredLine, sessionSpans } from './sessions-digest.js';

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
  workspace: 'sensei',
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
    // Days, as Overview showed them (audit 2026-10-28, item 15: three copies of
    // this rule — Sessions, Overview, Today — now one).
    expect(compactDuration(2 * 1440 + 180)).toBe('2d 3h');
    expect(compactDuration(1440)).toBe('1d');
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
  it('gives an unlinked (null workspace) session an empty workspace string', () => {
    const [s] = enrich([wire({ workspace: null })], NOW);
    expect(s.workspace).toBe('');
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
  it('counts by quality, distinct workspaces, and the median duration', () => {
    const rows = [
      wire({ id: 'a', workspace: 'sensei', outcome: 'completed', ftr: true, startedAt: isoDaysAgo(0), completedAt: isoDaysAgo(0, 10, 30) }),
      wire({ id: 'b', workspace: 'rokkit', outcome: 'corrected', ftr: false, startedAt: isoDaysAgo(0), completedAt: isoDaysAgo(0, 10, 20) }),
      wire({ id: 'c', workspace: 'rokkit', outcome: 'abandoned', ftr: null, startedAt: isoDaysAgo(0) }),
    ];
    const totals = computeTotals(enrich(rows, NOW));
    expect(totals.count).toBe(3);
    expect(totals.workspaces).toBe(2);
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

// #293: Observatory and workspace Sessions are one screen; only the words differ.
describe('sessionsCopy', () => {
  it('speaks of the week across workspaces in the observatory', () => {
    const c = sessionsCopy(null);
    expect(c.eyebrow).toBe('Observatory · Sessions');
    expect(c.title).toBe('The shape of your week.');
    expect(c.acrossWorkspaces).toBe(true);
    expect(c.kanji).toBe('録');
  });

  it('speaks of one workspace in its window', () => {
    const c = sessionsCopy('atlas');
    expect(c.eyebrow).toBe('atlas · sessions');
    expect(c.title).toBe('The shape of this workspace.');
    expect(c.acrossWorkspaces).toBe(false);
    expect(c.emptyDescription).toContain('on this workspace');
  });
});

// The median duration said "—" when sessions were short: under a minute
// rounds to 0, which read as "no data".
describe('medianLabel', () => {
  it('says there is no median only when no session has a duration', () => {
    expect(medianLabel(null)).toBe('—');
  });
  it('says under a minute rather than nothing', () => {
    expect(medianLabel(0)).toBe('<1m');
  });
  it('formats a real median', () => {
    expect(medianLabel(20)).toBe('20m');
    expect(medianLabel(75)).toBe('1h 15m');
  });
  it('is null when no session carries a duration', () => {
    expect(computeTotals([]).medianMins).toBeNull();
  });
});

// Audit item 15 (2026-10-28): the trend chart decided a day's tone and the
// rolling FTR inside the component.
describe('dayTone', () => {
  const b = (o: Record<string, number>) => ({ good: 0, bad: 0, ugly: 0, ...o }) as never;
  it('any ugly session marks the day; else more bad than good; else good; else neutral', () => {
    expect([b({ ugly: 1, good: 5 }), b({ bad: 2, good: 1 }), b({ good: 1, bad: 1 }), b({})].map(dayTone)).toEqual(['ugly', 'bad', 'good', 'neutral']);
  });
});

describe('rollingFtr', () => {
  it('is the mean of the last seven scored days, only on a scored day', () => {
    const days = [0.5, null, 1, 0.5, 1, 1, 0.5, 1, null].map((ftr) => ({ ftr }) as never);
    const r = rollingFtr(days);
    expect(r[0]).toBe(0.5);
    expect(r[1]).toBeNull(); // no FTR that day — no point
    expect(r[7]).toBeCloseTo((1 + 0.5 + 1 + 1 + 0.5 + 1) / 6); // window days 1..7, day 1 unscored
    expect(r[8]).toBeNull();
  });
});


// The range a Sessions page opens on follows the workspace's recency (user, 2026-10-28):
// 7 days when it ran within 7, 30 when within 30, 90 otherwise.
describe('recencyRange', () => {
  it('opens on the narrowest window that holds the newest session', () => {
    expect(recencyRange([isoDaysAgo(0)], NOW)).toBe('7d');
    expect(recencyRange([isoDaysAgo(40), isoDaysAgo(6)], NOW)).toBe('7d');
    expect(recencyRange([isoDaysAgo(8)], NOW)).toBe('30d');
    expect(recencyRange([isoDaysAgo(29)], NOW)).toBe('30d');
    expect(recencyRange([isoDaysAgo(31)], NOW)).toBe('90d');
  });

  it('opens on 90 days when nothing ran', () => {
    expect(recencyRange([], NOW)).toBe('90d');
  });
});

describe('labelStride', () => {
  it('labels every day up to two weeks, then about eight labels', () => {
    expect(labelStride(7)).toBe(1);
    expect(labelStride(14)).toBe(1);
    expect(labelStride(30)).toBe(4);
    expect(labelStride(90)).toBe(12);
  });
});

// Bands is the activity hotspot chart (user, 2026-10-28): per local day, the minutes the
// agent worked and the minutes spent on you, from each session's active time filed by the
// hour it happened — not a session's wall-clock span on its start day.
const hourOf = (daysAgo: number, hour: number) => {
  const d = new Date(2026, 6, 8, hour, 0, 0);
  d.setDate(d.getDate() - daysAgo);
  return Math.floor(d.getTime() / 3_600_000);
};

describe('activityByDay', () => {
  const days = buildDays('7d', NOW);

  it('files each hour of agent and user time under the local day it happened', () => {
    const late = wire({ id: 'late', startedAt: isoDaysAgo(1, 23), activeByHour: [
      [hourOf(1, 23), 20 * 60_000, 10 * 60_000],
      [hourOf(0, 0), 30 * 60_000, 5 * 60_000],
    ] });
    const a = activityByDay([late], days, NOW);
    expect(a.days.at(-2)).toMatchObject({ agentMins: 20, userMins: 10 });
    expect(a.days.at(-1)).toMatchObject({ agentMins: 30, userMins: 5 });
    expect(a.unmeasured).toBe(0);
  });

  it('leaves out hours outside the range, and counts sessions with no activity data apart', () => {
    const old = wire({ id: 'old', activeByHour: [[hourOf(20, 9), 60 * 60_000, 0]] });
    const unknown = wire({ id: 'unknown', activeByHour: null });
    const a = activityByDay([old, unknown], days, NOW);
    expect(a.days.every((d) => d.agentMins === 0 && d.userMins === 0)).toBe(true);
    expect(a.unmeasured).toBe(1);
  });
});

describe('bandsView', () => {
  const days = buildDays('90d', NOW);
  const activity = activityByDay([
    wire({ id: 'a', activeByHour: [[hourOf(0, 10), 60 * 60_000, 30 * 60_000]] }),
  ], days, NOW);
  const v = bandsView(activity.days, 1100, 320);

  it('draws one bar per day, oldest left, inside the plot', () => {
    expect(v.bars).toHaveLength(90);
    expect(v.bars[0].x).toBeLessThan(v.bars[89].x);
    for (const b of v.bars) {
      expect(b.x).toBeGreaterThanOrEqual(v.plot.left);
      expect(b.x + b.w).toBeLessThanOrEqual(v.plot.right + 0.001);
    }
  });

  it('stacks a day bottom-up agent → you, heights in proportion to minutes', () => {
    const today = v.bars[89];
    expect(today.segments.map((s) => s.tone)).toEqual(['agent', 'user']);
    const [agent, user] = today.segments;
    expect(agent.y + agent.h).toBeCloseTo(v.plot.bottom);
    expect(user.y + user.h).toBeCloseTo(agent.y);
    expect(agent.h / user.h).toBeCloseTo(2);
    expect(today.mins).toBe(90);
  });

  it('labels a readable subset of days, the newest always', () => {
    expect(v.ticks.length).toBeGreaterThan(1);
    expect(v.ticks.length).toBeLessThanOrEqual(9);
    expect(v.ticks.at(-1)!.label).toBe('Today');
  });

  it('scales the minutes axis to the busiest day, an hour at least', () => {
    expect(v.maxMins).toBeGreaterThanOrEqual(90);
    expect(v.yTicks.map((t) => t.label)).toContain('0m');
  });
});

describe('unmeasuredLine', () => {
  it('says how many sessions are not drawn, and nothing when all are', () => {
    expect(unmeasuredLine(0)).toBeNull();
    expect(unmeasuredLine(1)).toBe('1 session not analysed yet — not drawn');
    expect(unmeasuredLine(3)).toBe('3 sessions not analysed yet — not drawn');
  });
});

// A session that runs over several days is one piece of work across several bars (user,
// 2026-10-29: 'draw an overlay connecting multi day sessions'). Its span is the local days
// it was active on; a one-day session has none. Overlapping spans take separate lanes.
describe('sessionSpans and the bands links', () => {
  const days = buildDays('7d', NOW);
  const rows = [
    wire({ id: 'long', activeByHour: [[hourOf(4, 10), 60_000, 0], [hourOf(2, 9), 60_000, 0], [hourOf(1, 15), 60_000, 0]] }),
    wire({ id: 'short', activeByHour: [[hourOf(0, 10), 60_000, 0]] }),
    wire({ id: 'overlap', activeByHour: [[hourOf(3, 10), 60_000, 0], [hourOf(2, 11), 60_000, 0]] }),
    wire({ id: 'unknown', activeByHour: null }),
  ];

  it('lists the active days of each session that spans more than one, in axis order', () => {
    const spans = sessionSpans(rows, days);
    expect(spans.map((s) => s.id)).toEqual(['long', 'overlap']);
    expect(spans[0].days).toEqual([days[2], days[4], days[5]]);
  });

  it('draws each span from its first day to its last, a dot per active day, overlaps in separate lanes', () => {
    const activity = activityByDay(rows, days, NOW);
    const v = bandsView(activity.days, 1100, 320, sessionSpans(rows, days));
    const centre = (day: string) => {
      const b = v.bars.find((x) => x.day === day)!;
      return b.x + b.w / 2;
    };
    const long = v.links.find((l) => l.id === 'long')!;
    expect(long.x1).toBeCloseTo(centre(days[2]));
    expect(long.x2).toBeCloseTo(centre(days[5]));
    expect(long.dots.map((x) => Math.round(x))).toEqual([days[2], days[4], days[5]].map((d) => Math.round(centre(d))));
    const overlap = v.links.find((l) => l.id === 'overlap')!;
    expect(overlap.y).not.toBe(long.y);
    for (const l of v.links) expect(l.y).toBeLessThan(v.plot.top);
  });
});
