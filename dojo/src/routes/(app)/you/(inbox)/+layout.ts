import type { LayoutLoad } from './$types';
import { loadRelayInbox, type InboxLoad } from '$lib/relay/relay-inbox';

/** The inbox load, settled: the fan-out's result, or why it could not run. */
export type InboxOutcome = { ok: true; result: InboxLoad } | { ok: false; message: string };

// The inbox rail's data. Fans the single-tenant relay API out over the user's
// memberships (from the (app) layout via parent()) → a user-wide RelaySession[].
//
// STREAMED, not awaited: `inbox` is a promise, so the rail renders its skeleton at
// once and fills when the relay answers. It always RESOLVES — to the result, or to the
// failure — so a relay outage reaches the screen as an error state, never as an
// empty inbox (#104). The client-side state singleton is populated in +layout.svelte
// (a rune singleton must not be written during SSR). See inbox.md.
export const load: LayoutLoad = async ({ parent, fetch }) => {
	const { memberships, accessToken, hasMembership } = await parent();
	const list = hasMembership ? memberships : [];
	const inbox: Promise<InboxOutcome> = loadRelayInbox({ memberships: list, accessToken, fetch }).then(
		(result) => ({ ok: true as const, result }),
		(e: unknown) => ({ ok: false as const, message: e instanceof Error ? e.message : String(e) })
	);
	// The dōjōs the inbox spans — "Notify me" registers this browser with each.
	return { inbox, tenantKeys: list.map((m) => m.url) };
};
