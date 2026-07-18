import { describe, it, expect, vi, afterEach } from 'vitest';
import { isOnline, shouldFlushOnTransition, watchConnectivity } from './relay-connectivity';

// Relay P4.5 connectivity — the reconnect signal (browser online/offline events,
// no realtime yet). The decision "did we just reconnect, so flush?" is a pure
// function tested directly; the thin event-wiring is tested against jsdom's window.

describe('shouldFlushOnTransition', () => {
	it('flushes only on the offline → online edge', () => {
		expect(shouldFlushOnTransition(false, true)).toBe(true);
	});
	it('does not flush while steadily online', () => {
		expect(shouldFlushOnTransition(true, true)).toBe(false);
	});
	it('does not flush on going offline', () => {
		expect(shouldFlushOnTransition(true, false)).toBe(false);
		expect(shouldFlushOnTransition(false, false)).toBe(false);
	});
});

describe('isOnline', () => {
	afterEach(() => vi.restoreAllMocks());

	it('reads navigator.onLine when present', () => {
		vi.spyOn(navigator, 'onLine', 'get').mockReturnValue(false);
		expect(isOnline()).toBe(false);
		vi.spyOn(navigator, 'onLine', 'get').mockReturnValue(true);
		expect(isOnline()).toBe(true);
	});
});

describe('watchConnectivity', () => {
	afterEach(() => vi.restoreAllMocks());

	it('fires the handler with reconnected=true when going offline → online', () => {
		vi.spyOn(navigator, 'onLine', 'get').mockReturnValue(true);
		const seen: { online: boolean; reconnected: boolean }[] = [];
		const teardown = watchConnectivity((online, reconnected) => seen.push({ online, reconnected }));

		// Go offline, then back online.
		vi.spyOn(navigator, 'onLine', 'get').mockReturnValue(false);
		window.dispatchEvent(new Event('offline'));
		vi.spyOn(navigator, 'onLine', 'get').mockReturnValue(true);
		window.dispatchEvent(new Event('online'));

		expect(seen).toEqual([
			{ online: false, reconnected: false },
			{ online: true, reconnected: true }
		]);
		teardown();
	});

	it('teardown removes the listeners (no further handler calls)', () => {
		vi.spyOn(navigator, 'onLine', 'get').mockReturnValue(true);
		const handler = vi.fn();
		const teardown = watchConnectivity(handler);
		teardown();
		vi.spyOn(navigator, 'onLine', 'get').mockReturnValue(false);
		window.dispatchEvent(new Event('offline'));
		expect(handler).not.toHaveBeenCalled();
	});
});
