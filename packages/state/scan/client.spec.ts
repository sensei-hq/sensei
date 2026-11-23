import { describe, it, expect, afterEach, vi } from 'vitest';
import { scanClient, scanEvents } from './client.js';

/**
 * The scan client runs for real only here — every screen spec drives state
 * with a canned source, so a mutant that ignores the daemon's refusal text
 * survives them all. The five paths are pinned in one place, so a route
 * change is a loud one.
 */

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

const api = () => scanClient(BASE);

function stub(status: number, body?: unknown, statusText?: string) {
  const fetch = vi.fn().mockResolvedValue(response(status, body, statusText));
  vi.stubGlobal('fetch', fetch);
  return fetch;
}

const calledWith = (fetch: ReturnType<typeof stub>) => {
  const [url, init] = fetch.mock.calls[0] as [string, RequestInit | undefined];
  return { url, method: init?.method, headers: init?.headers, body: init?.body };
};

describe('scanClient — the paths', () => {
  it('lists workspaces', async () => {
    const fetch = stub(200, []);
    const res = await api().tryListWorkspaces();
    expect(res.ok).toBe(true);
    expect(calledWith(fetch)).toEqual({ url: `${BASE}/api/workspaces`, method: 'GET', headers: undefined, body: undefined });
  });

  it('reads the scan roots', async () => {
    const fetch = stub(200, []);
    await api().tryGetScanRoots();
    expect(calledWith(fetch)).toEqual({ url: `${BASE}/api/scan/roots`, method: 'GET', headers: undefined, body: undefined });
  });

  it('triggers a root scan', async () => {
    const fetch = stub(200, { ok: true, scanning: true });
    await api().tryScanFolder('/code');
    expect(calledWith(fetch)).toEqual({
      url: `${BASE}/api/scan`,
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ root: '/code', max_depth: 3 }),
    });
  });

  it('reads the index queue status', async () => {
    const fetch = stub(200, { queue: { pending: 0, blocked: 0, running: 0, completed: 0 }, repos: {} });
    await api().tryGetIndexStatus();
    expect(calledWith(fetch)).toEqual({ url: `${BASE}/api/index/status`, method: 'GET', headers: undefined, body: undefined });
  });
});

describe('scanClient — failures', () => {
  it('a refusal carries the daemon reason, not the status text', async () => {
    stub(409, { message: 'root is paused' }, 'Conflict');
    const res = await api().tryScanFolder('/code');
    expect(res.ok).toBe(false);
    if (!res.ok) {
      expect(res.error.status).toBe(409);
      expect(res.error.message).toMatch(/root is paused/);
    }
  });

  it('a dead daemon is status 0', async () => {
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new TypeError('fetch failed')));
    const res = await api().tryGetIndexStatus();
    expect(res).toEqual({ ok: false, error: { status: 0, message: 'fetch failed' } });
  });
});

describe('scanEvents — the socket', () => {
  it('opens the scan event stream', () => {
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
    const socket = scanEvents(BASE);
    const teardown = socket.subscribe(() => {});
    expect(opened).toBe(`${BASE}/api/scan/events`);
    teardown();
    socket.destroy();
  });
});
