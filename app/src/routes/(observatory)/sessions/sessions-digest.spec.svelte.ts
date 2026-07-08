// @vitest-environment jsdom
import { describe, it, expect, vi } from 'vitest';
import { SessionsDigestState } from './sessions-digest.svelte.js';
import type { SessionRow } from '$lib/types.js';

const wire = (o: Partial<SessionRow> = {}): SessionRow => ({
  id: 's1',
  project: 'sensei',
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
    const s = new SessionsDigestState(async () => [], [wire({ outcome: 'completed', ftr: true })], '7d');
    expect(s.range).toBe('7d');
    expect(s.enriched).toHaveLength(1);
    expect(s.totals.count).toBe(1);
    expect(s.totals.good).toBe(1);
    expect(s.dayBuckets).toHaveLength(7);
  });

  it('the day axis follows the active range', () => {
    const s = new SessionsDigestState(async () => [], [], '30d');
    expect(s.days).toHaveLength(30);
    expect(s.dayBuckets).toHaveLength(30);
  });
});

describe('SessionsDigestState — chart + mini-cycler', () => {
  it('setChart switches the active chart', () => {
    const s = new SessionsDigestState(async () => []);
    expect(s.chart).toBe('trend');
    s.setChart('bands');
    expect(s.chart).toBe('bands');
  });

  it('cycleMiniMode walks every mode including pulse, then wraps', () => {
    const s = new SessionsDigestState(async () => []);
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
    let resolve!: (rows: SessionRow[]) => void;
    const fetcher = vi.fn(
      (_range: '7d' | '30d' | '90d') => new Promise<SessionRow[]>((r) => (resolve = r)),
    );
    const s = new SessionsDigestState(fetcher, [wire({ id: 'a' })], '7d');

    const pending = s.setRange('30d');
    // Range + loading flip synchronously; the fetch is in flight.
    expect(s.range).toBe('30d');
    expect(s.loading).toBe(true);
    expect(fetcher).toHaveBeenCalledWith('30d');

    resolve([wire({ id: 'b' }), wire({ id: 'c' })]);
    await pending;

    expect(s.loading).toBe(false);
    expect(s.sessions.map((x) => x.id)).toEqual(['b', 'c']);
    expect(s.days).toHaveLength(30);
  });

  it('is a no-op when the range is unchanged — no redundant round-trip', async () => {
    const fetcher = vi.fn(async () => []);
    const s = new SessionsDigestState(fetcher, [], '7d');
    await s.setRange('7d');
    expect(fetcher).not.toHaveBeenCalled();
  });

  it('clears loading even when the fetch rejects', async () => {
    const fetcher = vi.fn(async () => {
      throw new Error('daemon down');
    });
    const s = new SessionsDigestState(fetcher, [], '7d');
    await expect(s.setRange('90d')).rejects.toThrow('daemon down');
    expect(s.loading).toBe(false);
    expect(s.range).toBe('90d');
  });
});
