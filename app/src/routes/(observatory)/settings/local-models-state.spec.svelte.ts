import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import type { ApiResult, SenseiApi } from '$lib/api.js';
import type { ProvisionModel, ProvisionPhase } from '$lib/types.js';
import {
  LocalModels,
  phaseDisplay,
  phaseError,
  POLL_INTERVAL_MS,
  type Timer,
} from './local-models.svelte.js';

// ── phaseDisplay ────────────────────────────────────────────────────────────

describe('phaseDisplay', () => {
  it('absent -> "not pulled", muted, no progress, actionable, not in flight', () => {
    expect(phaseDisplay({ phase: 'absent' })).toEqual({
      label: 'not pulled', tone: 'ink-mute', percent: null, actionable: true, inFlight: false,
    });
  });

  it('queued -> "queued…", soft ink, in flight, not actionable', () => {
    expect(phaseDisplay({ phase: 'queued' })).toEqual({
      label: 'queued…', tone: 'ink-soft', percent: null, actionable: false, inFlight: true,
    });
  });

  it('downloading with a known total -> percent label + numeric percent, accent', () => {
    expect(phaseDisplay({ phase: 'downloading', done: 25, total: 100 })).toEqual({
      label: 'downloading 25%', tone: 'accent', percent: 25, actionable: false, inFlight: true,
    });
  });

  it('downloading rounds the percent', () => {
    // 2/3 = 66.66… -> 67
    expect(phaseDisplay({ phase: 'downloading', done: 2, total: 3 }).percent).toBe(67);
    expect(phaseDisplay({ phase: 'downloading', done: 2, total: 3 }).label).toBe('downloading 67%');
  });

  it('downloading with an unknown total -> ellipsis label, null percent', () => {
    expect(phaseDisplay({ phase: 'downloading', done: 42, total: null })).toEqual({
      label: 'downloading…', tone: 'accent', percent: null, actionable: false, inFlight: true,
    });
  });

  it('verifying -> "verifying…", in flight', () => {
    expect(phaseDisplay({ phase: 'verifying' })).toMatchObject({
      label: 'verifying…', tone: 'ink-soft', inFlight: true, actionable: false, percent: null,
    });
  });

  it('loading -> "loading…", in flight', () => {
    expect(phaseDisplay({ phase: 'loading' })).toMatchObject({
      label: 'loading…', tone: 'ink-soft', inFlight: true, actionable: false, percent: null,
    });
  });

  it('ready -> "ready", success, 100%, not actionable, not in flight', () => {
    expect(phaseDisplay({ phase: 'ready' })).toEqual({
      label: 'ready', tone: 'success', percent: 100, actionable: false, inFlight: false,
    });
  });

  it('failed -> "failed", warning, actionable (retry), not in flight', () => {
    expect(phaseDisplay({ phase: 'failed', error: 'disk full' })).toEqual({
      label: 'failed', tone: 'warning', percent: null, actionable: true, inFlight: false,
    });
  });

  it('an unknown phase degrades to a neutral, non-actionable label (wire-API-wins)', () => {
    const weird = { phase: 'teleporting' } as unknown as ProvisionPhase;
    expect(phaseDisplay(weird)).toEqual({
      label: 'unknown', tone: 'ink-mute', percent: null, actionable: false, inFlight: false,
    });
  });
});

describe('phaseError', () => {
  it('carries the failed error string for a tooltip', () =>
    expect(phaseError({ phase: 'failed', error: 'no space left' })).toBe('no space left'));
  it('is empty for any non-failed phase', () => {
    expect(phaseError({ phase: 'ready' })).toBe('');
    expect(phaseError({ phase: 'downloading', done: 1, total: 2 })).toBe('');
  });
});

// ── LocalModels controller ──────────────────────────────────────────────────

const model = (over: Partial<ProvisionModel> = {}): ProvisionModel => ({
  id: 'gemma2:2b',
  name: 'Gemma 2 2B Instruct',
  phase: { phase: 'absent' },
  ...over,
});

// A hand-rolled api mock — only the two provisioning methods are exercised.
function mockApi(overrides: Partial<SenseiApi> = {}): SenseiApi {
  return {
    provisionStatus: vi.fn().mockResolvedValue({ models: [model()] }),
    provisionModel: vi.fn().mockResolvedValue(
      { ok: true, data: { model: 'gemma2:2b', phase: { phase: 'queued' } } } as ApiResult<{
        model: string; phase: ProvisionPhase;
      }>,
    ),
    ...overrides,
  } as unknown as SenseiApi;
}

// Real timer seam so poll tests can drive it with vi's fake timers.
const timer: Timer = {
  set: (fn, ms) => setTimeout(fn, ms),
  clear: (h) => clearTimeout(h),
};

describe('LocalModels.load', () => {
  it('fetches status, populates models, clears loading', async () => {
    const provisionStatus = vi.fn().mockResolvedValue({ models: [model({ phase: { phase: 'ready' } })] });
    const c = new LocalModels(mockApi({ provisionStatus }), timer);
    await c.load();
    expect(provisionStatus).toHaveBeenCalledOnce();
    expect(c.models).toHaveLength(1);
    expect(c.models[0].phase).toEqual({ phase: 'ready' });
    expect(c.loading).toBe(false);
    c.dispose();
  });
});

describe('LocalModels.pull', () => {
  it('posts the provision, optimistically flips the row to the returned phase', async () => {
    const provisionModel = vi.fn().mockResolvedValue({
      ok: true, data: { model: 'gemma2:2b', phase: { phase: 'queued' } },
    });
    const provisionStatus = vi.fn().mockResolvedValue({ models: [model({ phase: { phase: 'absent' } })] });
    const c = new LocalModels(mockApi({ provisionModel, provisionStatus }), timer);
    await c.load();

    await c.pull('gemma2:2b');
    expect(provisionModel).toHaveBeenCalledWith('gemma2:2b');
    // Immediate feedback: the row shows queued before any poll runs.
    expect(c.models[0].phase).toEqual({ phase: 'queued' });
    expect(c.error).toBeNull();
    expect(c.notice).toBeNull();
    c.dispose();
  });

  it('a 501 sets the not-available notice and leaves the row unchanged', async () => {
    const provisionModel = vi.fn().mockResolvedValue({
      ok: false, error: { status: 501, message: 'embedded provisioning not available in this build' },
    });
    const provisionStatus = vi.fn().mockResolvedValue({ models: [model({ phase: { phase: 'absent' } })] });
    const c = new LocalModels(mockApi({ provisionModel, provisionStatus }), timer);
    await c.load();

    await c.pull('gemma2:2b');
    expect(c.notice).toMatch(/aren.t available in this build/i);
    expect(c.error).toBeNull();
    expect(c.models[0].phase).toEqual({ phase: 'absent' });
    c.dispose();
  });

  it('a non-501 failure surfaces the wire error, no notice', async () => {
    const provisionModel = vi.fn().mockResolvedValue({
      ok: false, error: { status: 500, message: 'boom' },
    });
    const c = new LocalModels(mockApi({ provisionModel }), timer);
    await c.load();

    await c.pull('gemma2:2b');
    expect(c.error).toBe('boom');
    expect(c.notice).toBeNull();
    c.dispose();
  });
});

describe('LocalModels polling lifecycle', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('polls while a model is in flight and stops when all settle', async () => {
    // Sequence: pull -> queued (in flight); poll1 -> downloading (in flight);
    // poll2 -> ready (settles, poll stops).
    const provisionStatus = vi
      .fn()
      .mockResolvedValueOnce({ models: [model({ phase: { phase: 'absent' } })] }) // load
      .mockResolvedValueOnce({ models: [model({ phase: { phase: 'downloading', done: 1, total: 2 } })] })
      .mockResolvedValueOnce({ models: [model({ phase: { phase: 'ready' } })] });
    const provisionModel = vi.fn().mockResolvedValue({
      ok: true, data: { model: 'gemma2:2b', phase: { phase: 'queued' } },
    });
    const c = new LocalModels(mockApi({ provisionStatus, provisionModel }), timer);

    await c.load();
    await c.pull('gemma2:2b');
    expect(c.anyInFlight).toBe(true); // queued

    // First poll tick.
    await vi.advanceTimersByTimeAsync(POLL_INTERVAL_MS);
    expect(provisionStatus).toHaveBeenCalledTimes(2); // load + poll1
    expect(c.models[0].phase).toMatchObject({ phase: 'downloading' });
    expect(c.anyInFlight).toBe(true);

    // Second poll tick -> ready.
    await vi.advanceTimersByTimeAsync(POLL_INTERVAL_MS);
    expect(provisionStatus).toHaveBeenCalledTimes(3);
    expect(c.models[0].phase).toEqual({ phase: 'ready' });
    expect(c.anyInFlight).toBe(false);

    // No further polls once settled.
    await vi.advanceTimersByTimeAsync(POLL_INTERVAL_MS * 3);
    expect(provisionStatus).toHaveBeenCalledTimes(3);
    c.dispose();
  });

  it('dispose() stops the poll timer — no fetch after disposal', async () => {
    const provisionStatus = vi
      .fn()
      .mockResolvedValue({ models: [model({ phase: { phase: 'downloading', done: 1, total: 4 } })] });
    const c = new LocalModels(mockApi({ provisionStatus }), timer);
    await c.load(); // in flight -> schedules a poll
    expect(c.anyInFlight).toBe(true);

    c.dispose();
    await vi.advanceTimersByTimeAsync(POLL_INTERVAL_MS * 3);
    // Only the initial load fetched; the disposed timer never fired.
    expect(provisionStatus).toHaveBeenCalledTimes(1);
  });

  it('does not schedule overlapping polls (a single timer at a time)', async () => {
    // A slow status fetch that outlasts the interval must not stack polls.
    // Hold the pending poll's resolver in an object so TS doesn't narrow it to
    // `never` (the assignment lives inside the mock closure).
    const hang: { resolve: ((v: { models: ProvisionModel[] }) => void) | null } = { resolve: null };
    const provisionStatus = vi
      .fn()
      // load resolves immediately, in flight.
      .mockResolvedValueOnce({ models: [model({ phase: { phase: 'downloading', done: 1, total: 4 } })] })
      // first poll hangs until we resolve it.
      .mockImplementationOnce(
        () => new Promise<{ models: ProvisionModel[] }>((r) => { hang.resolve = r; }),
      )
      .mockResolvedValue({ models: [model({ phase: { phase: 'ready' } })] });
    const c = new LocalModels(mockApi({ provisionStatus }), timer);

    await c.load();
    await vi.advanceTimersByTimeAsync(POLL_INTERVAL_MS); // poll1 starts, hangs
    expect(provisionStatus).toHaveBeenCalledTimes(2);

    // Advance well past another interval while poll1 is still pending — no new poll.
    await vi.advanceTimersByTimeAsync(POLL_INTERVAL_MS * 2);
    expect(provisionStatus).toHaveBeenCalledTimes(2);

    // Let poll1 finish (ready) — poll stops, still exactly 2 calls.
    hang.resolve?.({ models: [model({ phase: { phase: 'ready' } })] });
    await vi.advanceTimersByTimeAsync(0);
    expect(provisionStatus).toHaveBeenCalledTimes(2);
    expect(c.anyInFlight).toBe(false);
    c.dispose();
  });
});
