// @vitest-environment jsdom
import { describe, it, expect, vi } from 'vitest';
import { SessionsDigestState, sessionsFetcher, loadSessionsDigest, type DigestRead } from './sessions-digest.svelte.js';
import type { ApiResult } from './api.js';
import type { HourActivity, SessionRow } from '$lib/types.js';

const ok = <T,>(data: T) => ({ ok: true as const, data });
const down = { ok: false as const, error: { status: 503, message: 'Service Unavailable' } };
/** A fetcher's read: the rows, and (by default no) hours of activity. */
const read = (sessions: SessionRow[], activity: HourActivity[] = []) => ok({ sessions, activity });
const none = async () => read([]);

const wire = (o: Partial<SessionRow> = {}): SessionRow => ({
  id: 's1',
  workspace: 'sensei',
  task: 'Fix auth',
  summary: null,
  outcome: 'completed',
  ftr: true,
  turns: 10,
  corrections: 0,
  startedAt: new Date().toISOString(),
  completedAt: null,
  agent: 'claude',
  ...o,
});

describe('SessionsDigestState — seeding + derivations', () => {
  it('seeds from the constructor and derives enriched / totals / turn days', () => {
    const s = new SessionsDigestState(none, [wire({ outcome: 'completed', ftr: true })], '7d');
    expect(s.range).toBe('7d');
    expect(s.enriched).toHaveLength(1);
    expect(s.totals.count).toBe(1);
    expect(s.totals.good).toBe(1);
    expect(s.turnDays).toHaveLength(7);
  });

  it('the day axis follows the active range', () => {
    const s = new SessionsDigestState(none, [], '30d');
    expect(s.days).toHaveLength(30);
    expect(s.turnDays).toHaveLength(30);
  });
});

describe('SessionsDigestState — chart + mini-cycler', () => {
  it('setChart switches the active chart', () => {
    const s = new SessionsDigestState(none);
    expect(s.chart).toBe('trend');
    s.setChart('bands');
    expect(s.chart).toBe('bands');
  });

  it('cycleMiniMode walks every mode including pulse, then wraps', () => {
    const s = new SessionsDigestState(none);
    expect(s.miniMode).toBe('numbers');
    s.cycleMiniMode();
    expect(s.miniMode).toBe('trend');
    s.miniMode = 'bands';
    s.cycleMiniMode();
    expect(s.miniMode).toBe('pulse');
    s.cycleMiniMode();
    expect(s.miniMode).toBe('numbers');
  });
});

describe('SessionsDigestState — range refetch', () => {
  it('refetches, swaps the rows, moves the axis, and toggles loading', async () => {
    let resolve!: (r: ApiResult<DigestRead>) => void;
    const fetcher = vi.fn(
      (_range: '7d' | '30d' | '90d') => new Promise<ApiResult<DigestRead>>((r) => (resolve = r)),
    );
    const s = new SessionsDigestState(fetcher, [wire({ id: 'a' })], '7d');

    const pending = s.setRange('30d');
    // Range + loading flip synchronously; the fetch is in flight.
    expect(s.range).toBe('30d');
    expect(s.loading).toBe(true);
    expect(fetcher).toHaveBeenCalledWith('30d');

    resolve(read([wire({ id: 'b' }), wire({ id: 'c' })]));
    await pending;

    expect(s.loading).toBe(false);
    expect(s.sessions.map((x) => x.id)).toEqual(['b', 'c']);
    expect(s.days).toHaveLength(30);
  });

  it('is a no-op when the range is unchanged — no redundant round-trip', async () => {
    const fetcher = vi.fn(none);
    const s = new SessionsDigestState(fetcher, [], '7d');
    await s.setRange('7d');
    expect(fetcher).not.toHaveBeenCalled();
  });
});

// #288: sessions that could not be read are an error — not an empty digest
// that reads as a quiet week.
describe('SessionsDigestState — a failed read', () => {
  it('starts in error when the first load failed', () => {
    const s = new SessionsDigestState(none, null, '7d', 'Service Unavailable');
    expect([s.error, s.sessions]).toEqual(['Service Unavailable', []]);
  });

  it('a failed range switch is an error, and drops the previous range\'s rows', async () => {
    const s = new SessionsDigestState(async () => down, [wire({ id: 'a' })], '7d');
    await s.setRange('90d');
    expect([s.error, s.sessions, s.loading, s.range]).toEqual(['Service Unavailable', [], false, '90d']);
  });

  it('retry() re-reads the current range and clears the error', async () => {
    const fetcher = vi.fn(async () => read([wire({ id: 'b' })]));
    const s = new SessionsDigestState(fetcher, null, '30d', 'Service Unavailable');
    await s.retry();
    expect(fetcher).toHaveBeenCalledWith('30d');
    expect([s.error, s.sessions.map((x) => x.id)]).toEqual([null, ['b']]);
  });
});

describe('sessionsFetcher', () => {
  it('reads the digest for a range — and, given one, a workspace — as its rows', async () => {
    const activity: HourActivity[] = [[490000, 3, 1, 100, 10]];
    const tryGetSessionsDigest = vi.fn(async () => ok({ sessions: [wire({ id: 'a' })], activity }));
    const got = await sessionsFetcher({ tryGetSessionsDigest }, 'proj-42')('30d');
    expect(tryGetSessionsDigest).toHaveBeenCalledWith('30d', 'proj-42');
    expect(got.ok && got.data.sessions.map((x) => x.id)).toEqual(['a']);
    expect(got.ok && got.data.activity).toEqual(activity);
  });

  it('passes a failed read through', async () => {
    const read = await sessionsFetcher({ tryGetSessionsDigest: async () => down })('7d');
    expect(read).toEqual(down);
  });
});

// A Sessions page opens on the workspace's recency (user, 2026-10-28): the widest window is
// read to find the newest session, then the chosen range is read from the server.
describe('loadSessionsDigest — the range follows recency', () => {
  const daysAgo = (n: number) => new Date(Date.now() - n * 86_400_000).toISOString();

  it('opens on 30 days when the newest session is ten days old, read from the server', async () => {
    const asked: string[] = [];
    const rows = [wire({ id: 'old', startedAt: daysAgo(10) })];
    const fetcher = vi.fn(async (range: string) => { asked.push(range); return read(rows); });
    const d = await loadSessionsDigest(fetcher);
    expect(asked).toEqual(['90d', '30d']);
    expect(d.range).toBe('30d');
    expect(d.sessions).toEqual(rows);
    expect(d.error).toBeNull();
  });

  it('reads once when the workspace is quiet: 90 days', async () => {
    const fetcher = vi.fn(async () => read([]));
    const d = await loadSessionsDigest(fetcher);
    expect(fetcher).toHaveBeenCalledTimes(1);
    expect(d.range).toBe('90d');
  });

  it('says why when the read fails, rather than choosing a range from nothing', async () => {
    const d = await loadSessionsDigest(async () => down);
    expect(d.sessions).toBeNull();
    expect(d.error).toMatch(/Service Unavailable/);
  });
});

// Bands is agent vs your time per day (user, 2026-10-28): the state derives it from the
// rows' hours, beside the count of sessions it could not place.
describe('SessionsDigestState — activity', () => {
  it('sums the rows\' hours into the day axis and counts the unanalysed apart', () => {
    const hour = Math.floor(Date.now() / 3_600_000);
    const s = new SessionsDigestState(none, [
      wire({ id: 'a', activeByHour: [[hour, 40 * 60_000, 20 * 60_000]] }),
      wire({ id: 'b', activeByHour: null }),
    ], '7d');
    expect(s.activity.days).toHaveLength(7);
    expect(s.activity.days.at(-1)).toMatchObject({ agentMins: 40, userMins: 20 });
    expect(s.activity.unmeasured).toBe(1);
  });
});

describe('SessionsDigestState — turn days', () => {
  it('derives each day\'s turns from the hours, and a range change re-reads them', async () => {
    const hour = Math.floor(Date.now() / 3_600_000);
    const s = new SessionsDigestState(async () => read([], [[hour, 9, 0, null, null]]), [], '7d', null, [[hour, 4, 1, 100, 10]]);
    expect(s.turnDays.at(-1)).toMatchObject({ turns: 4, corrections: 1, ftr: 0.75, tokens: 110 });
    await s.setRange('30d');
    expect(s.turnDays).toHaveLength(30);
    expect(s.turnDays.at(-1)).toMatchObject({ turns: 9, tokens: null });
  });
});
