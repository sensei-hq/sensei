/**
 * Tests for the generic EventManager.
 * Uses a mock EventSource to verify subscribe/dispatch/reconnect behavior.
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { EventManager } from './events.js';

// ── Mock EventSource ─────────────────────────────────────────────────────────

class MockEventSource {
  static instances: MockEventSource[] = [];
  readyState = 0; // CONNECTING
  onmessage: ((e: { data: string }) => void) | null = null;
  onerror: (() => void) | null = null;

  constructor(public url: string) {
    MockEventSource.instances.push(this);
    // Simulate async open
    setTimeout(() => { this.readyState = 1; }, 0); // OPEN
  }

  close() {
    this.readyState = 2; // CLOSED
  }

  // Test helper: simulate a server-sent event
  simulateMessage(data: string) {
    this.onmessage?.({ data });
  }

  // Test helper: simulate an error
  simulateError() {
    this.onerror?.();
  }
}

beforeEach(() => {
  MockEventSource.instances = [];
  vi.stubGlobal('EventSource', MockEventSource);
});

// ── Tests ────────────────────────────────────────────────────────────────────

describe('EventManager', () => {
  it('connects when first subscriber added', () => {
    const mgr = new EventManager<string>('http://test/sse', (d) => d);
    expect(MockEventSource.instances).toHaveLength(0);

    mgr.subscribe(() => {});
    expect(MockEventSource.instances).toHaveLength(1);
    expect(MockEventSource.instances[0].url).toBe('http://test/sse');
  });

  it('dispatches parsed events to handlers', () => {
    const mgr = new EventManager<{ type: string }>('http://test/sse', (d) => JSON.parse(d));
    const received: { type: string }[] = [];

    mgr.subscribe((e) => received.push(e));

    const source = MockEventSource.instances[0];
    source.simulateMessage('{"type":"file_indexed"}');
    source.simulateMessage('{"type":"repo_done"}');

    expect(received).toEqual([
      { type: 'file_indexed' },
      { type: 'repo_done' },
    ]);
  });

  it('supports multiple subscribers', () => {
    const mgr = new EventManager<string>('http://test/sse', (d) => d);
    const a: string[] = [];
    const b: string[] = [];

    mgr.subscribe((e) => a.push(e));
    mgr.subscribe((e) => b.push(e));

    // Should reuse same connection
    expect(MockEventSource.instances).toHaveLength(1);

    MockEventSource.instances[0].simulateMessage('hello');
    expect(a).toEqual(['hello']);
    expect(b).toEqual(['hello']);
  });

  it('unsubscribe stops receiving events', () => {
    const mgr = new EventManager<string>('http://test/sse', (d) => d);
    const received: string[] = [];

    const unsub = mgr.subscribe((e) => received.push(e));
    MockEventSource.instances[0].simulateMessage('first');
    unsub();
    MockEventSource.instances[0].simulateMessage('second');

    expect(received).toEqual(['first']);
  });

  it('disconnects when last subscriber removed', () => {
    const mgr = new EventManager<string>('http://test/sse', (d) => d);
    const unsub = mgr.subscribe(() => {});

    const source = MockEventSource.instances[0];
    expect(source.readyState).not.toBe(2); // not closed

    unsub();
    expect(source.readyState).toBe(2); // closed
  });

  it('destroy removes all handlers and closes', () => {
    const mgr = new EventManager<string>('http://test/sse', (d) => d);
    mgr.subscribe(() => {});
    mgr.subscribe(() => {});

    expect(mgr.subscriberCount).toBe(2);
    mgr.destroy();
    expect(mgr.subscriberCount).toBe(0);
    expect(MockEventSource.instances[0].readyState).toBe(2);
  });

  it('ignores parse errors without crashing', () => {
    const mgr = new EventManager<object>('http://test/sse', (d) => JSON.parse(d));
    const received: object[] = [];

    mgr.subscribe((e) => received.push(e));
    MockEventSource.instances[0].simulateMessage('not valid json');
    MockEventSource.instances[0].simulateMessage('{"ok":true}');

    // Should skip the bad event and deliver the good one
    expect(received).toEqual([{ ok: true }]);
  });

  it('reconnects on error when handlers exist', async () => {
    vi.useFakeTimers();

    const mgr = new EventManager<string>('http://test/sse', (d) => d, 100);
    mgr.subscribe(() => {});

    expect(MockEventSource.instances).toHaveLength(1);

    // Simulate error
    MockEventSource.instances[0].simulateError();
    expect(MockEventSource.instances[0].readyState).toBe(2); // old closed

    // Advance past reconnect delay
    vi.advanceTimersByTime(150);
    expect(MockEventSource.instances).toHaveLength(2); // new connection

    vi.useRealTimers();
    mgr.destroy();
  });

  it('does not reconnect after destroy', async () => {
    vi.useFakeTimers();

    const mgr = new EventManager<string>('http://test/sse', (d) => d, 100);
    mgr.subscribe(() => {});
    mgr.destroy();

    MockEventSource.instances[0].simulateError();
    vi.advanceTimersByTime(200);

    // Should NOT have created a new connection
    expect(MockEventSource.instances).toHaveLength(1);

    vi.useRealTimers();
  });
});
