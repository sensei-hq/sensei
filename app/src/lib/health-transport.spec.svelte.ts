// app/src/lib/health-transport.spec.svelte.ts

import { describe, it, expect } from 'vitest';
import type { HealthEvent } from './health-types.js';
import { MockTransport } from './health-transport.js';
import { okPayload, needsActionPayload, remedyFixture } from './health-state.spec.svelte.js';

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
