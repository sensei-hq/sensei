import type { LayoutLoad } from './$types';
import { loadRelayInbox } from '$lib/relay/relay-inbox';

// The inbox rail's data. Fans the single-tenant relay API out over the user's
// memberships (from the (app) layout via parent()) → a user-wide RelaySession[].
// The client-side state singleton is populated in +layout.svelte's onMount from this
// `sessions` (a rune singleton must not be written during SSR). Membership-less or a
// service failure → honest-empty; the rail always renders. See inbox.md.
export const load: LayoutLoad = async ({ parent, fetch }) => {
	const { memberships, accessToken, hasMembership } = await parent();
	if (!hasMembership || !memberships.length) return { sessions: [] };
	try {
		const sessions = await loadRelayInbox({ memberships, accessToken, fetch });
		return { sessions };
	} catch {
		return { sessions: [] };
	}
};
