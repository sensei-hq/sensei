import { describe, it, expect, vi, afterEach, beforeEach } from 'vitest';

// Relay P4.2 realtime — the pure channel spec + coalesce are tested directly; the
// impure subscribeRelay is tested against a MOCKED supabase-js so no real socket is
// opened. The mock is hoisted so the module-under-test picks it up on import.

// A recording fake supabase client: channel().on()...subscribe() chains, and
// removeChannel records the teardown. createClient captures the constructor args
// (so we can assert the accessToken callback + schema) and returns the fake.
const mock = vi.hoisted(() => {
	const state: {
		createClientArgs: unknown[];
		onCalls: { type: string; filter: Record<string, unknown>; cb: (p: unknown) => void }[];
		subscribeCount: number;
		removed: unknown[];
	} = { createClientArgs: [], onCalls: [], subscribeCount: 0, removed: [] };

	function makeChannel() {
		const channel = {
			on(type: string, filter: Record<string, unknown>, cb: (p: unknown) => void) {
				state.onCalls.push({ type, filter, cb });
				return channel; // chainable
			},
			subscribe() {
				state.subscribeCount++;
				return channel;
			}
		};
		return channel;
	}

	const client = {
		channel: vi.fn(() => makeChannel()),
		removeChannel: vi.fn(async (ch: unknown) => {
			state.removed.push(ch);
			return { status: 'ok' } as const;
		})
	};

	const createClient = vi.fn((...args: unknown[]) => {
		state.createClientArgs = args;
		return client;
	});

	return { state, client, createClient };
});

vi.mock('@supabase/supabase-js', () => ({ createClient: mock.createClient }));

import {
	relayChannelSpec,
	coalesce,
	subscribeRelay,
	type RelayChangeInfo
} from './relay-realtime';

afterEach(() => {
	vi.restoreAllMocks();
	mock.state.createClientArgs = [];
	mock.state.onCalls = [];
	mock.state.subscribeCount = 0;
	mock.state.removed = [];
	mock.client.channel.mockClear();
	mock.client.removeChannel.mockClear();
	mock.createClient.mockClear();
});

// ── Pure: channel spec ───────────────────────────────────────────────────────

describe('relayChannelSpec', () => {
	it('watches dojo.relay_sessions + dojo.relay_inbox for INSERT and UPDATE', () => {
		const spec = relayChannelSpec();
		// One listener per (table, event): 2 tables × {INSERT, UPDATE} = 4.
		expect(spec.listeners).toHaveLength(4);
		const asKeys = spec.listeners.map((l) => `${l.schema}.${l.table}:${l.event}`).sort();
		expect(asKeys).toEqual([
			'dojo.relay_inbox:INSERT',
			'dojo.relay_inbox:UPDATE',
			'dojo.relay_sessions:INSERT',
			'dojo.relay_sessions:UPDATE'
		]);
		// Every listener is the dojo schema — never the default `public`.
		expect(spec.listeners.every((l) => l.schema === 'dojo')).toBe(true);
	});

	it('uses a stable default topic and honours an override', () => {
		expect(relayChannelSpec().topic).toBe('relay:changes');
		expect(relayChannelSpec('relay:run-42').topic).toBe('relay:run-42');
	});
});

// ── Pure: coalesce / debounce ────────────────────────────────────────────────

describe('coalesce', () => {
	beforeEach(() => vi.useFakeTimers());
	afterEach(() => vi.useRealTimers());

	it('collapses N rapid calls into a single trailing call within the window', () => {
		const fn = vi.fn();
		const c = coalesce(fn, 300);
		c('a');
		c('b');
		c('c'); // three rapid calls
		expect(fn).not.toHaveBeenCalled(); // nothing yet — still debouncing
		vi.advanceTimersByTime(300);
		expect(fn).toHaveBeenCalledTimes(1); // exactly one refresh
		expect(fn).toHaveBeenCalledWith('c'); // with the LAST args (trailing edge)
	});

	it('fires again for a new burst after the window has elapsed', () => {
		const fn = vi.fn();
		const c = coalesce(fn, 250);
		c(1);
		vi.advanceTimersByTime(250);
		c(2);
		vi.advanceTimersByTime(250);
		expect(fn).toHaveBeenCalledTimes(2);
		expect(fn).toHaveBeenNthCalledWith(1, 1);
		expect(fn).toHaveBeenNthCalledWith(2, 2);
	});

	it('cancel() drops a pending trailing call (no stray fire after teardown)', () => {
		const fn = vi.fn();
		const c = coalesce(fn, 300);
		c('x');
		c.cancel();
		vi.advanceTimersByTime(1000);
		expect(fn).not.toHaveBeenCalled();
	});
});

// ── Impure: subscribeRelay guards (SSR / missing config) ─────────────────────

describe('subscribeRelay guards', () => {
	const base = {
		url: 'http://127.0.0.1:54321',
		anonKey: 'anon',
		accessToken: 'JWT',
		onChange: () => {}
	};

	it('is a no-op (no client, no-op teardown) when the access token is missing', () => {
		const teardown = subscribeRelay({ ...base, accessToken: null });
		expect(mock.createClient).not.toHaveBeenCalled();
		expect(() => teardown()).not.toThrow(); // no-op teardown, never throws
	});

	it('is a no-op when the url or anon key is missing', () => {
		expect(subscribeRelay({ ...base, url: undefined })).toBeTypeOf('function');
		expect(subscribeRelay({ ...base, anonKey: undefined })).toBeTypeOf('function');
		expect(mock.createClient).not.toHaveBeenCalled();
	});

	it('is a no-op under SSR (no window) and never throws', () => {
		const original = globalThis.window;
		// Simulate SSR: remove window for the duration of the call.
		// @ts-expect-error — deliberately deleting window to exercise the SSR guard.
		delete globalThis.window;
		try {
			const teardown = subscribeRelay(base);
			expect(mock.createClient).not.toHaveBeenCalled();
			expect(() => teardown()).not.toThrow();
		} finally {
			globalThis.window = original;
		}
	});
});

// ── Impure: subscribeRelay wiring + teardown ─────────────────────────────────

describe('subscribeRelay wiring', () => {
	beforeEach(() => vi.useFakeTimers());
	afterEach(() => vi.useRealTimers());

	const base = {
		url: 'http://127.0.0.1:54321',
		anonKey: 'anon-key',
		accessToken: 'the-user-jwt'
	};

	it('opens one channel, attaches the 4 postgres_changes listeners, subscribes once', () => {
		subscribeRelay({ ...base, onChange: () => {} });
		expect(mock.createClient).toHaveBeenCalledTimes(1);
		expect(mock.client.channel).toHaveBeenCalledTimes(1); // ONE channel
		expect(mock.state.subscribeCount).toBe(1); // subscribed once
		expect(mock.state.onCalls).toHaveLength(4);
		expect(mock.state.onCalls.every((c) => c.type === 'postgres_changes')).toBe(true);
	});

	it('builds the browser client with a JWT accessToken callback + the dojo schema', async () => {
		subscribeRelay({ ...base, onChange: () => {} });
		const [url, key, options] = mock.state.createClientArgs as [
			string,
			string,
			{ db: { schema: string }; accessToken: () => Promise<string> }
		];
		expect(url).toBe(base.url);
		expect(key).toBe(base.anonKey);
		expect(options.db.schema).toBe('dojo');
		// The accessToken callback resolves to the user's session JWT — this is what
		// makes Realtime authorize as the user so RLS scopes it to their own rows.
		await expect(options.accessToken()).resolves.toBe('the-user-jwt');
	});

	it('coalesces a burst of change events into a single onChange', () => {
		const onChange = vi.fn<(info: RelayChangeInfo) => void>();
		subscribeRelay({ ...base, onChange, debounceMs: 250 });
		// Fire every listener's callback back-to-back (a batch landing together).
		for (const c of mock.state.onCalls) c.cb({});
		expect(onChange).not.toHaveBeenCalled(); // debounced
		vi.advanceTimersByTime(250);
		expect(onChange).toHaveBeenCalledTimes(1); // exactly one refresh for the burst
	});

	it('teardown removes the channel and cancels a pending debounced refresh', () => {
		const onChange = vi.fn();
		const teardown = subscribeRelay({ ...base, onChange, debounceMs: 300 });
		mock.state.onCalls[0].cb({}); // queue a refresh
		teardown();
		expect(mock.client.removeChannel).toHaveBeenCalledTimes(1);
		vi.advanceTimersByTime(1000);
		expect(onChange).not.toHaveBeenCalled(); // the pending refresh was cancelled
	});

	it('does not leak channels across repeated subscribe/teardown cycles', () => {
		const t1 = subscribeRelay({ ...base, onChange: () => {} });
		t1();
		const t2 = subscribeRelay({ ...base, onChange: () => {} });
		t2();
		expect(mock.client.channel).toHaveBeenCalledTimes(2);
		expect(mock.client.removeChannel).toHaveBeenCalledTimes(2); // each opened one removed
	});
});
