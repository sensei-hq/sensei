import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { SyncState, classifySyncStage, type SyncSource } from './sync-state.svelte.js';
import type { OpsSyncStateRow, OpsWaveRun } from './types.js';
import type { StateStream, StreamFrame } from '../index.js';

function syncRow(entityKey: string, overrides: Partial<OpsSyncStateRow> = {}): OpsSyncStateRow {
  return {
    entity: 'repo',
    entity_key: entityKey,
    direction: 'push',
    state: 'synced',
    last_error: null,
    attempted_at: null,
    synced_at: null,
    updated_at: '2026-11-21T10:00:00.000Z',
    persona: 'alice',
    remedy: null,
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
  error?: { code: string; message: string },
): StreamFrame {
  return {
    v: 1,
    id: ++frameId,
    ts: '2026-11-21T10:00:00.000Z',
    entity,
    action,
    key,
    data,
    ...(done ? { done: true as const } : {}),
    ...(error ? { error } : {}),
  };
}

function cannedSync(rows: OpsSyncStateRow[] = []) {
  const stream: {
    on: ((e: StreamFrame) => void) | null;
    onOpen: (() => void) | null;
    onFault: ((reason: string) => void) | null;
  } = { on: null, onOpen: null, onFault: null };
  const source: SyncSource = {
    readSyncState: vi.fn(async () => ({ ok: true as const, data: rows })),
    readWaves: vi.fn(async () => ({ ok: true as const, data: { waves: [] as OpsWaveRun[] } })),
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

// ── classifySyncStage ────────────────────────────────────────────────────────

describe('classifySyncStage', () => {
  it('parses plan/push stage keys; rejects scan stages and ids', () => {
    expect(classifySyncStage('stage:sync_plan:alice')).toEqual({ stage: 'plan', persona: 'alice' });
    expect(classifySyncStage('stage:sync_push:alice')).toEqual({ stage: 'push', persona: 'alice' });
    expect(classifySyncStage('stage:scan_root:/x')).toBeNull();
    expect(classifySyncStage('7')).toBeNull();
  });
});

// ── SyncState ────────────────────────────────────────────────────────────────

describe('SyncState', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it('attach hydrates sync rows and stays reachable', async () => {
    const { source } = cannedSync([syncRow('a/b')]);
    const st = new SyncState(source);
    await st.attach();
    expect(st.rows.map((r) => r.entity_key)).toEqual(['a/b']);
    expect(st.daemonReachable).toBe(true);
    st.reset();
  });

  it('plan/push stages track activity; a failure keeps its reason', async () => {
    const { source } = cannedSync([syncRow('a/b')]);
    const st = new SyncState(source);
    await st.attach();
    st.apply(frame('job', 'update', 'stage:sync_plan:alice', { stage: 'sync_plan', path: 'alice' }));
    expect(st.activity.alice).toEqual({ stage: 'plan', state: 'started' });
    st.apply(
      frame('job', 'update', 'stage:sync_plan:alice', { stage: 'sync_plan', path: 'alice', items: 1 }, true),
    );
    expect(st.activity.alice).toEqual({ stage: 'plan', state: 'completed', items: 1 });
    st.apply(
      frame('job', 'update', 'stage:sync_push:alice', { stage: 'sync_push', path: 'alice' }, true, {
        code: 'task-failed',
        message: 'push refused',
      }),
    );
    expect(st.activity.alice).toEqual({ stage: 'push', state: 'failed', error: 'push refused' });
    st.reset();
  });

  it('scan stages and queue ids never record; sync stages need no row', async () => {
    const { source } = cannedSync();
    const st = new SyncState(source);
    await st.attach();
    st.apply(frame('job', 'update', 'stage:scan_root:/x', { stage: 'scan_root', path: '/x' }));
    st.apply(frame('job', 'add', '3', { kind: 'process_file' }));
    expect(st.activity).toEqual({});
    // Activity is transient pass state — it records even when no sync-state
    // row names the persona yet.
    st.apply(frame('job', 'update', 'stage:sync_plan:bob', { stage: 'sync_plan', path: 'bob' }));
    expect(st.activity.bob).toEqual({ stage: 'plan', state: 'started' });
    st.reset();
  });

  it('sync waves open and close; job ids never touch waves', async () => {
    const { source } = cannedSync();
    const st = new SyncState(source);
    await st.attach();
    st.apply(frame('sync', 'add', 'pass-1', { total: 2 }));
    st.apply(frame('job', 'add', '3', { kind: 'process_file' }));
    expect(st.waves).toEqual([{ wave: 'sync', key: 'pass-1', total: 2, settled: 0, done: false }]);
    st.apply(frame('sync', 'update', 'pass-1', { settled: 2, total: 2 }, true));
    expect(st.waves[0]).toMatchObject({ settled: 2, done: true });
    st.reset();
  });

  it('attach loads pass history; a terminal re-reads it', async () => {
    const run: OpsWaveRun = {
      wave: 'sync', key: 'pass-1', parent_task_id: null, total: 2, settled: 2,
      outcome: 'done', opened_at: '', closed_at: '',
    };
    const { source, stream } = cannedSync();
    source.readWaves = vi.fn(async () => ({ ok: true as const, data: { waves: [run] } }));
    const st = new SyncState(source);
    await st.attach();
    expect(st.waveHistory.map((r) => r.key)).toEqual(['pass-1']);
    stream.on!(frame('sync', 'update', 'pass-1', { settled: 2, total: 2 }, true));
    await vi.waitFor(() => expect(source.readWaves).toHaveBeenCalledTimes(2));
    st.reset();
    expect(st.waveHistory).toEqual([]);
  });

  it('a socket fault is held until the next open re-reads', async () => {
    const { source, stream } = cannedSync();
    const st = new SyncState(source);
    await st.attach();
    stream.onFault!('the live stream dropped; reconnecting');
    expect(st.streamFault).toBe('the live stream dropped; reconnecting');
    stream.onOpen!();
    expect(st.streamFault).toBeNull();
    expect(source.readSyncState).toHaveBeenCalledTimes(2);
    st.reset();
  });
});
