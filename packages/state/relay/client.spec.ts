// The relay client's refusals (#426 found the gap): the daemon answers a refusal with
// `{ error: <code>, message: <reason> }`; the person must read the reason, not the code.
import { describe, it, expect, vi, afterEach } from 'vitest';
import { relayClient } from './client.js';

afterEach(() => vi.unstubAllGlobals());

const answer = (status: number, body: unknown) =>
  vi.stubGlobal('fetch', vi.fn(async () => new Response(body === null ? null : JSON.stringify(body), { status })));

describe('relayClient · a refusal', () => {
  it('carries the daemon’s reason (message), not its code', async () => {
    answer(409, { error: 'relay_conflict', message: 'a run is already open in this workspace' });
    const r = await relayClient('http://daemon').tryCreateRun({} as never);
    expect(r).toEqual({
      ok: false,
      error: {
        status: 409,
        message: 'a run is already open in this workspace',
        body: { error: 'relay_conflict', message: 'a run is already open in this workspace' },
      },
    });
  });

  it('a body with a code and no message: the code; no body: the status text', async () => {
    answer(409, { error: 'relay_conflict' });
    expect(await relayClient('http://daemon').tryGetRuns()).toMatchObject({ ok: false, error: { status: 409, message: 'relay_conflict' } });
    vi.stubGlobal('fetch', vi.fn(async () => new Response('not json', { status: 502, statusText: 'Bad Gateway' })));
    expect(await relayClient('http://daemon').tryGetRuns()).toMatchObject({ ok: false, error: { status: 502, message: 'Bad Gateway' } });
  });
});

describe('relayClient · the catalogue (#455)', () => {
  it('reads GET /relay/catalogue', async () => {
    const fetchSpy = vi.fn(async () => new Response(JSON.stringify({ assistants: [], editors: [], unknown: [] }), { status: 200 }));
    vi.stubGlobal('fetch', fetchSpy);
    const r = await relayClient('http://daemon').tryGetCatalogue();
    expect(r).toEqual({ ok: true, data: { assistants: [], editors: [], unknown: [] } });
    expect(fetchSpy).toHaveBeenCalledWith('http://daemon/relay/catalogue', { method: 'GET' });
  });
});
