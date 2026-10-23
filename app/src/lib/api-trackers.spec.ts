import { describe, it, expect, afterEach, vi } from 'vitest';
import { senseiApi } from './api.js';

/**
 * The tracker routes answer failures with a fixed code and, for some codes, the
 * values the UI must name (`board_already_connected` → the owning workspace;
 * `board_not_found` → the normalised site and board). Collapsing those into a
 * status text would strip exactly what the Integrations screen shows, so the
 * wrappers keep the code as the message and the parsed body beside it.
 */

const response = (status: number, body: unknown, statusText = 'X') =>
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

const api = () => senseiApi(7744);
const P = '11111111-1111-1111-1111-111111111111';
const T = '22222222-2222-2222-2222-222222222222';

function stub(status: number, body?: unknown) {
  const fetch = vi.fn().mockResolvedValue(response(status, body));
  vi.stubGlobal('fetch', fetch);
  return fetch;
}

describe('tracker wrappers', () => {
  it('list reads GET …/trackers', async () => {
    const fetch = stub(200, []);
    expect(await api().tryListTrackers(P)).toEqual({ ok: true, data: [] });
    expect(fetch.mock.calls[0][0]).toBe(`http://127.0.0.1:7744/api/workspaces/${P}/trackers`);
  });

  it('connect POSTs the kind, the level and the adapter body to …/trackers', async () => {
    const fetch = stub(201, { tracker: { id: 't' }, first_sync: { pending: true } });
    const body = { board_url: 'https://a.atlassian.net/x/boards/4', email: 'e@x.io', api_token: 'tok' };
    const res = await api().tryConnectTracker(P, 'jira', 'epic', body);
    expect(res.ok).toBe(true);
    const [url, init] = fetch.mock.calls[0];
    expect(url).toBe(`http://127.0.0.1:7744/api/workspaces/${P}/trackers`);
    expect(init.method).toBe('POST');
    expect(JSON.parse(init.body)).toEqual({ kind: 'jira', level: 'epic', ...body });
  });

  it('sync POSTs …/trackers/{tracker}/sync and disconnect DELETEs …/trackers/{tracker} (204, no body)', async () => {
    const fetch = stub(200, { counts: { sprints: 1 } });
    expect((await api().trySyncTracker(P, T)).ok).toBe(true);
    expect(fetch.mock.calls[0][0]).toBe(`http://127.0.0.1:7744/api/workspaces/${P}/trackers/${T}/sync`);
    expect(fetch.mock.calls[0][1].method).toBe('POST');

    const del = stub(204);
    expect(await api().tryDisconnectTracker(P, T)).toEqual({ ok: true, data: undefined });
    expect(del.mock.calls[0][0]).toBe(`http://127.0.0.1:7744/api/workspaces/${P}/trackers/${T}`);
    expect(del.mock.calls[0][1].method).toBe('DELETE');
  });

  it('the usage reads hit one tracker\'s routes, encoding the sprint and the key', async () => {
    const fetch = stub(200, []);
    await api().tryListSprints(P, T);
    await api().tryListTrackerTasks(P, T, 'sprint-1');
    await api().tryListTrackerTasks(P, T);
    await api().tryGetTrackerTask(P, 'AUTH 1/2');
    await api().tryListUnlinkedSessions(P, T, 'sprint-1');
    const base = `http://127.0.0.1:7744/api/workspaces/${P}`;
    expect(fetch.mock.calls.map((c) => c[0])).toEqual([
      `${base}/trackers/${T}/sprints`,
      `${base}/trackers/${T}/tasks?sprint=sprint-1`,
      `${base}/trackers/${T}/tasks`,
      `${base}/tracker-tasks/AUTH%201%2F2`,
      `${base}/trackers/${T}/sessions/unlinked?sprint=sprint-1`,
    ]);
    expect(fetch.mock.calls.every((c) => (c[1]?.method ?? 'GET') === 'GET')).toBe(true);
  });

  it('a refusal carries the code as the message and the body beside it', async () => {
    const owner = { error: 'board_already_connected', workspace: { id: 'p2', name: 'other' } };
    stub(409, owner);
    const res = await api().tryConnectTracker(P, 'jira', 'sprint', {});
    expect(res).toEqual({
      ok: false,
      error: { status: 409, message: 'board_already_connected', body: owner },
    });
  });

  it('a non-JSON refusal keeps the status text', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(response(500, undefined, 'Internal Server Error')));
    const res = await api().tryListTrackers(P);
    expect(res).toEqual({ ok: false, error: { status: 500, message: 'Internal Server Error' } });
  });

  it('an unreachable daemon is status 0, never an empty list', async () => {
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new TypeError('connection refused')));
    const res = await api().tryListTrackers(P);
    expect(res.ok).toBe(false);
    if (!res.ok) expect(res.error.status).toBe(0);
  });
});
