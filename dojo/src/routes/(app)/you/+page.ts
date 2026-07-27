import type { PageLoad } from './$types';
import { guardTenantScope } from '$lib/org-guard';
import { listRuns, listGates, DojoApiError, type RelayRun, type RelayGate } from '$lib/relay-data';
import { toKitInbox } from '$lib/relay-map';

// The personal landing is the Inbox — one list of every in-flight session. It
// loads REAL /v1 runs + their pending gates (the SAME console client the shipped
// (console) relay list uses) and maps them to sorted inbox rows. The fetch runs
// behind guardTenantScope: a membership-less viewer skips the network and gets an
// honest-empty inbox (DJ1); a live-service failure/404 degrades to empty + a
// surfaced error so the page always renders.
type RelayLists = { runs: RelayRun[]; gates: RelayGate[] };
const EMPTY_RELAY: RelayLists = { runs: [], gates: [] };

export const load: PageLoad = async ({ parent, fetch }) => {
	const { tenantKey, accessToken } = await parent();
	let relay: RelayLists = EMPTY_RELAY;
	let error: string | null = null;
	try {
		const guarded = await guardTenantScope<RelayLists>(tenantKey, EMPTY_RELAY, async (tk) => {
			const [runs, gates] = await Promise.all([
				listRuns(tk, { fetch, accessToken }),
				listGates(tk, { fetch, accessToken })
			]);
			return { runs, gates };
		});
		relay = guarded.value;
	} catch (e) {
		error = e instanceof DojoApiError ? e.message : 'could not reach the dojo service';
	}
	return {
		error,
		inbox: toKitInbox(relay.runs, relay.gates)
	};
};
