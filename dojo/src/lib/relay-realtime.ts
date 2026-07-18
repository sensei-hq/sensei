// Relay P4.2 — realtime swap. A browser-only Supabase Realtime subscription that
// turns the relay surface LIVE: instead of only refreshing on a user action
// (invalidateAll), the run-list + run-detail pages open ONE Realtime channel on
// the signed-in user's relay rows and refresh when a row changes (a raised gate,
// a segment update, a progress tick).
//
// How RLS scopes it to the user (the crux):
//   The channel authorizes as the *user*, not as the anon role. We build the
//   browser client with an `accessToken: () => <the session JWT>` callback
//   (supabase-js RealtimeClientOptions). supabase Realtime enforces RLS on
//   `postgres_changes`, so an authed client only receives changes for rows it can
//   SELECT — and P4.1's `user_id = auth.uid()` SELECT policies mean that's
//   exactly the user's own runs. A bare anon client would be rejected by RLS and
//   receive nothing. The anon/publishable key is only the API key for the
//   connection handshake; the JWT is what RLS reads.
//
// Structure (mirrors relay-connectivity.ts / relay-push.ts): the PURE, browser-
// free bits — the channel/topic spec and the coalesce (debounce) helper — are
// exported standalone so they unit-test without a socket; the impure orchestrator
// (subscribeRelay) guards for SSR / missing config and returns a no-op teardown
// rather than throwing. It is never imported at SSR/module-eval time in a way that
// touches `window`; the guard makes an accidental SSR import a safe no-op.

import { createClient } from '@supabase/supabase-js';

// ── Pure: channel spec (browser-free, unit-tested) ───────────────────────────

/** One `postgres_changes` listener config: which schema/table/event to watch. */
export interface RelayListenerSpec {
	schema: string;
	table: string;
	/** INSERT surfaces a raised gate / new run; UPDATE surfaces a segment or
	 *  progress change. DELETE isn't relevant to the relay surface. */
	event: 'INSERT' | 'UPDATE';
}

/** The whole subscription spec: a stable topic name + the listeners to attach. */
export interface RelayChannelSpec {
	topic: string;
	listeners: RelayListenerSpec[];
}

/** The dojo schema + the two relay tables the surface reads. Kept as a const so
 *  the spec builder and any test share one source of truth (no magic strings). */
const RELAY_SCHEMA = 'dojo';
const RELAY_TABLES = ['relay_sessions', 'relay_inbox'] as const;

/**
 * Build the Realtime channel spec: one topic and an INSERT + UPDATE listener per
 * relay table. Pure and deterministic so the "correct schema/tables/events" wiring
 * is asserted directly in a unit test, independent of the supabase socket. The
 * topic is a fixed per-surface name (one channel per page mount).
 */
export function relayChannelSpec(topic = 'relay:changes'): RelayChannelSpec {
	const listeners: RelayListenerSpec[] = [];
	for (const table of RELAY_TABLES) {
		listeners.push({ schema: RELAY_SCHEMA, table, event: 'INSERT' });
		listeners.push({ schema: RELAY_SCHEMA, table, event: 'UPDATE' });
	}
	return { topic, listeners };
}

// ── Pure: coalesce / debounce (browser-free, unit-tested) ────────────────────

/** A debounced function plus a `cancel()` to drop any pending trailing call —
 *  used by the teardown so an unmount can't fire a stray refresh. */
export interface Coalesced<A extends unknown[]> {
	(...args: A): void;
	cancel(): void;
}

/**
 * Coalesce rapid calls into at most one trailing call per `waitMs`. A burst of N
 * events (a batch of INSERTs/UPDATEs landing together) collapses to a single
 * `onChange` → a single `invalidateAll`, avoiding a refresh storm. Trailing-edge:
 * the call fires `waitMs` after the LAST invocation, with that call's args. Pure
 * over an injectable clock (`setTimer`/`clearTimer`) so it's tested with fake
 * timers and needs no DOM.
 */
export function coalesce<A extends unknown[]>(
	fn: (...args: A) => void,
	waitMs = 300,
	setTimer: (cb: () => void, ms: number) => ReturnType<typeof setTimeout> = setTimeout,
	clearTimer: (h: ReturnType<typeof setTimeout>) => void = clearTimeout
): Coalesced<A> {
	let handle: ReturnType<typeof setTimeout> | null = null;
	const coalesced = ((...args: A) => {
		if (handle !== null) clearTimer(handle);
		handle = setTimer(() => {
			handle = null;
			fn(...args);
		}, waitMs);
	}) as Coalesced<A>;
	coalesced.cancel = () => {
		if (handle !== null) {
			clearTimer(handle);
			handle = null;
		}
	};
	return coalesced;
}

// ── Impure: the thin Realtime channel ────────────────────────────────────────

/** What a change event carries back to the page — enough to decide/log, but the
 *  page's `onChange` just triggers a (debounced) refresh. */
export interface RelayChangeInfo {
	table: string;
	event: RelayListenerSpec['event'];
}

/** Options for {@link subscribeRelay}. `accessToken` is the signed-in user's
 *  session JWT — the value that makes Realtime authorize as the user so RLS scopes
 *  the stream to their own rows. */
export interface SubscribeRelayOpts {
	url: string | undefined;
	anonKey: string | undefined;
	accessToken: string | null | undefined;
	onChange: (info: RelayChangeInfo) => void;
	/** Debounce window for coalescing rapid changes into one onChange. */
	debounceMs?: number;
	/** Fixed channel topic (one channel per mount). */
	topic?: string;
}

/** A teardown that removes the channel + cancels any pending debounced refresh. */
export type RelayUnsubscribe = () => void;

const NOOP_UNSUBSCRIBE: RelayUnsubscribe = () => {};

/**
 * Open ONE Realtime channel on the user's relay rows and call `onChange`
 * (coalesced) on any INSERT/UPDATE to `dojo.relay_sessions` / `dojo.relay_inbox`.
 * Returns a teardown that removes the channel and cancels the debounce — call it
 * on unmount so navigating between runs never accumulates channels.
 *
 * SSR / missing-config safe: if there's no `window` (server render) or the url /
 * anonKey / accessToken is missing (unauthenticated), it subscribes to nothing and
 * returns a no-op teardown — it never throws at import or SSR, and the page's
 * action-refresh path keeps working.
 */
export function subscribeRelay(opts: SubscribeRelayOpts): RelayUnsubscribe {
	const { url, anonKey, accessToken, onChange, debounceMs = 300, topic } = opts;

	// Guard: server render, or no config / no session → do nothing, safely.
	if (typeof window === 'undefined' || !url || !anonKey || !accessToken) {
		return NOOP_UNSUBSCRIBE;
	}

	// Coalesce the refresh so a burst of changes triggers a single onChange.
	const fire = coalesce(onChange, debounceMs);

	// A browser client whose Realtime connection authorizes as the USER: the
	// `accessToken` callback hands supabase the session JWT, so RLS on
	// postgres_changes scopes the stream to the user's own relay rows. `db.schema`
	// keeps it consistent with the server client; auth session persistence is off
	// (this client is realtime-only — the JWT comes from the callback, not storage).
	const client = createClient(url, anonKey, {
		auth: { persistSession: false, autoRefreshToken: false },
		db: { schema: RELAY_SCHEMA },
		accessToken: async () => accessToken
	});

	const spec = relayChannelSpec(topic);
	let channel = client.channel(spec.topic);
	for (const l of spec.listeners) {
		channel = channel.on(
			'postgres_changes',
			{ event: l.event, schema: l.schema, table: l.table },
			() => fire({ table: l.table, event: l.event })
		);
	}
	channel.subscribe();

	return () => {
		fire.cancel();
		// removeChannel is async (unsubscribe round-trip) but the teardown is sync;
		// we don't await — the channel is dropped from the client immediately and the
		// socket closes in the background. Errors here are non-fatal.
		void client.removeChannel(channel);
	};
}
