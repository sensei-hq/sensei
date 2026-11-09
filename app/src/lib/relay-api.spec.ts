import { describe, it, expect, afterEach, vi } from 'vitest';
import { relayApi, parseRelayStreamEvent } from './relay-api.js';

/**
 * The runtime behind `relayApi(port)` (review relay-screen-impl-r1 F3): every screen
 * spec mocks `$lib/relay-api`, so this file is the only place the client itself runs —
 * a mutant that ignores the daemon's refusal text survives them all.
 *
 * The six paths are provisional until L3 publishes the daemon's relay routes (Q9): they
 * are pinned here, in one place, so that publishing them is a loud change.
 */

const BASE = 'http://127.0.0.1:7744';
const CREW = '019a5e2c-4b1d-7c00-8d2e-00000000c7f3';

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

const api = () => relayApi(7744);

function stub(status: number, body?: unknown, statusText?: string) {
  const fetch = vi.fn().mockResolvedValue(response(status, body, statusText));
  vi.stubGlobal('fetch', fetch);
  return fetch;
}

const calledWith = (fetch: ReturnType<typeof stub>) => {
  const [url, init] = fetch.mock.calls[0] as [string, RequestInit | undefined];
  return { url, method: init?.method, headers: init?.headers, body: init?.body };
};

describe('relayApi — the paths', () => {
  it('Now and the queue are GET …/relay/now and …/relay/queue, no query when nothing narrows', async () => {
    const fetch = stub(200, { crews: [] });
    expect(await api().tryGetNow({})).toEqual({ ok: true, data: { crews: [] } });
    expect(calledWith(fetch)).toEqual({ url: `${BASE}/relay/now`, method: 'GET', headers: undefined, body: undefined });

    fetch.mockClear();
    await api().tryGetQueue({});
    expect(calledWith(fetch).url).toBe(`${BASE}/relay/queue`);
  });

  it('a narrowed read sends ?workspace_id&repo_key, in that order, encoded', async () => {
    const fetch = stub(200, { triage: [], backlog: [] });
    await api().tryGetQueue({ repo_key: 'github.com/sensei-hq/sensei', workspace_id: 'w1' });
    expect(calledWith(fetch).url).toBe(
      `${BASE}/relay/queue?workspace_id=w1&repo_key=github.com%2Fsensei-hq%2Fsensei`,
    );

    fetch.mockClear();
    await api().tryGetNow({ workspace_id: 'w2' });
    expect(calledWith(fetch).url).toBe(`${BASE}/relay/now?workspace_id=w2`);
  });

  it('a task is read by its crew and task, both encoded into the path', async () => {
    const fetch = stub(200, { task: {}, run: {}, decision: null, handoff: null, history: [], lanes: [] });
    await api().tryGetTask(`${CREW}/x`, 't 1');
    expect(calledWith(fetch).url).toBe(`${BASE}/relay/tasks/${CREW}%2Fx/t%201`);
    expect(calledWith(fetch).method).toBe('GET');
  });

  it('the three writes POST their body as JSON to /relay/messages, /answers and /backlog', async () => {
    const cases: Array<[string, (a: ReturnType<typeof api>) => Promise<unknown>, unknown]> = [
      [
        '/relay/messages',
        (a) => a.trySendMessage({ crew_id: CREW, to: { kind: 'everyone' }, task_id: null, text: 'Stop.', surface: 'desktop' }),
        { crew_id: CREW, to: { kind: 'everyone' }, task_id: null, text: 'Stop.', surface: 'desktop' },
      ],
      [
        '/relay/answers',
        (a) => a.tryAnswerDecision({ decision_id: 'ask-1', option: 'db', note: null, surface: 'dojo' }),
        { decision_id: 'ask-1', option: 'db', note: null, surface: 'dojo' },
      ],
      [
        '/relay/backlog',
        (a) => a.tryAddToBacklog({ ticket_provider: 'jira', ticket_key: 'SEN-58' }),
        { ticket_provider: 'jira', ticket_key: 'SEN-58' },
      ],
    ];
    for (const [path, call, body] of cases) {
      const fetch = stub(200, { id: 'x' });
      expect(await call(api())).toEqual({ ok: true, data: { id: 'x' } });
      const sent = calledWith(fetch);
      expect(sent.url, path).toBe(`${BASE}${path}`);
      expect(sent.method, path).toBe('POST');
      expect(sent.headers, path).toEqual({ 'Content-Type': 'application/json' });
      expect(JSON.parse(sent.body as string), path).toEqual(body);
    }
  });
});

describe('relayApi — the answers', () => {
  it('a refusal keeps the daemon’s own text: the 409’s reason is what the screen shows', async () => {
    stub(409, { error: 'already answered on the dōjō' });
    expect(await api().tryAnswerDecision({ decision_id: 'ask-1', option: 'db', note: null, surface: 'desktop' })).toEqual({
      ok: false,
      error: { status: 409, message: 'already answered on the dōjō' },
    });
  });

  it('an error body with no text, or one that is not JSON, falls back to the status text', async () => {
    stub(500, { error: 42 }, 'Internal Server Error');
    expect(await api().tryGetNow({})).toEqual({
      ok: false,
      error: { status: 500, message: 'Internal Server Error' },
    });

    stub(502, undefined, 'Bad Gateway');
    expect(await api().tryGetNow({})).toEqual({ ok: false, error: { status: 502, message: 'Bad Gateway' } });
  });

  it('204 is an answer with no body', async () => {
    stub(204, undefined);
    const result = await api().tryAddToBacklog({ item_id: 'item-1' });
    expect(result).toEqual({ ok: true, data: undefined });
  });

  it('a request that never reaches the daemon is status 0 with the error’s message', async () => {
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('connection refused')));
    expect(await api().tryGetTask(CREW, 't-header')).toEqual({
      ok: false,
      error: { status: 0, message: 'connection refused' },
    });
  });
});

describe('parseRelayStreamEvent', () => {
  // F8: the relay socket speaks the shared {action, entity, data} contract.
  // Shape-checked here; the route dispatches by entity.
  it('parses a step event with its step', () => {
    expect(
      parseRelayStreamEvent(
        JSON.stringify({
          action: 'add',
          entity: 'step',
          data: { task_id: 't1', step: { key: 'a', title: 'A', state: 'open', reason: null, updated_at: 'x' } },
        }),
      ),
    ).toEqual({
      action: 'add',
      entity: 'step',
      data: { task_id: 't1', step: { key: 'a', title: 'A', state: 'open', reason: null, updated_at: 'x' } },
    });
  });

  it('parses a task notification', () => {
    const event = parseRelayStreamEvent(JSON.stringify({ action: 'update', entity: 'task', data: { op: 'report', task_id: 't1' } }));
    expect(event.entity).toBe('task');
    expect(event.data.task_id).toBe('t1');
  });

  it.each(['not json', '{"action":"add"}', '{"entity":"step"}', '[]', 'null'])('rejects %s', (raw) => {
    expect(() => parseRelayStreamEvent(raw)).toThrow();
  });
});
