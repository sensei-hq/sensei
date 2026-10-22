// @vitest-environment jsdom
import { describe, it, expect, vi } from 'vitest';
import { SessionsDigestState, sessionsFetcher } from './sessions-digest.svelte.js';
import type { ApiResult } from './api.js';
import type { SessionRow } from '$lib/types.js';

const ok = <T,>(data: T) => ({ ok: true as const, data });
const down = { ok: false as const, error: { status: 503, message: 'Service Unavailable' } };
const none = async () => ok<SessionRow[]>([]);

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
  it('seeds from the constructor and derives enriched / totals / dayBuckets', () => {
    const s = new SessionsDigestState(none, [wire({ outcome: 'completed', ftr: true })], '7d');
    expect(s.range).toBe('7d');
    expect(s.enriched).toHaveLength(1);
    expect(s.totals.count).toBe(1);
    expect(s.totals.good).toBe(1);
    expect(s.dayBuckets).toHaveLength(7);
  });

  it('the day axis follows the active range', () => {
    const s = new SessionsDigestState(none, [], '30d');
    expect(s.days).toHaveLength(30);
    expect(s.dayBuckets).toHaveLength(30);
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
    let resolve!: (r: ApiResult<SessionRow[]>) => void;
    const fetcher = vi.fn(
      (_range: '7d' | '30d' | '90d') => new Promise<ApiResult<SessionRow[]>>((r) => (resolve = r)),
    );
    const s = new SessionsDigestState(fetcher, [wire({ id: 'a' })], '7d');

    const pending = s.setRange('30d');
    // Range + loading flip synchronously; the fetch is in flight.
    expect(s.range).toBe('30d');
    expect(s.loading).toBe(true);
    expect(fetcher).toHaveBeenCalledWith('30d');

    resolve(ok([wire({ id: 'b' }), wire({ id: 'c' })]));
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
    const fetcher = vi.fn(async () => ok([wire({ id: 'b' })]));
    const s = new SessionsDigestState(fetcher, null, '30d', 'Service Unavailable');
    await s.retry();
    expect(fetcher).toHaveBeenCalledWith('30d');
    expect([s.error, s.sessions.map((x) => x.id)]).toEqual([null, ['b']]);
  });
});

describe('sessionsFetcher', () => {
  it('reads the digest for a range — and, given one, a workspace — as its rows', async () => {
    const tryGetSessionsDigest = vi.fn(async () => ok({ sessions: [wire({ id: 'a' })] }));
    const read = await sessionsFetcher({ tryGetSessionsDigest }, 'proj-42')('30d');
    expect(tryGetSessionsDigest).toHaveBeenCalledWith('30d', 'proj-42');
    expect(read.ok && read.data.map((x) => x.id)).toEqual(['a']);
  });

  it('passes a failed read through', async () => {
    const read = await sessionsFetcher({ tryGetSessionsDigest: async () => down })('7d');
    expect(read).toEqual(down);
  });
});
