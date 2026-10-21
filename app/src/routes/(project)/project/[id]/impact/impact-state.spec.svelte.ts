import { describe, it, expect, vi } from 'vitest';
import { ProjectImpactState, flattenBuckets, splitLog, type ProjectImpactApi } from './impact-state.svelte';
import type { ImpactRow } from '$lib/impact.js';
import type { ImpactVerdictEntry } from '$lib/types.js';

const ok = <T>(data: T) => ({ ok: true as const, data });
const fail = (status: number, message: string) => ({ ok: false as const, error: { status, message } });

const row = (id: string, verdict: ImpactRow['verdict']) => ({ id, verdict }) as ImpactRow;
const entry = (id: string, verdict: ImpactVerdictEntry['verdict']) => ({ id, verdict }) as ImpactVerdictEntry;

function api(over: Partial<ProjectImpactApi> = {}): ProjectImpactApi {
  return {
    tryCreateImpactVerdict: vi.fn().mockResolvedValue(ok({ id: 'v1' })),
    tryDecideImpactVerdict: vi.fn().mockResolvedValue(ok(undefined)),
    ...over,
  } as ProjectImpactApi;
}

describe('project impact view', () => {
  it('lists negative verdicts first, then positive, neutral, pending', () => {
    const rows = flattenBuckets({
      positive: [row('p', 'positive')], negative: [row('n', 'negative')],
      neutral: [row('z', 'neutral')], pending: [row('w', 'pending')],
    });
    expect(rows.map((r) => r.id)).toEqual(['n', 'p', 'z', 'w']);
  });

  it('splits the log into pending and decided', () => {
    const { pending, decided } = splitLog([entry('a', 'pending'), entry('b', 'success'), entry('c', 'pending')]);
    expect(pending.map((e) => e.id)).toEqual(['a', 'c']);
    expect(decided.map((e) => e.id)).toEqual(['b']);
  });

  it('selects the picked row, else the first', () => {
    const s = new ProjectImpactState();
    const rows = [row('a', 'negative'), row('b', 'positive')];
    expect(s.selectedIn(rows)?.id).toBe('a');
    s.selectedId = 'b';
    expect(s.selectedIn(rows)?.id).toBe('b');
    s.selectedId = 'gone';
    expect(s.selectedIn(rows)?.id).toBe('a');
  });
});

describe('ProjectImpactState — logging an impact', () => {
  it('logs a titled impact, clears the form and reloads', async () => {
    const s = new ProjectImpactState();
    s.logForm = { title: '  faster reviews ', note: ' ' };
    const a = api();
    const reload = vi.fn().mockResolvedValue(undefined);
    await s.logImpact(a, 'p1', reload);
    expect(a.tryCreateImpactVerdict).toHaveBeenCalledWith('p1', 'faster reviews', undefined);
    expect(s.logForm).toEqual({ title: '', note: '' });
    expect(reload).toHaveBeenCalledOnce();
    expect(s.logging).toBe(false);
  });

  it('logs nothing without a title', async () => {
    const s = new ProjectImpactState();
    const a = api();
    await s.logImpact(a, 'p1', vi.fn());
    expect(a.tryCreateImpactVerdict).not.toHaveBeenCalled();
  });

  it('keeps what was typed and says why when logging fails', async () => {
    const s = new ProjectImpactState();
    s.logForm = { title: 'x', note: 'y' };
    const reload = vi.fn();
    await s.logImpact(api({ tryCreateImpactVerdict: vi.fn().mockResolvedValue(fail(500, 'boom')) }), 'p1', reload);
    expect(s.logForm).toEqual({ title: 'x', note: 'y' });
    expect(s.logError).toContain('boom');
    expect(reload).not.toHaveBeenCalled();
  });
});

describe('ProjectImpactState — deciding', () => {
  it('decides and reloads; a refusal is shown on that entry', async () => {
    const s = new ProjectImpactState();
    const reload = vi.fn().mockResolvedValue(undefined);
    const a = api();
    await s.decide(a, 'p1', 'v1', 'success', reload);
    expect(a.tryDecideImpactVerdict).toHaveBeenCalledWith('p1', 'v1', 'success');
    expect(reload).toHaveBeenCalledOnce();

    await s.decide(api({ tryDecideImpactVerdict: vi.fn().mockResolvedValue(fail(404, 'Not Found')) }), 'p1', 'v2', 'failure', reload);
    expect(s.decideError('v2')).toContain('Not Found');
    expect(s.isDeciding('v2')).toBe(false);
    expect(reload).toHaveBeenCalledOnce();
  });
});
