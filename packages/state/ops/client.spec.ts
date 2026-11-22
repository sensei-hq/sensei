import { describe, it, expect, afterEach, vi } from 'vitest';
import { opsClient, opsEvents } from './client.js';

// The ops client runs for real only here — every state spec drives a canned
// source, so the eight paths are pinned in one place, loud on change.

const BASE = 'http://127.0.0.1:7744';

const response = (status: number, body: unknown, statusText = 'Status Text') =>
  ({
    ok: status >= 200 && status < 300,
    status,
    statusText,
    json: async () => {
      if (body === undefined) throw new SyntaxError('no body');
      return body;
    },
  }) as unknown as Response;

afterEach(() => {
  vi.unstubAllGlobals();
});

const api = () => opsClient(BASE);

function stub(status: number, body?: unknown, statusText?: string) {
  const fetch = vi.fn().mockResolvedValue(response(status, body, statusText));
  vi.stubGlobal('fetch', fetch);
  return fetch;
}

const calledWith = (fetch: ReturnType<typeof stub>) => {
  const [url, init] = fetch.mock.calls[0] as [string, RequestInit | undefined];
  return { url, method: init?.method, headers: init?.headers, body: init?.body };
};

describe('opsClient — the paths', () => {
  it('reads the queue listing', async () => {
    const fetch = stub(200, { pending: [], blocked: [], running: [], backoff: [] });
    await api().tryGetQueue(50);
    expect(calledWith(fetch).url).toBe(`${BASE}/api/tasks/queue?limit=50`);
  });

  it('reads the failed feed', async () => {
    const fetch = stub(200, { tasks: [] });
    await api().tryGetFailed(7, 100);
    expect(calledWith(fetch).url).toBe(`${BASE}/api/tasks/failed?days=7&limit=100`);
  });

  it('reads abandonments', async () => {
    const fetch = stub(200, { days: 7, by_day: [], worst: [], tasks: [] });
    await api().tryGetAbandoned(7, 50);
    expect(calledWith(fetch).url).toBe(`${BASE}/api/tasks/abandoned?days=7&limit=50`);
  });

  it('reads index errors', async () => {
    const fetch = stub(200, []);
    await api().tryGetIndexErrors();
    expect(calledWith(fetch)).toEqual({ url: `${BASE}/api/index/errors`, method: 'GET', headers: undefined, body: undefined });
  });

  it('reads schedules', async () => {
    const fetch = stub(200, { tasks: [] });
    await api().tryGetSchedules();
    expect(calledWith(fetch).url).toBe(`${BASE}/api/tasks/scheduled`);
  });

  it('patches a schedule', async () => {
    const fetch = stub(200, { name: 'metrics' });
    await api().tryPatchSchedule('metrics', { interval_secs: 60 });
    const call = calledWith(fetch);
    expect(call.url).toBe(`${BASE}/api/tasks/scheduled/metrics`);
    expect(call.method).toBe('PATCH');
    expect(JSON.parse(call.body as string)).toEqual({ interval_secs: 60 });
  });

  it('reads a schedule log tail', async () => {
    const fetch = stub(200, { worker: 'metrics', file: 'x', lines: [] });
    await api().tryGetScheduleLog('metrics', 25);
    expect(calledWith(fetch).url).toBe(`${BASE}/api/tasks/scheduled/metrics/log?lines=25`);
  });

  it('reads sync state', async () => {
    const feed = { count: 0, counts: {}, entities: [] };
    const fetch = stub(200, feed);
    const res = await api().tryGetSyncState();
    expect(calledWith(fetch).url).toBe(`${BASE}/api/dojo/sync-state`);
    expect(res).toEqual({ ok: true, data: feed });
  });

  it('reads one task with attempts and children', async () => {
    const detail = { taskId: 7, current: null, attempts: [], children: [], childSummary: {} };
    const fetch = stub(200, detail);
    const res = await api().tryGetTask(7);
    expect(calledWith(fetch).url).toBe(`${BASE}/api/tasks/7`);
    expect(res).toEqual({ ok: true, data: detail });
  });

  it('retries a failed task', async () => {
    const fetch = stub(200, { taskId: 9 });
    const res = await api().tryRetryTask(7);
    const call = calledWith(fetch);
    expect(call.url).toBe(`${BASE}/api/tasks/7/retry`);
    expect(call.method).toBe('POST');
    expect(res).toEqual({ ok: true, data: { taskId: 9 } });
  });

  it('starts a transcript backfill', async () => {
    const fetch = stub(200, { ok: true, queued: true, taskId: 11, from: null });
    const res = await api().tryStartBackfill();
    const call = calledWith(fetch);
    expect(call.url).toBe(`${BASE}/api/transcripts/backfill`);
    expect(call.method).toBe('POST');
    expect(res).toEqual({ ok: true, data: { ok: true, queued: true, taskId: 11, from: null } });
  });

  it('an already-running backfill answers without a wave id', async () => {
    stub(200, { ok: true, queued: false, running: true });
    const res = await api().tryStartBackfill();
    expect(res).toEqual({ ok: true, data: { ok: true, queued: false, running: true } });
  });

  it('reads alert dispositions', async () => {
    const acks = { acks: [{ class: 'task', key: '7', acked: true, seen_clears: 0 }] };
    const fetch = stub(200, acks);
    const res = await api().tryGetAlertAcks();
    expect(calledWith(fetch).url).toBe(`${BASE}/api/ops/alerts/acks`);
    expect(res).toEqual({ ok: true, data: acks });
  });

  it('writes an ack disposition', async () => {
    const row = { class: 'task', key: '7', acked: true, seen_clears: 0 };
    const fetch = stub(200, row);
    const res = await api().tryPutAlertAck('task', '7', true);
    const call = calledWith(fetch);
    expect(call.url).toBe(`${BASE}/api/ops/alerts/acks`);
    expect(call.method).toBe('PUT');
    expect(JSON.parse(call.body as string)).toEqual({ class: 'task', key: '7', acked: true });
    expect(res).toEqual({ ok: true, data: row });
  });

  it('reads wave runs, filterable by wave', async () => {
    const runs = { waves: [] };
    const fetch = stub(200, runs);
    const res = await api().tryGetWaves('transcript', 10);
    expect(calledWith(fetch).url).toBe(`${BASE}/api/tasks/waves?wave=transcript&limit=10`);
    expect(res).toEqual({ ok: true, data: runs });
  });

  it('reads all waves unfiltered', async () => {
    const fetch = stub(200, { waves: [] });
    await api().tryGetWaves();
    expect(calledWith(fetch).url).toBe(`${BASE}/api/tasks/waves?limit=50`);
  });
});

describe('opsClient — failures', () => {
  it('a refusal carries the daemon reason, not the status text', async () => {
    stub(400, { message: 'days must be a positive number of days' }, 'Bad Request');
    const res = await api().tryGetFailed(-1, 10);
    expect(res.ok).toBe(false);
    if (!res.ok) {
      expect(res.error.status).toBe(400);
      expect(res.error.message).toMatch(/positive number/);
    }
  });
});

describe('opsEvents — the socket', () => {
  it('opens the task stream with validated frames', () => {
    let opened = '';
    vi.stubGlobal(
      'EventSource',
      class {
        constructor(url: string) {
          opened = url;
        }
        close() {}
        addEventListener() {}
        removeEventListener() {}
      },
    );
    const socket = opsEvents(BASE);
    const teardown = socket.subscribe(() => {});
    expect(opened).toBe(`${BASE}/api/tasks/progress`);
    teardown();
    socket.destroy();
  });
});
