import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import {
  JobsState,
  classifyJobFrame,
  waveProgress,
  type JobsSource,
} from './jobs-state.svelte.js';
import type {
  OpsAbandonedFeed,
  OpsBackoffView,
  OpsFailedTask,
  OpsIndexError,
  OpsQueueListing,
  OpsTaskView,
} from './types.js';
import type { StateStream, StreamFrame } from '../index.js';
import type { ApiResult } from '../result.js';

// ── Factories ────────────────────────────────────────────────────────────────

function row(id: number, overrides: Partial<OpsTaskView> = {}): OpsTaskView {
  return {
    id,
    kind: 'process_file',
    path: `/r/f${id}.ts`,
    folder_path: '/r',
    parent_task_id: null,
    retry_number: 0,
    wait_ms: 12,
    running_ms: null,
    blocked_on: [],
    budget_secs: null,
    nearing_budget: false,
    ...overrides,
  };
}

function listing(overrides: Partial<OpsQueueListing> = {}): OpsQueueListing {
  return {
    pending: [],
    blocked: [],
    running: [],
    backoff: [],
    totals: { pending: 0, blocked: 0, running: 0 },
    by_kind: {},
    truncated: false,
    ...overrides,
  };
}

let frameId = 0;
function frame(
  entity: string,
  action: StreamFrame['action'],
  key: string,
  data: Record<string, unknown> = {},
  done = false,
): StreamFrame {
  return { v: 1, id: ++frameId, ts: '2026-11-21T10:00:00.000Z', entity, action, key, data, ...(done ? { done: true as const } : {}) };
}

function failedTask(id: number, overrides: Partial<OpsFailedTask> = {}): OpsFailedTask {
  return {
    taskId: id,
    parentTaskId: null,
    kind: 'process_file',
    folderPath: '/r',
    path: `/r/f${id}.ts`,
    status: 'failed',
    itemsProcessed: 0,
    durationMs: null,
    retryNumber: 0,
    error: 'boom',
    startedAt: '2026-11-21T10:00:00.000Z',
    completedAt: null,
    ...overrides,
  };
}

function cannedJobs(opts: { queue?: OpsQueueListing } = {}) {
  const stream: {
    on: ((e: StreamFrame) => void) | null;
    onOpen: (() => void) | null;
    onFault: ((reason: string) => void) | null;
  } = { on: null, onOpen: null, onFault: null };
  const abandoned: OpsAbandonedFeed = { days: 7, by_day: [], worst: [], tasks: [] };
  const source: JobsSource = {
    readQueue: vi.fn(async () => ({ ok: true as const, data: opts.queue ?? listing() })),
    readFailed: vi.fn(async () => ({ ok: true as const, data: { tasks: [] as OpsFailedTask[] } })),
    readAbandoned: vi.fn(async () => ({ ok: true as const, data: abandoned })),
    readIndexErrors: vi.fn(async () => ({ ok: true as const, data: [] as OpsIndexError[] })),
    events: ((on, onOpen, onFault) => {
      stream.on = on;
      stream.onOpen = onOpen;
      stream.onFault = onFault ?? null;
      onOpen();
      return () => {};
    }) as StateStream<StreamFrame>,
  };
  return { source, stream };
}

// ── classifyJobFrame ─────────────────────────────────────────────────────────

describe('classifyJobFrame', () => {
  it('routes job frames; stages and foreign entities fall through', () => {
    expect(classifyJobFrame(frame('job', 'add', '7', { kind: 'x' }))).toBe('queued');
    expect(classifyJobFrame(frame('job', 'update', '7'))).toBe('started');
    expect(classifyJobFrame(frame('job', 'update', '7', {}, true))).toBe('terminal');
    expect(classifyJobFrame(frame('job', 'update', 'stage:scan_root:/x'))).toBe('stage');
    expect(classifyJobFrame(frame('folder', 'add', '/x'))).toBe('other');
  });
});

describe('waveProgress', () => {
  it('computes share completed, none without a total', () => {
    expect(waveProgress({ wave: 't', key: 'k', total: 4, settled: 1, done: false })).toEqual({
      settled: 1,
      total: 4,
      share: 0.25,
    });
    expect(waveProgress({ wave: 't', key: 'k', total: 0, settled: 0, done: false }).share).toBeNull();
  });
});

// ── JobsState ────────────────────────────────────────────────────────────────

describe('JobsState', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it('attach hydrates the listing and stays live on frames', async () => {
    const { source, stream } = cannedJobs({
      queue: listing({
        pending: [row(1)],
        totals: { pending: 1, blocked: 0, running: 0 },
        by_kind: { process_file: { pending: 1, blocked: 0, running: 0 } },
        truncated: true,
      }),
    });
    const st = new JobsState(source);
    await st.attach();
    expect(st.pending.map((r) => r.id)).toEqual([1]);
    expect(st.totals).toEqual({ pending: 1, blocked: 0, running: 0 });
    expect(st.byKind).toEqual({ process_file: { pending: 1, blocked: 0, running: 0 } });
    expect(st.truncated).toBe(true);
    expect(st.daemonReachable).toBe(true);
    // A queued frame adds a row without a re-read.
    stream.on!(frame('job', 'add', '2', { kind: 'process_file', path: '/r/f2.ts', folder_path: '/r' }));
    expect(st.pending.map((r) => r.id)).toEqual([1, 2]);
    st.reset();
  });

  it('started moves pending to running; terminal removes', async () => {
    const { source, stream } = cannedJobs({ queue: listing({ pending: [row(1)] }) });
    const st = new JobsState(source);
    await st.attach();
    stream.on!(frame('job', 'update', '1'));
    expect(st.pending).toHaveLength(0);
    expect(st.running.map((r) => r.id)).toEqual([1]);
    stream.on!(frame('job', 'update', '1', {}, true));
    expect(st.running).toHaveLength(0);
    st.reset();
  });

  it('a terminal clears a backing-off row by task id', async () => {
    const backoff: OpsBackoffView[] = [
      { task_id: 5, kind: 'process_file', path: '/r/f5.ts', folder_path: '/r', attempt: 2, retry_in_ms: 4000 },
    ];
    const { source, stream } = cannedJobs({ queue: listing({ backoff }) });
    const st = new JobsState(source);
    await st.attach();
    expect(st.backoff.map((r) => r.task_id)).toEqual([5]);
    stream.on!(frame('job', 'update', '5', {}, true));
    expect(st.backoff).toHaveLength(0);
    st.reset();
  });

  it('unknown terminals are no-ops; duplicate adds replace', async () => {
    const { source, stream } = cannedJobs();
    const st = new JobsState(source);
    await st.attach();
    stream.on!(frame('job', 'update', '99', {}, true));
    expect(st.running).toHaveLength(0);
    const data = { kind: 'process_file', path: '/r/f1.ts', folder_path: '/r' };
    stream.on!(frame('job', 'add', '1', data));
    stream.on!(frame('job', 'add', '1', data));
    expect(st.pending).toHaveLength(1);
    st.reset();
  });

  it('wave frames open and close waves; folder frames belong to scan', async () => {
    const { source, stream } = cannedJobs();
    const st = new JobsState(source);
    await st.attach();
    stream.on!(frame('transcript', 'add', '9', { total: 2 }));
    expect(st.waves).toEqual([{ wave: 'transcript', key: '9', total: 2, settled: 0, done: false }]);
    stream.on!(frame('folder', 'add', '/x', { files_total: 3 }));
    expect(st.waves).toHaveLength(1);
    stream.on!(frame('transcript', 'update', '9', { settled: 2, total: 2 }, true));
    expect(st.waves[0]).toMatchObject({ settled: 2, done: true });
    st.reset();
  });

  it('a socket fault is held until the next open re-reads', async () => {
    const { source, stream } = cannedJobs();
    const st = new JobsState(source);
    await st.attach();
    stream.onFault!('the live stream dropped; reconnecting');
    expect(st.streamFault).toBe('the live stream dropped; reconnecting');
    stream.onOpen!();
    expect(st.streamFault).toBeNull();
    expect(source.readQueue).toHaveBeenCalledTimes(2);
    st.reset();
  });

  it('attach loads the failures feed; refresh replaces it', async () => {
    const { source } = cannedJobs();
    const st = new JobsState(source);
    await st.attach();
    expect(st.failuresLoaded).toBe(true);
    expect(st.failed).toEqual([]);
    source.readFailed = vi.fn(async () => ({ ok: true as const, data: { tasks: [failedTask(7)] } }));
    await st.refreshFailures();
    expect(st.failed.map((t) => t.taskId)).toEqual([7]);
    expect(st.failuresError).toBeNull();
    st.reset();
    expect(st.failuresLoaded).toBe(false);
  });

  it('a failed feed read keeps last good and names the feed, not the daemon', async () => {
    const { source } = cannedJobs();
    source.readFailed = vi
      .fn()
      .mockResolvedValueOnce({ ok: true, data: { tasks: [failedTask(7)] } })
      .mockResolvedValue({ ok: false, error: { status: 500, message: 'feed failed closed' } });
    const st = new JobsState(source);
    await st.attach();
    await st.refreshFailures();
    // Last good rows stand; the fault names the feed; reachability is the
    // queue poller's verdict, untouched by a feed that failed closed.
    expect(st.failed.map((t) => t.taskId)).toEqual([7]);
    expect(st.failuresError).toMatch(/failed tasks/);
    expect(st.daemonReachable).toBe(true);
    st.reset();
  });

  it('the poller refreshes waits; a failed read keeps last good', async () => {
    const queue = listing({ pending: [row(1)] });
    const readQueue = vi
      .fn()
      .mockResolvedValueOnce({ ok: true, data: queue })
      .mockResolvedValue({ ok: false, error: { status: 0, message: 'down' } });
    const { source, stream } = cannedJobs();
    source.readQueue = readQueue as typeof source.readQueue;
    const st = new JobsState(source);
    await st.attach();
    expect(st.daemonReachable).toBe(true);
    await vi.advanceTimersByTimeAsync(16000);
    expect(st.daemonReachable).toBe(false);
    expect(st.pending.map((r) => r.id)).toEqual([1]);
    st.reset();
  });
});
