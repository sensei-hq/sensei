import { describe, it, expect, vi } from 'vitest';
import { alertsSource, jobsSseSource, schedulesSseSource, syncSseSource } from './sse.js';
import type { OpsApi } from '../client.js';
import type {
  OpsQueueListing,
  OpsScheduledTask,
  OpsSyncStateFeed,
  OpsSyncStateRow,
} from '../types.js';

// The sources adapt the client's envelopes to the stores' rows: the daemon
// wraps the schedules list in `{ tasks }` and the sync rows in
// `{ count, counts, entities }`; the stores hydrate from the bare arrays.
describe('ops SSE sources', () => {
  const stream = () => ({}) as never;

  function api(over: Partial<OpsApi> = {}): OpsApi {
    return { tryGetQueue: vi.fn(), tryGetSchedules: vi.fn(), tryGetSyncState: vi.fn(), ...over } as OpsApi;
  }

  it('jobs reads the bare queue listing', async () => {
    const listing = { pending: [], truncated: false } as unknown as OpsQueueListing;
    const a = api({ tryGetQueue: vi.fn(async () => ({ ok: true as const, data: listing })) });
    const res = await jobsSseSource(a, stream).readQueue();
    expect(res).toEqual({ ok: true, data: listing });
  });

  it('jobs delegates the failures reads', async () => {
    const a = api({
      tryGetFailed: vi.fn(async () => ({ ok: true as const, data: { tasks: [] } })),
      tryGetAbandoned: vi.fn(async () => ({ ok: true as const, data: { days: 7 } })),
      tryGetIndexErrors: vi.fn(async () => ({ ok: true as const, data: [] })),
    });
    const source = jobsSseSource(a, stream);
    await source.readFailed();
    await source.readAbandoned();
    await source.readIndexErrors();
    expect(a.tryGetFailed).toHaveBeenCalledOnce();
    expect(a.tryGetAbandoned).toHaveBeenCalledOnce();
    expect(a.tryGetIndexErrors).toHaveBeenCalledOnce();
  });

  it('schedules unwraps the { tasks } envelope', async () => {
    const tasks = [{ name: 'metrics' }] as OpsScheduledTask[];
    const a = api({ tryGetSchedules: vi.fn(async () => ({ ok: true as const, data: { tasks } })) });
    const res = await schedulesSseSource(a, stream).readSchedules();
    expect(res).toEqual({ ok: true, data: tasks });
  });

  it('sync unwraps the { entities } envelope', async () => {
    const entities = [{ entity_key: 'a/b' }] as OpsSyncStateRow[];
    const feed: OpsSyncStateFeed = { count: 1, counts: { synced: 1 }, entities };
    const a = api({ tryGetSyncState: vi.fn(async () => ({ ok: true as const, data: feed })) });
    const res = await syncSseSource(a, stream).readSyncState();
    expect(res).toEqual({ ok: true, data: entities });
  });

  it('alerts delegates disposition reads and writes', async () => {
    const a = api({
      tryGetAlertAcks: vi.fn(async () => ({ ok: true as const, data: { acks: [] } })),
      tryPutAlertAck: vi.fn(async () => ({ ok: true as const, data: {} })),
    });
    const source = alertsSource(a);
    await source.readAcks();
    await source.writeAck('task', '7', true);
    expect(a.tryGetAlertAcks).toHaveBeenCalledOnce();
    expect(a.tryPutAlertAck).toHaveBeenCalledWith('task', '7', true);
  });

  it('a failed read passes through untouched', async () => {
    const error = { status: 0, message: 'down' };
    const a = api({
      tryGetQueue: vi.fn(async () => ({ ok: false as const, error })),
      tryGetSchedules: vi.fn(async () => ({ ok: false as const, error })),
      tryGetSyncState: vi.fn(async () => ({ ok: false as const, error })),
    });
    expect(await jobsSseSource(a, stream).readQueue()).toEqual({ ok: false, error });
    expect(await schedulesSseSource(a, stream).readSchedules()).toEqual({ ok: false, error });
    expect(await syncSseSource(a, stream).readSyncState()).toEqual({ ok: false, error });
  });
});
