// app/src/lib/health-transport.spec.svelte.ts

import { describe, it, expect, vi, beforeEach } from 'vitest';
import type { HealthEvent, HealthPayload } from './health-types.js';
import { MockTransport, RealTransport } from './health-transport.js';
import { okPayload, needsActionPayload, remedyFixture } from './health-state.spec.svelte.js';

// ── Hoisted Tauri mocks (must precede any import of health-transport) ─────────

const invokeMock = vi.fn();
const listenMock = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({ invoke: invokeMock }));
vi.mock('@tauri-apps/api/event', () => ({ listen: listenMock }));

// ── check() ──────────────────────────────────────────────────────────────────

describe('MockTransport — check()', () => {
  it('returns the configured checkPayload', async () => {
    const payload = okPayload();
    const t = new MockTransport({ checkPayload: payload });
    const result = await t.check();
    expect(result).toBe(payload);
  });

  it('records one call per invocation in checkCalls', async () => {
    const t = new MockTransport({ checkPayload: okPayload() });
    expect(t.checkCalls).toHaveLength(0);
    await t.check();
    expect(t.checkCalls).toHaveLength(1);
  });

  it('accumulates multiple check() calls in checkCalls', async () => {
    const t = new MockTransport({ checkPayload: okPayload() });
    await t.check();
    await t.check();
    await t.check();
    expect(t.checkCalls).toHaveLength(3);
  });
});

// ── resolve() — default behaviour (no opts) ──────────────────────────────────

describe('MockTransport — resolve() defaults', () => {
  it('default resolveEvents is empty — onEvent is not called', async () => {
    const t = new MockTransport({ checkPayload: okPayload() });
    const events: HealthEvent[] = [];
    await t.resolve(okPayload(), (ev) => events.push(ev));
    expect(events).toHaveLength(0);
  });

  it('resolves with checkPayload when resolveTerminal is absent', async () => {
    const payload = okPayload();
    const t = new MockTransport({ checkPayload: payload });
    const result = await t.resolve(okPayload(), () => {});
    expect(result).toBe(payload);
  });
});

// ── resolve() — scripted events ───────────────────────────────────────────────

describe('MockTransport — resolve() scripted events', () => {
  it('fires onEvent once per scripted event, in order', async () => {
    const events: HealthEvent[] = [
      { kind: 'phase', phase: 'resolving' },
      { kind: 'report', payload: okPayload() },
    ];
    const t = new MockTransport({ checkPayload: okPayload(), resolveEvents: events });
    const received: HealthEvent[] = [];
    await t.resolve(okPayload(), (ev) => received.push(ev));
    expect(received).toHaveLength(2);
    expect(received[0]).toBe(events[0]);
    expect(received[1]).toBe(events[1]);
  });

  it('resolves with resolveTerminal when provided', async () => {
    const terminal = needsActionPayload();
    const t = new MockTransport({
      checkPayload: okPayload(),
      resolveTerminal: terminal,
    });
    const result = await t.resolve(okPayload(), () => {});
    expect(result).toBe(terminal);
  });

  it('delivers phase events correctly', async () => {
    const phaseEvent: HealthEvent = { kind: 'phase', phase: 'resolving' };
    const t = new MockTransport({
      checkPayload: okPayload(),
      resolveEvents: [phaseEvent],
    });
    const received: HealthEvent[] = [];
    await t.resolve(okPayload(), (ev) => received.push(ev));
    expect(received[0]).toEqual({ kind: 'phase', phase: 'resolving' });
  });

  it('delivers component events correctly', async () => {
    const componentEvent: HealthEvent = {
      kind: 'component',
      id: 'postgres',
      patch: { status: 'installing' },
    };
    const t = new MockTransport({
      checkPayload: okPayload(),
      resolveEvents: [componentEvent],
    });
    const received: HealthEvent[] = [];
    await t.resolve(okPayload(), (ev) => received.push(ev));
    expect(received[0]).toEqual(componentEvent);
  });

  it('delivers remedy events correctly', async () => {
    const remedyEvent: HealthEvent = {
      kind: 'remedy',
      remedy: remedyFixture(),
    };
    const t = new MockTransport({
      checkPayload: okPayload(),
      resolveEvents: [remedyEvent],
    });
    const received: HealthEvent[] = [];
    await t.resolve(okPayload(), (ev) => received.push(ev));
    expect(received[0]).toEqual(remedyEvent);
  });

  it('delivers report events correctly', async () => {
    const reportEvent: HealthEvent = {
      kind: 'report',
      payload: needsActionPayload(),
    };
    const t = new MockTransport({
      checkPayload: okPayload(),
      resolveEvents: [reportEvent],
    });
    const received: HealthEvent[] = [];
    await t.resolve(okPayload(), (ev) => received.push(ev));
    expect(received[0]).toEqual(reportEvent);
  });

  it('delivers all four event kinds in a single sequence', async () => {
    const resolveEvents: HealthEvent[] = [
      { kind: 'phase', phase: 'resolving' },
      { kind: 'component', id: 'postgres', patch: { status: 'checking' } },
      { kind: 'remedy', remedy: remedyFixture() },
      { kind: 'report', payload: needsActionPayload() },
    ];
    const t = new MockTransport({ checkPayload: okPayload(), resolveEvents });
    const received: HealthEvent[] = [];
    await t.resolve(okPayload(), (ev) => received.push(ev));
    expect(received).toHaveLength(4);
    expect(received.map((e) => e.kind)).toEqual(['phase', 'component', 'remedy', 'report']);
  });
});

// ── resolve() — call recording ────────────────────────────────────────────────

describe('MockTransport — resolve() call recording', () => {
  it('records each call with its current arg in resolveCalls', async () => {
    const current = okPayload();
    const t = new MockTransport({ checkPayload: okPayload() });
    expect(t.resolveCalls).toHaveLength(0);
    await t.resolve(current, () => {});
    expect(t.resolveCalls).toHaveLength(1);
    expect(t.resolveCalls[0].current).toBe(current);
  });

  it('accumulates multiple resolve() calls in resolveCalls', async () => {
    const p1 = okPayload();
    const p2 = needsActionPayload();
    const t = new MockTransport({ checkPayload: okPayload() });
    await t.resolve(p1, () => {});
    await t.resolve(p2, () => {});
    expect(t.resolveCalls).toHaveLength(2);
    expect(t.resolveCalls[0].current).toBe(p1);
    expect(t.resolveCalls[1].current).toBe(p2);
  });
});

// ── RealTransport ─────────────────────────────────────────────────────────────

/**
 * Helper: wires listenMock so it captures the handler and returns an unlisten
 * stub.  Returns `fire(ev)` to push an event and `unlisten` to inspect calls.
 */
function rigListen() {
  let handler: ((e: { payload: HealthEvent }) => void) | null = null;
  const unlisten = vi.fn();
  listenMock.mockImplementation(async (_channel: string, fn: (e: { payload: HealthEvent }) => void) => {
    handler = fn;
    return unlisten;
  });
  return {
    fire: (ev: HealthEvent) => handler?.({ payload: ev }),
    unlisten,
  };
}

describe('RealTransport', () => {
  beforeEach(() => {
    invokeMock.mockReset();
    listenMock.mockReset();
  });

  // ── check() ────────────────────────────────────────────────────────────────

  it('check() calls invoke("health_check") exactly once', async () => {
    const expected = okPayload();
    invokeMock.mockResolvedValueOnce(expected);

    const t = new RealTransport();
    await t.check();

    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(invokeMock).toHaveBeenCalledWith('health_check');
  });

  it('check() resolves with the typed HealthPayload returned by invoke', async () => {
    const expected = okPayload();
    invokeMock.mockResolvedValueOnce(expected);

    const t = new RealTransport();
    const result = await t.check();

    expect(result).toBe(expected);
  });

  it('check() propagates rejections from invoke', async () => {
    const err = new Error('tauri bridge down');
    invokeMock.mockRejectedValueOnce(err);

    const t = new RealTransport();
    await expect(t.check()).rejects.toThrow('tauri bridge down');
  });

  // ── resolve() ──────────────────────────────────────────────────────────────

  it('resolve() subscribes to "health" before invoking "health_check_and_resolve"', async () => {
    const callOrder: string[] = [];

    listenMock.mockImplementation(async (_channel: string, fn: (e: { payload: HealthEvent }) => void) => {
      callOrder.push('listen');
      // immediately fire the terminal report so the promise settles
      setTimeout(() => fn({ payload: { kind: 'report', payload: okPayload() } }), 0);
      return vi.fn();
    });

    invokeMock.mockImplementation(async (cmd: string) => {
      callOrder.push(cmd);
    });

    const t = new RealTransport();
    await t.resolve(okPayload(), () => {});

    expect(callOrder[0]).toBe('listen');
    expect(callOrder[1]).toBe('health_check_and_resolve');
  });

  it('resolve() invokes "health_check_and_resolve" exactly once', async () => {
    const { fire } = rigListen();
    invokeMock.mockImplementation(async () => {
      fire({ kind: 'report', payload: okPayload() });
    });

    const t = new RealTransport();
    await t.resolve(okPayload(), () => {});

    const resolveCallCount = invokeMock.mock.calls.filter(
      (c) => c[0] === 'health_check_and_resolve',
    ).length;
    expect(resolveCallCount).toBe(1);
  });

  it('resolve() forwards every received event to onEvent in order', async () => {
    const { fire } = rigListen();
    const terminal = needsActionPayload();

    invokeMock.mockImplementation(async () => {
      fire({ kind: 'phase', phase: 'resolving' });
      fire({ kind: 'component', id: 'postgres', patch: { status: 'checking' } });
      fire({ kind: 'report', payload: terminal });
    });

    const t = new RealTransport();
    const received: HealthEvent[] = [];
    await t.resolve(okPayload(), (ev) => received.push(ev));

    expect(received).toHaveLength(3);
    expect(received[0]).toEqual({ kind: 'phase', phase: 'resolving' });
    expect(received[1]).toEqual({ kind: 'component', id: 'postgres', patch: { status: 'checking' } });
    expect(received[2]).toEqual({ kind: 'report', payload: terminal });
  });

  it('resolve() resolves with the payload of the first kind:"report" event', async () => {
    const terminal = needsActionPayload();
    const { fire } = rigListen();

    invokeMock.mockImplementation(async () => {
      fire({ kind: 'report', payload: terminal });
    });

    const t = new RealTransport();
    const result = await t.resolve(okPayload(), () => {});

    expect(result).toBe(terminal);
  });

  it('resolve() ignores events that arrive after the terminal report (settled flag)', async () => {
    const terminal = okPayload();
    const { fire } = rigListen();

    invokeMock.mockImplementation(async () => {
      fire({ kind: 'report', payload: terminal });
      // These arrive after settlement — must be silently ignored.
      fire({ kind: 'phase', phase: 'resolving' });
      fire({ kind: 'report', payload: needsActionPayload() });
    });

    const t = new RealTransport();
    const received: HealthEvent[] = [];
    const result = await t.resolve(okPayload(), (ev) => received.push(ev));

    // Only the first report fires onEvent; subsequent events are dropped.
    const reportCount = received.filter((e) => e.kind === 'report').length;
    expect(reportCount).toBe(1);
    expect(result).toBe(terminal);
  });

  it('resolve() calls unlisten once the report arrives (success cleanup)', async () => {
    const { fire, unlisten } = rigListen();

    invokeMock.mockImplementation(async () => {
      fire({ kind: 'report', payload: okPayload() });
    });

    const t = new RealTransport();
    await t.resolve(okPayload(), () => {});

    expect(unlisten).toHaveBeenCalledTimes(1);
  });

  it('resolve() calls unlisten when invoke("health_check_and_resolve") rejects (error cleanup)', async () => {
    const { unlisten } = rigListen();
    const err = new Error('rust panic');

    invokeMock.mockRejectedValueOnce(err);

    const t = new RealTransport();
    await expect(t.resolve(okPayload(), () => {})).rejects.toThrow('rust panic');

    expect(unlisten).toHaveBeenCalledTimes(1);
  });

  it('resolve() rejects if invoke("health_check_and_resolve") rejects before any report', async () => {
    const { unlisten } = rigListen();
    const err = new Error('sidecar unavailable');

    invokeMock.mockRejectedValueOnce(err);

    const t = new RealTransport();
    const received: HealthEvent[] = [];
    await expect(t.resolve(okPayload(), (ev) => received.push(ev))).rejects.toThrow('sidecar unavailable');

    expect(received).toHaveLength(0);
    expect(unlisten).toHaveBeenCalledTimes(1);
  });
});
