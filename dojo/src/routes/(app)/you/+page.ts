import type { PageLoad } from './$types';
import { guardTenantScope } from '$lib/org-guard';
import { listRuns, listGates, DojoApiError, type RelayRun, type RelayGate } from '$lib/relay-data';
import { toKitRuns } from '$lib/relay-map';
import { needsYou, projects } from '$lib/components/kit/fixtures';

// The Your-work landing data. The live-runs strip now loads REAL `/v1` runs (+ the
// pending gates that mark a run as "needs you") via the SAME console client the
// shipped `(console)` relay list uses (relay-data.listRuns/listGates), mapped to
// the kit shape. The needs-you band and active-projects list stay fixture-backed
// (no single cross-dōjō needs route yet; projects pending its route) and remain
// gated behind `hasMembership` so a solo viewer sees an honest-empty landing (DJ1).
// The relay fetch runs behind `guardTenantScope` — a membership-less viewer skips
// the network; a live-service failure/404 degrades to empty so the page renders.
type RelayLists = { runs: RelayRun[]; gates: RelayGate[] };
const EMPTY_RELAY: RelayLists = { runs: [], gates: [] };

export const load: PageLoad = async ({ parent, fetch }) => {
	const { hasMembership, tenantKey, accessToken } = await parent();
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
		needsYou: hasMembership ? needsYou : [],
		runs: toKitRuns(relay.runs, relay.gates),
		projects: hasMembership ? projects : []
	};
};
