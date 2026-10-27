import { describe, it, expect, vi } from 'vitest';
import { InstrumentsState, type InstrumentsApi, type InstrumentsStore } from './instruments-state.svelte';

const ok = <T>(data: T) => ({ ok: true as const, data });
const fail = (status: number, message: string) => ({ ok: false as const, error: { status, message } });

function api(over: Partial<InstrumentsApi> = {}): InstrumentsApi {
  return {
    tryCallServerTool: vi.fn().mockResolvedValue(
      ok({ result: { content: [{ type: 'text', text: '{"callers":["a"]}' }] }, isError: false, duration_ms: 42 }),
    ),
    tryGetSessions: vi.fn().mockResolvedValue(ok({ sessions: [{ id: 's1', task: 't', startedAt: '2026-10-20T00:00:00Z', ftr: 1 }] })),
    tryGetObservatoryFtr: vi.fn().mockResolvedValue(ok({ rate: 0.5 })),
    ...over,
  } as InstrumentsApi;
}

function store(over: Partial<InstrumentsStore> = {}): InstrumentsStore {
  const summary = { used: 1, partial: 0, ignored: 0, total: 1 };
  return {
    loadSessionReplay: vi.fn().mockResolvedValue({ replay: { calls: [{ callId: 1 }], summary }, error: null }),
    ...over,
  } as InstrumentsStore;
}

const tool = {
  serverId: 'srv',
  name: 'get_callers',
  schema: { type: 'object', properties: { name: { type: 'string' }, depth: { type: 'integer' } }, required: ['name'] },
};

describe('InstrumentsState — the playground', () => {
  it('runs a tool through its server with typed arguments, and shows its answer', async () => {
    const s = new InstrumentsState('playground');
    s.toolParams = { name: 'scan_root', depth: '2' };
    const a = api();
    await s.execute(a, tool);
    expect(a.tryCallServerTool).toHaveBeenCalledWith('srv', 'get_callers', { name: 'scan_root', depth: 2 });
    expect(JSON.parse(s.toolResult)).toEqual({ callers: ['a'] });
    expect(s.durationMs).toBe(42);
    expect(s.toolError).toBeNull();
    expect(s.executing).toBe(false);
  });

  it('shows a tool’s own failure as a failure, with what it said', async () => {
    const s = new InstrumentsState('playground');
    s.toolParams = { name: 'x' };
    const said = ok({ result: { content: [{ type: 'text', text: 'no such symbol' }] }, isError: true, duration_ms: 3 });
    await s.execute(api({ tryCallServerTool: vi.fn().mockResolvedValue(said) }), tool);
    expect(s.toolError).toBe('The tool reported an error');
    expect(s.toolResult).toBe('no such symbol');
  });

  it('says why a call failed instead of showing an empty answer (#288)', async () => {
    const s = new InstrumentsState('playground');
    s.toolParams = { name: 'x' };
    await s.execute(api({ tryCallServerTool: vi.fn().mockResolvedValue(fail(502, 'spawn failed')) }), tool);
    expect(s.toolResult).toBe('');
    expect(s.toolError).toContain('spawn failed');
    expect(s.executing).toBe(false);
  });

  it('refuses arguments the schema does not allow, without calling', async () => {
    const s = new InstrumentsState('playground');
    s.toolParams = { name: '' };
    const a = api();
    await s.execute(a, tool);
    expect(a.tryCallServerTool).not.toHaveBeenCalled();
    expect(s.toolError).toBe('name is required');
  });
});

describe('InstrumentsState — picking a tool', () => {
  it('starts the new tool clean: its own inputs, no earlier answer or failure', async () => {
    const s = new InstrumentsState('playground');
    s.toolParams = { name: 'x' };
    await s.execute(api({ tryCallServerTool: vi.fn().mockResolvedValue(fail(500, 'boom')) }), tool);
    s.pickTool('srv:search', { query: 'x' });
    expect(s.selectedToolId).toBe('srv:search');
    expect(s.toolParams).toEqual({ query: 'x' });
    expect(s.toolResult).toBe('');
    expect(s.toolError).toBeNull();
    expect(s.durationMs).toBeNull();
  });
});

describe('InstrumentsState — replay', () => {
  it('reads the session list once', async () => {
    const s = new InstrumentsState('replay');
    const a = api();
    await s.loadReplaySessions(a);
    await s.loadReplaySessions(a);
    expect(a.tryGetSessions).toHaveBeenCalledTimes(1);
    expect(s.replaySessions.map((r) => r.id)).toEqual(['s1']);
    expect(s.replayLoaded).toBe(true);
  });

  it('keeps why the session list could not be read, and retries on request', async () => {
    const s = new InstrumentsState('replay');
    const a = api({ tryGetSessions: vi.fn().mockResolvedValue(fail(500, 'boom')) });
    await s.loadReplaySessions(a);
    expect(s.replayError).toContain('boom');
    await s.retryReplaySessions(a);
    expect(a.tryGetSessions).toHaveBeenCalledTimes(2);
  });

  it('opens a session’s calls and summary', async () => {
    const s = new InstrumentsState('replay');
    const st = store();
    await s.selectReplaySession(st, 's1');
    expect(st.loadSessionReplay).toHaveBeenCalledWith('s1', { classify: true });
    expect(s.selectedSessionId).toBe('s1');
    expect(s.sessionCalls).toHaveLength(1);
    expect(s.sessionSummary.total).toBe(1);
    expect(s.replayLoading).toBe(false);
  });

  it('keeps why a session’s replay could not be read', async () => {
    const s = new InstrumentsState('replay');
    await s.selectReplaySession(store({ loadSessionReplay: vi.fn().mockResolvedValue({ replay: null, error: 'gone' }) }), 's1');
    expect(s.replayDetailError).toBe('gone');
    expect(s.sessionCalls).toEqual([]);
  });
});

describe('InstrumentsState — health', () => {
  it('reads the 14-day first-try rate once', async () => {
    const s = new InstrumentsState('insights');
    const a = api();
    await s.loadHolisticFtr(a);
    await s.loadHolisticFtr(a);
    expect(a.tryGetObservatoryFtr).toHaveBeenCalledTimes(1);
    expect(s.holisticFtr?.data).toEqual({ rate: 0.5 });
  });
});
