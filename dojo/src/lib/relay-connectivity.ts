// Relay P4.5 — connectivity state (SSR-safe, testable).
//
// Realtime is NOT built yet (P4.2 later), so the reconnect signal here is the
// browser's own `online`/`offline` events + `navigator.onLine`. This module keeps
// the event-wiring thin and puts the *decision* logic — "did we just come back
// online, and should we flush?" — behind a pure function so it unit-tests without
// a DOM and without faking events.
//
// SSR-safe: every `window`/`navigator` access is guarded; on the server (no
// `window`) we default to ONLINE so a server render never shows a false offline
// banner. The .svelte page owns the Svelte $state; this module owns the
// pure transition + the thin subscribe wiring.

/** Read the current connectivity, SSR-safe. Defaults to online when there's no
 *  `navigator` (server render) — we don't render a false "offline" on the server. */
export function isOnline(): boolean {
	if (typeof navigator === 'undefined') return true;
	// `onLine` is a heuristic (it can be true behind a captive portal) but it's the
	// right cheap signal for "the network went away"; a failed send is the backstop.
	return navigator.onLine !== false;
}

/**
 * Pure transition decision: given the previous and next connectivity, should the
 * app run its reconnect handler (flush the queue)? True ONLY on a genuine
 * offline→online edge — not on online→online (steady) or any →offline. Keeping
 * this pure means the "flush on reconnect" rule is tested directly, without
 * dispatching browser events.
 */
export function shouldFlushOnTransition(prev: boolean, next: boolean): boolean {
	return prev === false && next === true;
}

/** Called by {@link watchConnectivity} on every change, with the new state and
 *  whether this change is a reconnect edge (so the caller can flush). */
export type ConnectivityHandler = (online: boolean, reconnected: boolean) => void;

/**
 * Wire the browser `online`/`offline` events to `handler`, tracking the previous
 * state so it can flag the reconnect edge via {@link shouldFlushOnTransition}.
 * Returns a teardown that removes the listeners (call on unmount — no leaks).
 * SSR-safe: a no-op teardown when there's no `window`.
 */
export function watchConnectivity(handler: ConnectivityHandler): () => void {
	if (typeof window === 'undefined') return () => {};
	let prev = isOnline();
	const onChange = () => {
		const next = isOnline();
		const reconnected = shouldFlushOnTransition(prev, next);
		prev = next;
		handler(next, reconnected);
	};
	window.addEventListener('online', onChange);
	window.addEventListener('offline', onChange);
	return () => {
		window.removeEventListener('online', onChange);
		window.removeEventListener('offline', onChange);
	};
}
