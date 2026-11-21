import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import {
  SchedulesState,
  pausedSchedules,
  failingSchedules,
  type SchedulesSource,
} from './schedules-state.svelte.js';
import type { OpsScheduledTask } from './types.js';
import type { StateStream, StreamFrame } from '../index.js';

function schedule(name: string, overrides: Partial<OpsScheduledTask> = {}): OpsScheduledTask {
  return {
    name,
    description: `${name} worker`,
    enabled: true,
    interval_secs: 300,
    window: null,
    days: [],
    last_run_at: null,
    last_ok: null,
    last_error: null,
    avg_ms: null,
    running_since: null,
    last_skip_reason: null,
    last_skip_at: null,
    overrun_limit_secs: 600,
    next_run_at: null,
    ...overrides,
  };
}

let frameId = 0;
function frame(entity: string, key: string, data: Record<string, unknown> = {}): StreamFrame {
  return { v: 1, id: ++frameId, ts: '2026-11-21T10:00:00.000Z', entity, action: 'update', key, data };
}

function cannedSchedules(items: OpsScheduledTask[] = []) {
  const stream: { on: ((e: StreamFrame) => void) | null; onOpen: (() => void) | null } = {
    on: null,
    onOpen: null,
  };
  const source: SchedulesSource = {
    readSchedules: vi.fn(async () => ({ ok: true as const, data: items })),
    events: ((on, onOpen) => {
      stream.on = on;
      stream.onOpen = onOpen;
      onOpen();
      return () => {};
    }) as StateStream<StreamFrame>,
  };
  return { source, stream };
}

// ── Selectors ────────────────────────────────────────────────────────────────

describe('schedule selectors', () => {
  const items = [
    schedule('a', { enabled: false }),
    schedule('b', { last_ok: false, last_error: 'boom' }),
    schedule('c', { last_ok: true }),
  ];

  it('pausedSchedules names paused rows only', () => {
    expect(pausedSchedules(items).map((s) => s.name)).toEqual(['a']);
  });

  it('failingSchedules names rows whose last run failed', () => {
    expect(failingSchedules(items).map((s) => s.name)).toEqual(['b']);
  });
});

// ── SchedulesState ───────────────────────────────────────────────────────────

describe('SchedulesState', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it('attach hydrates rows and stays reachable', async () => {
    const { source } = cannedSchedules([schedule('tick')]);
    const st = new SchedulesState(source);
    await st.attach();
    expect(st.items.map((s) => s.name)).toEqual(['tick']);
    expect(st.daemonReachable).toBe(true);
    expect(source.readSchedules).toHaveBeenCalledTimes(1);
    st.reset();
  });

  it('a schedule frame re-reads; foreign frames never do', async () => {
    const { source } = cannedSchedules([schedule('tick')]);
    const st = new SchedulesState(source);
    await st.attach();
    st.apply(frame('job', '7'));
    st.apply(frame('folder', '/x'));
    expect(source.readSchedules).toHaveBeenCalledTimes(1);
    st.apply(frame('schedule', 'tick'));
    expect(source.readSchedules).toHaveBeenCalledTimes(2);
    st.reset();
  });

  it('the poller re-reads rows; a failed read keeps last good', async () => {
    const readSchedules = vi
      .fn()
      .mockResolvedValueOnce({ ok: true, data: [schedule('tick')] })
      .mockResolvedValue({ ok: false, error: { status: 0, message: 'down' } });
    const { source } = cannedSchedules();
    source.readSchedules = readSchedules as typeof source.readSchedules;
    const st = new SchedulesState(source);
    await st.attach();
    await vi.advanceTimersByTimeAsync(61000);
    expect(st.daemonReachable).toBe(false);
    expect(st.items.map((s) => s.name)).toEqual(['tick']);
    st.reset();
  });
});
