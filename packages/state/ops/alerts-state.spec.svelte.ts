import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { AlertsState, type AlertsSource } from './alerts-state.svelte.js';
import type { OpsAlertAck } from './types.js';
import { JobsState, type JobsSource } from './jobs-state.svelte.js';
import { SchedulesState, type SchedulesSource } from './schedules-state.svelte.js';
import { SyncState, type SyncSource } from './sync-state.svelte.js';
import type { StateStream, StreamFrame } from '../index.js';

// The dedupe store derives standing alerts from the three domain states —
// never from its own fetch — dedupes repeats, clears on success, and holds
// acks until the condition clears or recurs. Specs drive the real states
// with canned sources (no attach: rows are set directly, no sockets).

const events = ((on, onOpen) => {
  onOpen();
  return () => {};
}) as StateStream<StreamFrame>;

const jobsSource = (): JobsSource => ({
  readQueue: vi.fn(async () => ({
    ok: true as const,
    data: { pending: [], blocked: [], running: [], backoff: [], totals: { pending: 0, blocked: 0, running: 0 }, by_kind: {}, truncated: false },
  })),
  readFailed: vi.fn(async () => ({ ok: true as const, data: { tasks: [] } })),
  readAbandoned: vi.fn(async () => ({ ok: true as const, data: { days: 7, by_day: [], worst: [], tasks: [] } })),
  readIndexErrors: vi.fn(async () => ({ ok: true as const, data: [] })),
  events,
});

const schedulesSource = (): SchedulesSource => ({
  readSchedules: vi.fn(async () => ({ ok: true as const, data: [] })),
  patchSchedule: vi.fn(async () => ({ ok: false as const, error: { status: 0, message: 'unused' } })),
  readScheduleLog: vi.fn(async () => ({ ok: false as const, error: { status: 0, message: 'unused' } })),
  events,
});

const syncSource = (): SyncSource => ({
  readSyncState: vi.fn(async () => ({ ok: true as const, data: [] })),
  events,
});

function cannedAlerts(acks: OpsAlertAck[] = []): AlertsSource & { writes: Array<{ cls: string; key: string; acked: boolean }> } {
  const writes: Array<{ cls: string; key: string; acked: boolean }> = [];
  return {
    writes,
    readAcks: vi.fn(async () => ({ ok: true as const, data: { acks } })),
    writeAck: vi.fn(async (cls: string, key: string, acked: boolean) => {
      writes.push({ cls, key, acked });
      return { ok: true as const, data: { class: cls, key, acked, seen_clears: 0, updated_at: '' } };
    }),
  };
}

function states() {
  return { jobs: new JobsState(jobsSource()), schedules: new SchedulesState(schedulesSource()), sync: new SyncState(syncSource()) };
}

const failedTask = (taskId: number, error: string | null = 'boom') => ({
  taskId, parentTaskId: null, kind: 'process_file', folderPath: '/r', path: `/r/f${taskId}.ts`,
  status: 'failed', itemsProcessed: 0, durationMs: null, retryNumber: 0, error,
  startedAt: '2026-11-21T10:00:00.000Z', completedAt: null,
  id: 'row-1', retriedFrom: null, retriedFromTaskId: null,
});

const schedule = (name: string, over: Record<string, unknown> = {}) => ({
  name, description: name, enabled: true, interval_secs: 300, window: null, days: [],
  last_run_at: null, last_ok: null, last_error: null, avg_ms: null, running_since: null,
  last_skip_reason: null, last_skip_at: null, overrun_limit_secs: 600, next_run_at: null, ...over,
});

const syncRow = (entity_key: string, over: Record<string, unknown> = {}) => ({
  entity: 'repository', entity_key, direction: 'push', state: 'synced', last_error: null,
  attempted_at: null, synced_at: null, updated_at: '2026-11-21T10:00:00.000Z', persona: 'alice',
  remedy: null, ...over,
});

describe('AlertsState', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });
  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it('derives one alert per condition across every class', () => {
    const deps = states();
    deps.jobs.failed = [failedTask(7)];
    deps.schedules.items = [schedule('metrics', { last_ok: false, last_error: 'nope' })];
    deps.sync.rows = [syncRow('a/b', { state: 'error', last_error: 'denied' })];
    const st = new AlertsState(deps, cannedAlerts());
    const created = st.refresh();
    expect(created).toHaveLength(3);
    expect(st.alerts.map((a) => a.class).sort()).toEqual(['schedule', 'sync', 'task']);
    expect(st.alerts.find((a) => a.class === 'task')).toMatchObject({ key: '7', level: 'warn', count: 1 });
  });

  it('a repeat bumps the count instead of raising a second alert', () => {
    const deps = states();
    deps.jobs.failed = [failedTask(7)];
    const st = new AlertsState(deps, cannedAlerts());
    st.refresh();
    const again = st.refresh();
    expect(again).toHaveLength(0);
    expect(st.alerts).toHaveLength(1);
    expect(st.alerts[0].count).toBe(2);
  });

  it('success clears the alert; a recurrence escalates one level', () => {
    const deps = states();
    deps.jobs.failed = [failedTask(7)];
    const st = new AlertsState(deps, cannedAlerts());
    st.refresh();
    deps.jobs.failed = [];
    st.refresh();
    expect(st.alerts).toHaveLength(0);
    deps.jobs.failed = [failedTask(7)];
    const recurred = st.refresh();
    expect(recurred).toHaveLength(1);
    expect(st.alerts[0]).toMatchObject({ level: 'critical', escalated: true });
  });

  it('an ack holds while the condition stands and drops when it clears', async () => {
    const deps = states();
    deps.jobs.failed = [failedTask(7)];
    const st = new AlertsState(deps, cannedAlerts());
    st.refresh();
    await st.ack('task', '7');
    expect(st.alerts[0].acked).toBe(true);
    st.refresh();
    expect(st.alerts[0].acked).toBe(true);
    deps.jobs.failed = [];
    st.refresh();
    expect(st.alerts).toHaveLength(0);
  });

  it('an ack writes through; a refused write leaves the alert unacked', async () => {
    const deps = states();
    deps.jobs.failed = [failedTask(7)];
    const source = cannedAlerts();
    const st = new AlertsState(deps, source);
    st.refresh();
    await st.ack('task', '7');
    expect(source.writes).toEqual([{ cls: 'task', key: '7', acked: true }]);
    expect(st.alerts[0].acked).toBe(true);
    source.writeAck = vi.fn(async () => ({ ok: false as const, error: { status: 0, message: 'down' } }));
    deps.jobs.failed = [failedTask(7), failedTask(8)];
    st.refresh();
    const res = await st.ack('task', '8');
    expect(res.ok).toBe(false);
    expect(st.alerts.find((a) => a.key === '8')?.acked).toBe(false);
  });

  it('a clear writes through; a refused write faults the dispositions', async () => {
    const deps = states();
    deps.jobs.failed = [failedTask(7)];
    const source = cannedAlerts();
    const st = new AlertsState(deps, source);
    st.refresh();
    deps.jobs.failed = [];
    st.refresh();
    await vi.waitFor(() => expect(source.writes).toEqual([{ cls: 'task', key: '7', acked: false }]));
    expect(st.dispositionsError).toBeNull();
    // A clear whose write fails faults by name instead of vanishing.
    deps.jobs.failed = [failedTask(9)];
    st.refresh();
    source.writeAck = vi.fn(async () => ({ ok: false as const, error: { status: 0, message: 'down' } }));
    deps.jobs.failed = [];
    st.refresh();
    await vi.waitFor(() => expect(st.dispositionsError).toMatch(/dispositions/));
    // A source that throws instead of answering faults the same slot.
    deps.jobs.failed = [failedTask(11)];
    st.refresh();
    source.writeAck = vi.fn(async () => {
      throw new Error('socket hung up');
    });
    deps.jobs.failed = [];
    st.refresh();
    await vi.waitFor(() => expect(st.dispositionsError).toMatch(/socket hung up/));
  });

  it('hydrated dispositions restore acks and recurrence across restarts', async () => {
    const deps = states();
    deps.jobs.failed = [failedTask(7), failedTask(8)];
    const source = cannedAlerts([
      { class: 'task', key: '7', acked: true, seen_clears: 0, updated_at: '' },
      { class: 'task', key: '8', acked: false, seen_clears: 2, updated_at: '' },
    ]);
    const st = new AlertsState(deps, source);
    await st.loadDispositions();
    st.refresh();
    expect(st.alerts.find((a) => a.key === '7')?.acked).toBe(true);
    expect(st.alerts.find((a) => a.key === '8')).toMatchObject({ escalated: true, level: 'critical' });
  });

  it('a stream fault alerts and clears on the next open', () => {
    const deps = states();
    deps.jobs.streamFault = 'dropped';
    const st = new AlertsState(deps, cannedAlerts());
    expect(st.refresh()).toHaveLength(1);
    expect(st.alerts[0]).toMatchObject({ class: 'stream', key: 'jobs' });
    deps.jobs.streamFault = null;
    st.refresh();
    expect(st.alerts).toHaveLength(0);
  });

  it('an unreachable daemon is one critical, not one per domain', () => {
    const deps = states();
    deps.jobs.daemonReachable = false;
    deps.sync.daemonReachable = false;
    const st = new AlertsState(deps, cannedAlerts());
    const created = st.refresh();
    expect(created).toHaveLength(1);
    expect(created[0]).toMatchObject({ class: 'daemon', level: 'critical' });
  });

  it('a skipped sync row with a remedy refuses quietly at info', () => {
    const deps = states();
    deps.sync.rows = [syncRow('a/b', { state: 'skipped', last_error: 'private', remedy: 'ask an owner' })];
    const st = new AlertsState(deps, cannedAlerts());
    const created = st.refresh();
    expect(created).toHaveLength(1);
    expect(created[0]).toMatchObject({ level: 'info', kind: 'refusal' });
  });

  it('the poller toasts new warnings and criticals, never repeats or infos', async () => {
    const deps = states();
    deps.jobs.failed = [failedTask(7)];
    deps.sync.rows = [syncRow('a/b', { state: 'skipped', last_error: 'private', remedy: 'ask' })];
    const st = new AlertsState(deps, cannedAlerts());
    const toasted: string[] = [];
    st.start((a) => void toasted.push(`${a.class}:${a.key}`), 1000);
    // The opening derive waits one microtask for the hydrate.
    await vi.waitFor(() => expect(toasted).toEqual(['task:7']));
    await vi.advanceTimersByTimeAsync(3000);
    // Repeats never re-toast; the info-level refusal never toasts at all.
    expect(toasted).toEqual(['task:7']);
    deps.schedules.items = [schedule('metrics', { last_ok: false })];
    await vi.advanceTimersByTimeAsync(1000);
    expect(toasted).toEqual(['task:7', 'schedule:metrics']);
    st.stop();
    st.reset();
  });
});
